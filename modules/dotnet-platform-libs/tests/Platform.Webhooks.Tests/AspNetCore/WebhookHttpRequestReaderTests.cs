using System.Net;
using System.Net.Http.Headers;
using System.Text;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Webhooks.AspNetCore.DependencyInjection;
using Platform.Webhooks.AspNetCore.Inbound;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;

namespace Platform.Webhooks.Tests.AspNetCore;

public sealed class WebhookHttpRequestReaderTests
{
    [Fact]
    public async Task Reads_request_into_verification_request()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.PostAsync("/read?eventId=evt_1", new StringContent("{\"id\":\"evt_1\"}", Encoding.UTF8, "application/json"));

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"eventId\":\"evt_1\"", body);
        Assert.Contains("\"bodyLength\":14", body);
    }

    [Fact]
    public async Task Rejects_body_exceeding_limit()
    {
        await using var app = BuildApp(maximumBodyBytes: 4);
        var client = app.GetTestClient();

        var response = await client.PostAsync("/read?eventId=evt_1", new StringContent("{\"id\":\"evt_1\"}", Encoding.UTF8, "application/json"));

        Assert.Equal(HttpStatusCode.RequestEntityTooLarge, response.StatusCode);
    }

    [Fact]
    public async Task Status_endpoint_returns_backend_and_ssrf_metadata()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/_webhooks/health");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"backend\":", body);
        Assert.Contains("\"ssrf\":", body);
    }

    private static WebApplication BuildApp(long? maximumBodyBytes = null)
    {
        var options = new WebhookOptions();
        if (maximumBodyBytes.HasValue) options = new WebhookOptions { MaximumInboundBodyBytes = maximumBodyBytes.Value };
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformWebhooksAspNetCore(_ => { });
        var app = builder.Build();
        app.MapPost("/read", async (HttpRequest request, [FromQuery] string eventId) =>
        {
            var provider = new WebhookProviderId("test");
            var verification = await WebhookHttpRequestReader.TryReadAsync(request, provider, eventId, options.MaximumInboundBodyBytes);
            if (verification is null) return Results.StatusCode(StatusCodes.Status413PayloadTooLarge);
            return Results.Ok(new
            {
                provider = verification.Provider.Value,
                eventId = verification.EventId,
                bodyLength = verification.Body.Length,
            });
        });
        app.MapPlatformWebhookStatus();
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }

    private static void CopyInto(WebhookOptions source, WebhookOptions target)
    {
        target.GetType().GetProperty(nameof(WebhookOptions.MaximumInboundBodyBytes))!.SetValue(target, source.MaximumInboundBodyBytes);
    }
}
