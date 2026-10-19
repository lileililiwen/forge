using System.Diagnostics;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Observability;
using Platform.Observability.DependencyInjection;
using Platform.Observability.Diagnostics;
using Platform.Observability.Hosting;

namespace Platform.Observability.Tests;

public sealed class CorrelationMiddlewareTests
{
    [Fact]
    public async Task Incoming_request_receives_generated_correlation_id()
    {
        await using var app = BuildApp(o => { });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.True(response.Headers.TryGetValues("X-Correlation-Id", out var values));
        var value = Assert.Single(values);
        Assert.False(string.IsNullOrWhiteSpace(value));
    }

    [Fact]
    public async Task Incoming_correlation_id_is_rejected_when_not_accepted()
    {
        await using var app = BuildApp(o => { });
        var client = app.GetTestClient();
        var request = new HttpRequestMessage(HttpMethod.Get, "/probe");
        request.Headers.Add("X-Correlation-Id", "client-supplied-id");

        var response = await client.SendAsync(request);

        Assert.True(response.Headers.TryGetValues("X-Correlation-Id", out var values));
        var echoed = Assert.Single(values);
        Assert.NotEqual("client-supplied-id", echoed);
    }

    [Fact]
    public async Task Incoming_correlation_id_is_accepted_when_enabled()
    {
        await using var app = BuildApp(o => o.AcceptIncomingCorrelationHeader = true);
        var client = app.GetTestClient();
        var request = new HttpRequestMessage(HttpMethod.Get, "/probe");
        request.Headers.Add("X-Correlation-Id", "client-supplied-id");

        var response = await client.SendAsync(request);

        Assert.True(response.Headers.TryGetValues("X-Correlation-Id", out var values));
        Assert.Equal("client-supplied-id", Assert.Single(values));
    }

    [Fact]
    public async Task Disabled_request_enrichment_does_not_emit_activity()
    {
        await using var app = BuildApp(o => o.EnableRequestEnrichment = false);
        using var listener = new RequestListener();
        ActivitySource.AddActivityListener(listener.Listener);
        var client = app.GetTestClient();

        await client.GetAsync("/probe");

        Assert.DoesNotContain(listener.Activities, a => a.OperationName == PlatformObservabilityNames.RequestCorrelationOperation);
    }

    [Fact]
    public async Task Enabled_request_enrichment_emits_activity_with_bounded_tags()
    {
        await using var app = BuildApp(o => o.EnableRequestEnrichment = true);
        using var listener = new RequestListener();
        ActivitySource.AddActivityListener(listener.Listener);
        var client = app.GetTestClient();

        await client.GetAsync("/probe");

        var activity = listener.Activities.FirstOrDefault(a => a.OperationName == PlatformObservabilityNames.RequestCorrelationOperation);
        Assert.NotNull(activity);
        var tag = activity!.TagObjects.ToDictionary(t => (string)t.Key, t => t.Value);
        Assert.Equal("/probe", tag[PlatformObservabilityNames.TagRoute]);
        Assert.Equal("GET", tag[PlatformObservabilityNames.TagMethod]);
    }

    private static WebApplication BuildApp(Action<PlatformObservabilityOptions> configure)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Development" });
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformObservability(configure);
        var app = builder.Build();
        app.UsePlatformObservability();
        app.MapGet("/probe", (HttpContext context) =>
        {
            var accessor = context.RequestServices.GetRequiredService<IPlatformCorrelationAccessor>();
            return Results.Json(new { correlationId = accessor.Current });
        });
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }

    private sealed class RequestListener : IDisposable
    {
        private readonly List<Activity> _activities = new();
        public ActivityListener Listener { get; }
        public IReadOnlyList<Activity> Activities => _activities;
        public RequestListener()
        {
            Listener = new ActivityListener
            {
                ShouldListenTo = source => source.Name == PlatformObservabilityNames.RequestActivitySource,
                Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData,
                ActivityStarted = activity => _activities.Add(activity),
            };
            ActivitySource.AddActivityListener(Listener);
        }
        public void Dispose() => Listener.Dispose();
    }
}
