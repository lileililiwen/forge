using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.Eventing;
using Platform.Eventing.DependencyInjection;

namespace Platform.ConsumerConformance.Tests;

public sealed class EventingRegistrationTests
{
    [Fact]
    public void AddPlatformEventing_registers_options_serializer_and_deserializer()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformEventing();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<EventingOptions>>().Value;
        Assert.Equal("Eventing", EventingOptions.SectionName);
        Assert.Equal(1024, options.InProcessBoundedCapacity);
        Assert.IsType<IntegrationEventEnvelopeSerializer>(provider.GetRequiredService<IIntegrationEventEnvelopeSerializer>());
        Assert.IsType<IntegrationEventEnvelopeDeserializer>(provider.GetRequiredService<IIntegrationEventEnvelopeDeserializer>());
        Assert.IsType<SystemClock>(provider.GetRequiredService<IClock>());
    }

    [Fact]
    public async Task AddPlatformEventingInProcess_registers_in_process_bus()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformEventing();
        services.AddPlatformEventingInProcess();

        await using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var bus = provider.GetRequiredService<IEventBus>();
        Assert.IsType<InProcessEventBus>(bus);
    }

    [Fact]
    public async Task AddPlatformEventing_does_not_register_an_in_process_bus()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformEventing();

        await using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        Assert.Null(provider.GetService<InProcessEventBus>());
    }

    [Fact]
    public async Task InProcessEventBus_publishes_to_registered_handlers()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformEventing();
        services.AddPlatformEventingInProcess();
        services.AddSingleton<ConsumerEventRecorder>();
        services.AddSingleton<IIntegrationEventHandler<TestIntegrationEvent>>(sp => sp.GetRequiredService<ConsumerEventRecorder>());

        await using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var bus = provider.GetRequiredService<IEventBus>();
        var envelope = new IntegrationEventEnvelope("message-1", typeof(TestIntegrationEvent).FullName!, "{\"Text\":\"hi\"}", new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero), null);
        await bus.PublishAsync(envelope, CancellationToken.None);

        var recorder = provider.GetRequiredService<ConsumerEventRecorder>();
        var completion = await recorder.Completed.WaitAsync(TimeSpan.FromSeconds(2));
        Assert.Equal("hi", completion.Text);
    }

    private sealed record TestIntegrationEvent(string Text) : IntegrationEvent(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

    private sealed class ConsumerEventRecorder : IIntegrationEventHandler<TestIntegrationEvent>
    {
        private TaskCompletionSource<TestIntegrationEvent>? _tcs;

        public string ConsumerName => "consumer-event-recorder";

        public Task<TestIntegrationEvent> Completed => (_tcs ??= new TaskCompletionSource<TestIntegrationEvent>(TaskCreationOptions.RunContinuationsAsynchronously)).Task;

        public Task HandleAsync(TestIntegrationEvent payload, IntegrationEventEnvelope envelope, CancellationToken cancellationToken)
        {
            _tcs?.TrySetResult(payload);
            return Task.CompletedTask;
        }
    }
}
