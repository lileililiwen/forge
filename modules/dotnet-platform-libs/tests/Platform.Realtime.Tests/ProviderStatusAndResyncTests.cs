using System.Text;
using System.Text.Json;
using Microsoft.AspNetCore.Mvc.Testing;
using Platform.Realtime.Delivery;
using Platform.Realtime.Tests;

namespace Platform.Realtime.Tests;

public class ProviderStatusAndResyncTests
{
    [Fact]
    public async Task Status_endpoint_exposes_safe_sse_status()
    {
        using var app = TestDoubles.BuildApp(new TestDoubles.AllowAuthorizer(), new TestDoubles.ShortSource());
        var client = app.GetTestClient();

        var response = await client.GetAsync("/status");
        var body = await response.Content.ReadAsStringAsync();

        Assert.Equal(200, (int)response.StatusCode);
        Assert.Contains("SSE", body);
        Assert.DoesNotContain("ConnectionString", body, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Resync_endpoint_returns_404_when_no_handler_registered()
    {
        using var app = TestDoubles.BuildApp(new TestDoubles.AllowAuthorizer(), new TestDoubles.ShortSource());
        var client = app.GetTestClient();

        var response = await client.PostAsync(
            "/resync",
            new StringContent(JsonSerializer.Serialize(new RealtimeResyncRequest { ConnectionId = "c1" }), Encoding.UTF8, "application/json"));

        Assert.Equal(404, (int)response.StatusCode);
    }

    [Fact]
    public async Task Resync_endpoint_routes_request_to_application_handler()
    {
        var handler = new TestDoubles.TestResyncHandler();
        using var app = TestDoubles.BuildApp(
            new TestDoubles.AllowAuthorizer(),
            new TestDoubles.ShortSource(),
            resyncHandler: handler);
        var client = app.GetTestClient();

        var response = await client.PostAsync(
            "/resync",
            new StringContent(
                JsonSerializer.Serialize(new RealtimeResyncRequest { ConnectionId = "c1", LastReceivedToken = "t0", TenantId = "t9" }),
                Encoding.UTF8,
                "application/json"));

        Assert.Equal(202, (int)response.StatusCode);
        Assert.NotNull(handler.Captured);
        Assert.Equal("c1", handler.Captured!.ConnectionId);
        Assert.Equal("t0", handler.Captured.LastReceivedToken);
        Assert.Equal("t9", handler.Captured.TenantId);
    }
}
