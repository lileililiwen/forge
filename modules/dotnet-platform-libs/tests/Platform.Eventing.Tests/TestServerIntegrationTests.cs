using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Eventing.DependencyInjection;

namespace Platform.Eventing.Tests;

public class TestServerIntegrationTests
{
    [Fact]
    public async Task Host_with_AddPlatformEventing_resolves_default_options_and_clock()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"boundedCapacity\":1024", body);
        Assert.Contains("\"clock\":\"", body);
    }

    [Fact]
    public async Task Host_with_configured_section_overrides_defaults()
    {
        await using var app = BuildApp(extraConfiguration: new Dictionary<string, string?>
        {
            ["Eventing:InProcessBoundedCapacity"] = "256",
        });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"boundedCapacity\":256", body);
    }

    [Fact]
    public async Task Host_publishes_envelope_through_consumer_handler()
    {
        var handler = new RecordingOrderPlacedHandler();

        await using var app = BuildApp(handler: handler);
        var client = app.GetTestClient();

        var response = await client.PostAsync("/publish", content: null);

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        await WaitForHandlerAsync(handler, expected: 1);
        var (_, envelope) = Assert.Single(handler.Received);
        Assert.Equal(typeof(OrderPlaced).AssemblyQualifiedName, envelope.PayloadType);
    }

    private static WebApplication BuildApp(
        IReadOnlyDictionary<string, string?>? extraConfiguration = null,
        RecordingOrderPlacedHandler? handler = null)
    {
        var options = new WebApplicationOptions
        {
            EnvironmentName = "Testing",
        };
        var builder = WebApplication.CreateBuilder(options);
        builder.WebHost.UseTestServer();
        if (extraConfiguration is not null)
        {
            builder.Configuration.AddInMemoryCollection(extraConfiguration);
        }

        builder.Services.AddPlatformEventing(eventingOptions =>
            builder.Configuration
                .GetSection(EventingOptions.SectionName)
                .Bind(eventingOptions));
        builder.Services.AddPlatformEventingInProcess();
        if (handler is not null)
        {
            builder.Services.AddSingleton<IIntegrationEventHandler<OrderPlaced>>(handler);
        }

        var app = builder.Build();
        app.MapGet("/probe", (IOptions<EventingOptions> options, IClock clock) => new
        {
            boundedCapacity = options.Value.InProcessBoundedCapacity,
            clock = clock.UtcNow.ToString("O"),
        });
        app.MapPost("/publish", async (
            [FromServices] IEventBus bus,
            [FromServices] IIntegrationEventEnvelopeSerializer serializer,
            [FromServices] IClock clock) =>
        {
            var orderPlaced = new OrderPlaced(clock.UtcNow, Guid.NewGuid(), 42m);
            var envelope = serializer.Serialize(orderPlaced);
            await bus.PublishAsync(envelope);
            return Results.Ok(new
            {
                messageId = envelope.MessageId,
                payloadType = envelope.PayloadType,
            });
        });

        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }

    private static async Task WaitForHandlerAsync(
        RecordingOrderPlacedHandler handler,
        int expected,
        int timeoutMs = 2000)
    {
        var deadline = Environment.TickCount + timeoutMs;
        while (Environment.TickCount < deadline)
        {
            if (handler.Received.Count >= expected)
            {
                return;
            }
            await Task.Delay(10);
        }
    }
}
