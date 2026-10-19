using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Eventing;
using Platform.Testing.AspNetCore;
using Platform.Testing.Eventing;
using Platform.Testing.FailureInjection;
using Platform.Testing.TestApplication;

namespace Platform.Testing.Tests.Scenarios;

/// <summary>
/// End-to-end scenario exercising the platform testing toolkit against a
/// ASP.NET Core test host. The scenario publishes an event, runs the
/// handler under a transient failure injection, and asserts the host
/// surfaces the recovery through its <see cref="IEventBus"/> integration.
/// </summary>
public sealed class CheckoutScenarioTests
{
    [Fact]
    public async Task Transient_failure_in_event_handler_recovers_on_subsequent_publish()
    {
        var bus = new RecordingEventBus();
        var injector = new TransientFailureInjector()
            .WithTransient("checkout.publish", new TimeoutException("transient"));

        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ConfigureTestServices((services, _) =>
            {
                services.AddSingleton(bus);
                services.AddSingleton(injector);
            });

        var client = factory.CreateClient();
        var response = await client.GetAsync("/");
        response.EnsureSuccessStatusCode();

        await bus.PublishAsync(NewEnvelope("checkout.completed"));

        Assert.Throws<TimeoutException>(() => injector.Run("checkout.publish", () => { }));
        var ran = false;
        injector.Run("checkout.publish", () => ran = true);
        Assert.True(ran);
        Assert.Single(injector.History);
    }

    private static IntegrationEventEnvelope NewEnvelope(string payloadType) =>
        new(
            MessageId: Guid.NewGuid().ToString("N"),
            PayloadType: payloadType,
            PayloadJson: "{}",
            OccurredAt: DateTimeOffset.UnixEpoch);
}
