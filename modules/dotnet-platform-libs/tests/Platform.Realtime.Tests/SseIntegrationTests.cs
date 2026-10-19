using Microsoft.AspNetCore.Mvc.Testing;
using Platform.Realtime.Tests;

namespace Platform.Realtime.Tests;

public class SseIntegrationTests
{
    [Fact]
    public async Task Unauthorized_connection_is_rejected_with_401()
    {
        using var app = TestDoubles.BuildApp(new TestDoubles.DenyAuthorizer(), new TestDoubles.ShortSource());
        var client = app.GetTestClient();

        var response = await client.GetAsync("/sse");

        Assert.Equal(401, (int)response.StatusCode);
    }

    [Fact]
    public async Task Authorized_connection_streams_events_with_sse_headers()
    {
        using var app = TestDoubles.BuildApp(new TestDoubles.AllowAuthorizer(), new TestDoubles.ShortSource());
        var client = app.GetTestClient();

        var response = await client.GetAsync("/sse");
        var body = await response.Content.ReadAsStringAsync();

        Assert.Equal(200, (int)response.StatusCode);
        Assert.Equal("text/event-stream", response.Content.Headers.ContentType?.MediaType);
        Assert.Contains("data: hello", body);
        Assert.Contains("event: tick", body);
    }

    [Fact]
    public async Task Cross_tenant_messages_are_filtered_by_application_router()
    {
        using var app = TestDoubles.BuildApp(
            new TestDoubles.AllowAuthorizer(),
            new TestDoubles.RoutingSource(),
            new TestDoubles.TenantResolver("t1"),
            new TestDoubles.SameTenantRouter());
        var client = app.GetTestClient();

        var response = await client.GetAsync("/sse");
        var body = await response.Content.ReadAsStringAsync();

        Assert.Contains("same-tenant", body);
        Assert.DoesNotContain("cross-tenant", body);
        Assert.DoesNotContain("broadcast", body);
    }

    [Fact]
    public async Task Oversized_payloads_are_dropped_by_the_sink()
    {
        using var app = TestDoubles.BuildApp(
            new TestDoubles.AllowAuthorizer(),
            new TestDoubles.PayloadSource(),
            configure: o => o.MaxPayloadBytes = 10);
        var client = app.GetTestClient();

        var response = await client.GetAsync("/sse");
        var body = await response.Content.ReadAsStringAsync();

        Assert.Contains("data: ok", body);
        Assert.DoesNotContain("this is definitely", body);
    }

    [Fact]
    public async Task Connection_limit_rejects_extra_connections_with_503()
    {
        using var app = TestDoubles.BuildApp(
            new TestDoubles.AllowAuthorizer(),
            new TestDoubles.LongSource(),
            configure: o => o.MaxConcurrentConnections = 1);
        var client = app.GetTestClient();

        using var cts = new CancellationTokenSource();
        var first = await client.GetAsync("/sse", HttpCompletionOption.ResponseHeadersRead, cts.Token);
        Assert.Equal(200, (int)first.StatusCode);

        var second = await client.GetAsync("/sse", HttpCompletionOption.ResponseHeadersRead);
        Assert.Equal(503, (int)second.StatusCode);

        cts.Cancel();
        first.Dispose();
        second.Dispose();
    }
}
