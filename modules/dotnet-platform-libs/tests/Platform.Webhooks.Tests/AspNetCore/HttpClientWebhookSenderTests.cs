using System.Net;
using System.Net.Http;
using System.Net.Sockets;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Http;
using Platform.Webhooks.AspNetCore.Outbound;
using Platform.Webhooks.Contracts.Outbound;

namespace Platform.Webhooks.Tests.AspNetCore;

public sealed class HttpClientWebhookSenderTests
{
    [Fact]
    public async Task Returns_transport_failure_when_target_unreachable()
    {
        var services = new ServiceCollection();
        services.AddHttpClient<IWebhookHttpSender, HttpClientWebhookSender>();
        await using var provider = services.BuildServiceProvider();
        var sender = provider.GetRequiredService<IWebhookHttpSender>();

        var closedPort = GetClosedLoopbackPort();
        var result = await sender.SendAsync(new WebhookHttpSendRequest(new Uri($"http://127.0.0.1:{closedPort}/hook"), "{}", "X-Webhook-Signature", "sig", "X-Webhook-Event", "evt", "X-Webhook-Event-Type", "order.created", TimeSpan.FromSeconds(2)));

        Assert.Null(result.ResponseStatus);
        Assert.NotNull(result.Failure);
        Assert.True(result.Failure!.Transient);
    }

    [Fact]
    public async Task Returns_retryable_for_5xx()
    {
        var server = await StartReceiverAsync(async context =>
        {
            context.Response.StatusCode = (int)HttpStatusCode.ServiceUnavailable;
            await context.Response.WriteAsync("down");
        });
        try
        {
            var services = new ServiceCollection();
            services.AddHttpClient<IWebhookHttpSender, HttpClientWebhookSender>().ConfigurePrimaryHttpMessageHandler(() => server.CreateHandler());
            await using var provider = services.BuildServiceProvider();
            var sender = provider.GetRequiredService<IWebhookHttpSender>();
            var result = await sender.SendAsync(new WebhookHttpSendRequest(new Uri("http://server/hook"), "{}", "X-Webhook-Signature", "sig", "X-Webhook-Event", "evt", "X-Webhook-Event-Type", "order.created", TimeSpan.FromSeconds(5)));
            Assert.Equal(503, result.ResponseStatus);
            Assert.True(result.Failure!.Transient);
        }
        finally
        {
            server.Dispose();
        }
    }

    [Fact]
    public async Task Returns_permanent_for_4xx()
    {
        var server = await StartReceiverAsync(async context =>
        {
            context.Response.StatusCode = (int)HttpStatusCode.BadRequest;
            await context.Response.WriteAsync("nope");
        });
        try
        {
            var services = new ServiceCollection();
            services.AddHttpClient<IWebhookHttpSender, HttpClientWebhookSender>().ConfigurePrimaryHttpMessageHandler(() => server.CreateHandler());
            await using var provider = services.BuildServiceProvider();
            var sender = provider.GetRequiredService<IWebhookHttpSender>();
            var result = await sender.SendAsync(new WebhookHttpSendRequest(new Uri("http://server/hook"), "{}", "X-Webhook-Signature", "sig", "X-Webhook-Event", "evt", "X-Webhook-Event-Type", "order.created", TimeSpan.FromSeconds(5)));
            Assert.Equal(400, result.ResponseStatus);
            Assert.False(result.Failure!.Transient);
        }
        finally
        {
            server.Dispose();
        }
    }

    [Fact]
    public async Task Returns_success_for_2xx()
    {
        var server = await StartReceiverAsync(async context =>
        {
            context.Response.StatusCode = (int)HttpStatusCode.OK;
            await context.Response.WriteAsync("ok");
        });
        try
        {
            var services = new ServiceCollection();
            services.AddHttpClient<IWebhookHttpSender, HttpClientWebhookSender>().ConfigurePrimaryHttpMessageHandler(() => server.CreateHandler());
            await using var provider = services.BuildServiceProvider();
            var sender = provider.GetRequiredService<IWebhookHttpSender>();
            var result = await sender.SendAsync(new WebhookHttpSendRequest(new Uri("http://server/hook"), "{}", "X-Webhook-Signature", "sig", "X-Webhook-Event", "evt", "X-Webhook-Event-Type", "order.created", TimeSpan.FromSeconds(5)));
            Assert.Equal(200, result.ResponseStatus);
            Assert.Null(result.Failure);
        }
        finally
        {
            server.Dispose();
        }
    }

    private static int GetClosedLoopbackPort()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        listener.Stop();
        return port;
    }

    private static async Task<TestServer> StartReceiverAsync(Func<HttpContext, Task> handler)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        var app = builder.Build();
        app.MapPost("/hook", handler);
        await app.StartAsync();
        return app.GetTestServer();
    }
}
