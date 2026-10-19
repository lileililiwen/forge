using System.Net;
using System.Net.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Web.Resilience;
using Platform.Web.Resilience.Handlers;
using Platform.Web.Telemetry;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.Web.Edge.Tests;

public sealed class ResilienceHandlerTests
{
    [Fact]
    public async Task Transient_get_is_retried_until_attempts_exhausted()
    {
        var attempts = 0;
        var telemetry = new RecordingTelemetry();
        var inner = new StubHandler((request, _) =>
        {
            attempts++;
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.ServiceUnavailable));
        });
        var handler = BuildHandler(telemetry, inner, maxRetryAttempts: 3);
        using var invoker = new HttpMessageInvoker(handler);

        var response = await invoker.SendAsync(new HttpRequestMessage(HttpMethod.Get, "https://api.example.com/v1/items"), CancellationToken.None);

        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
        Assert.Equal(3, attempts);
        Assert.Equal(2, telemetry.Events.Count(e => e.Decision == HttpResilienceDecision.Retried));
    }

    [Fact]
    public async Task Post_is_not_retried_by_default()
    {
        var attempts = 0;
        var telemetry = new RecordingTelemetry();
        var inner = new StubHandler((request, _) =>
        {
            attempts++;
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.ServiceUnavailable));
        });
        var handler = BuildHandler(telemetry, inner, maxRetryAttempts: 3);
        using var invoker = new HttpMessageInvoker(handler);

        var response = await invoker.SendAsync(new HttpRequestMessage(HttpMethod.Post, "https://api.example.com/v1/items") { Content = new StringContent("{}") }, CancellationToken.None);

        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
        Assert.Equal(1, attempts);
        Assert.DoesNotContain(telemetry.Events, e => e.Decision == HttpResilienceDecision.Retried);
    }

    [Fact]
    public async Task Post_with_explicit_idempotent_method_is_retried()
    {
        var attempts = 0;
        var telemetry = new RecordingTelemetry();
        var inner = new StubHandler((request, _) =>
        {
            attempts++;
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.ServiceUnavailable));
        });
        var handler = BuildHandler(telemetry, inner, maxRetryAttempts: 3, extraIdempotent: new HashSet<string>(StringComparer.OrdinalIgnoreCase) { "POST" });
        using var invoker = new HttpMessageInvoker(handler);

        var response = await invoker.SendAsync(new HttpRequestMessage(HttpMethod.Post, "https://api.example.com/v1/items") { Content = new StringContent("{}") }, CancellationToken.None);

        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
        Assert.Equal(3, attempts);
    }

    [Fact]
    public async Task Successful_response_does_not_retry()
    {
        var attempts = 0;
        var telemetry = new RecordingTelemetry();
        var inner = new StubHandler((request, _) =>
        {
            attempts++;
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK));
        });
        var handler = BuildHandler(telemetry, inner, maxRetryAttempts: 5);
        using var invoker = new HttpMessageInvoker(handler);

        var response = await invoker.SendAsync(new HttpRequestMessage(HttpMethod.Get, "https://api.example.com/v1/items"), CancellationToken.None);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Equal(1, attempts);
        Assert.DoesNotContain(telemetry.Events, e => e.Decision == HttpResilienceDecision.Retried);
    }

    [Fact]
    public async Task Retry_attempt_header_is_emitted()
    {
        var headers = new List<int>();
        var inner = new StubHandler((request, _) =>
        {
            if (request.Headers.TryGetValues("X-Retry-Attempt", out var values))
                headers.Add(int.Parse(values.First(), System.Globalization.CultureInfo.InvariantCulture));
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.BadGateway));
        });
        var telemetry = new RecordingTelemetry();
        var handler = BuildHandler(telemetry, inner, maxRetryAttempts: 2);
        using var invoker = new HttpMessageInvoker(handler);

        await invoker.SendAsync(new HttpRequestMessage(HttpMethod.Get, "https://api.example.com/v1/items"), CancellationToken.None);

        Assert.Equal(new[] { 1, 2 }, headers);
    }

    [Fact]
    public async Task Circuit_breaker_opens_after_threshold()
    {
        var telemetry = new RecordingTelemetry();
        var inner = new StubHandler((request, _) => Task.FromResult(new HttpResponseMessage(HttpStatusCode.InternalServerError)));
        var options = new PlatformHttpResilienceOptions
        {
            MaxRetryAttempts = 1,
            CircuitBreakerMinimumThroughput = 2,
            CircuitBreakerFailureRatio = 0.5,
            CircuitBreakerSamplingDuration = TimeSpan.FromMinutes(1),
            CircuitBreakerBreakDuration = TimeSpan.FromMinutes(1)
        };
        var handler = BuildHandler(telemetry, inner, options);
        using var invoker = new HttpMessageInvoker(handler);

        for (var i = 0; i < 2; i++)
            await invoker.SendAsync(new HttpRequestMessage(HttpMethod.Get, "https://api.example.com/v1/items"), CancellationToken.None);

        await Assert.ThrowsAsync<PlatformHttpCircuitOpenException>(async () =>
            await invoker.SendAsync(new HttpRequestMessage(HttpMethod.Get, "https://api.example.com/v1/items"), CancellationToken.None));
    }

    private static PlatformHttpResilienceHandler BuildHandler(RecordingTelemetry telemetry, StubHandler inner, int maxRetryAttempts, HashSet<string>? extraIdempotent = null)
    {
        var options = new PlatformHttpResilienceOptions
        {
            MaxRetryAttempts = maxRetryAttempts,
            RetryBaseDelay = TimeSpan.Zero,
            CircuitBreakerBreakDuration = TimeSpan.FromMilliseconds(1)
        };
        if (extraIdempotent is not null)
            options.IdempotentMethods = extraIdempotent;
        return BuildHandler(telemetry, inner, options);
    }

    private static PlatformHttpResilienceHandler BuildHandler(RecordingTelemetry telemetry, StubHandler inner, PlatformHttpResilienceOptions options)
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformWebTelemetry();
        services.AddSingleton<IHttpResilienceTelemetry>(telemetry);
        var sp = services.BuildServiceProvider();
        var handler = ActivatorUtilities.CreateInstance<PlatformHttpResilienceHandler>(sp,
            Options.Create(options),
            telemetry,
            sp.GetRequiredService<IPlatformWebTelemetry>(),
            sp.GetRequiredService<ILogger<PlatformHttpResilienceHandler>>());
        handler.InnerHandler = inner;
        return handler;
    }

    private sealed class StubHandler : HttpMessageHandler
    {
        private readonly Func<HttpRequestMessage, CancellationToken, Task<HttpResponseMessage>> _handler;
        public StubHandler(Func<HttpRequestMessage, CancellationToken, Task<HttpResponseMessage>> handler) => _handler = handler;
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken) => _handler(request, cancellationToken);
    }

    private sealed class RecordingTelemetry : IHttpResilienceTelemetry
    {
        private readonly List<HttpResilienceEvent> _events = new();
        public IReadOnlyList<HttpResilienceEvent> Events => _events;
        public void Record(HttpResilienceEvent resilienceEvent) => _events.Add(resilienceEvent);
    }
}
