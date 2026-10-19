using System.Net;
using System.Threading;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Http;
using Microsoft.Extensions.Logging;
using Polly.CircuitBreaker;
using Platform.Http.Resilience;
using Platform.Http.Resilience.DependencyInjection;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.Http.Resilience.Tests;

public sealed class HttpResilienceTests
{
    [Fact]
    public async Task Post_transient_failure_is_not_retried()
    {
        var (client, stub, _) = BuildClient(MaxRetryAttempts(2));
        stub.Responder = (_, _, _) => Task.FromResult(new HttpResponseMessage(HttpStatusCode.ServiceUnavailable));

        await client.PostAsync("https://api.example.com/v1/items", new StringContent("{}"), CancellationToken.None);

        Assert.Equal(1, stub.Calls);
    }

    [Fact]
    public async Task Get_transient_failure_is_retried_to_the_configured_limit()
    {
        var (client, stub, _) = BuildClient(MaxRetryAttempts(2));
        stub.Responder = (_, _, _) => Task.FromResult(new HttpResponseMessage(HttpStatusCode.ServiceUnavailable));

        await client.GetAsync("https://api.example.com/v1/items");

        Assert.Equal(3, stub.Calls);
    }

    [Fact]
    public async Task Successful_get_is_not_retried()
    {
        var (client, stub, _) = BuildClient(MaxRetryAttempts(3));
        stub.Responder = (_, _, _) => Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK));

        var response = await client.GetAsync("https://api.example.com/v1/items");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Equal(1, stub.Calls);
    }

    [Fact]
    public async Task Non_idempotent_method_with_extra_idempotent_set_is_retried()
    {
        var (client, stub, _) = BuildClient(o =>
        {
            o.MaxRetryAttempts = 2;
            o.IdempotentMethods.Add("POST");
        });
        stub.Responder = (_, _, _) => Task.FromResult(new HttpResponseMessage(HttpStatusCode.ServiceUnavailable));

        await client.PostAsync("https://api.example.com/v1/items", new StringContent("{}"), CancellationToken.None);

        Assert.Equal(3, stub.Calls);
    }

    [Fact]
    public async Task Circuit_breaker_opens_and_records_circuit_open_telemetry()
    {
        var (client, stub, telemetry) = BuildClient(o =>
        {
            o.MaxRetryAttempts = 1;
            o.CircuitBreakerFailureRatio = 1.0;
            o.CircuitBreakerMinimumThroughput = 2;
            o.CircuitBreakerBreakDuration = TimeSpan.FromMinutes(1);
            o.AttemptTimeout = TimeSpan.FromSeconds(5);
        });
        stub.Responder = (_, _, _) => Task.FromResult(new HttpResponseMessage(HttpStatusCode.InternalServerError));

        // First call trips the circuit (initial attempt + one retry = 2 failures, minimum throughput 2).
        await client.GetAsync("https://api.example.com/v1/items");
        // Second call is rejected by the open circuit.
        await Assert.ThrowsAnyAsync<Exception>(() => client.GetAsync("https://api.example.com/v1/items"));

        Assert.Contains(telemetry.Events, e => e.Decision == PlatformHttpResilienceDecision.CircuitOpen);
    }

    [Fact]
    public async Task Cancellation_is_preserved_and_recorded_without_being_retried()
    {
        var (client, stub, telemetry) = BuildClient(o =>
        {
            o.MaxRetryAttempts = 3;
            o.AttemptTimeout = TimeSpan.FromSeconds(30);
            o.TotalTimeout = TimeSpan.FromSeconds(30);
        });
        stub.Responder = async (_, _, ct) =>
        {
            await Task.Delay(Timeout.InfiniteTimeSpan, ct);
            return new HttpResponseMessage(HttpStatusCode.OK);
        };

        using var cts = new CancellationTokenSource();
        cts.CancelAfter(50);
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => client.GetAsync("https://api.example.com/v1/items", cts.Token));

        Assert.True(stub.Calls >= 1);
        Assert.Contains(telemetry.Events, e => e.Decision == PlatformHttpResilienceDecision.Cancelled);
    }

    [Fact]
    public async Task Per_attempt_timeout_short_circuits_without_hanging()
    {
        var (client, stub, _) = BuildClient(o =>
        {
            o.MaxRetryAttempts = 1;
            o.AttemptTimeout = TimeSpan.FromMilliseconds(50);
        });
        stub.Responder = async (_, _, ct) =>
        {
            await Task.Delay(TimeSpan.FromSeconds(2), ct);
            return new HttpResponseMessage(HttpStatusCode.OK);
        };

        var started = System.Diagnostics.Stopwatch.StartNew();
        await Assert.ThrowsAnyAsync<Exception>(() => client.GetAsync("https://api.example.com/v1/items"));
        started.Stop();

        Assert.True(started.Elapsed < TimeSpan.FromSeconds(2), "Request should have been short-circuited by the per-attempt timeout.");
    }

    [Fact]
    public void Options_validation_rejects_out_of_range_retry_count()
    {
        var options = new PlatformHttpResilienceOptions { MaxRetryAttempts = 99 };
        Assert.Contains(options.Validate(), e => e.Contains("MaxRetryAttempts", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Options_validation_rejects_out_of_range_circuit_ratio()
    {
        var options = new PlatformHttpResilienceOptions { CircuitBreakerFailureRatio = 2 };
        Assert.Contains(options.Validate(), e => e.Contains("FailureRatio", StringComparison.OrdinalIgnoreCase));
    }

    private static Action<PlatformHttpResilienceOptions> MaxRetryAttempts(int value) => o => o.MaxRetryAttempts = value;

    private static (HttpClient Client, StubBehavior Stub, RecordingTelemetry Telemetry) BuildClient(Action<PlatformHttpResilienceOptions>? configure = null)
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformWebTelemetry();
        var telemetry = new RecordingTelemetry();
        services.AddSingleton<IPlatformHttpResilienceTelemetry>(telemetry);
        var stub = new StubBehavior();
        services.AddSingleton(stub);
        services.AddHttpClient("test")
            .AddPlatformHttpResilience(configure ?? (_ => { }))
            .ConfigurePrimaryHttpMessageHandler(() => new StubHandler(stub));
        var provider = services.BuildServiceProvider();
        var client = provider.GetRequiredService<IHttpClientFactory>().CreateClient("test");
        return (client, stub, telemetry);
    }

    private sealed class StubBehavior
    {
        private int _calls;
        public int Calls => _calls;
        public Func<HttpRequestMessage, int, CancellationToken, Task<HttpResponseMessage>>? Responder { get; set; }

        public Task<HttpResponseMessage> Invoke(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            Interlocked.Increment(ref _calls);
            if (Responder is null)
                return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK));
            return Responder(request, _calls, cancellationToken);
        }
    }

    private sealed class StubHandler : HttpMessageHandler
    {
        private readonly StubBehavior _behavior;
        public StubHandler(StubBehavior behavior) => _behavior = behavior;
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken) =>
            _behavior.Invoke(request, cancellationToken);
    }

    private sealed class RecordingTelemetry : IPlatformHttpResilienceTelemetry
    {
        private readonly List<PlatformHttpResilienceEvent> _events = new();
        public IReadOnlyList<PlatformHttpResilienceEvent> Events => _events;
        public void Record(PlatformHttpResilienceEvent resilienceEvent) => _events.Add(resilienceEvent);
    }
}
