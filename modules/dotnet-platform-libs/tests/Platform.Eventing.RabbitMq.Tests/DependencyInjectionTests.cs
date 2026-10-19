using Microsoft.Extensions.DependencyInjection;
using Platform.Eventing.Contracts;
using Platform.Eventing.RabbitMq.DependencyInjection;

namespace Platform.Eventing.RabbitMq.Tests;

public class DependencyInjectionTests
{
    private static ServiceProvider BuildProvider(Action<RabbitMqEventingOptions>? configure = null, Action<IServiceCollection>? seed = null)
    {
        var services = new ServiceCollection();
        seed?.Invoke(services);
        services.AddLogging();
        services.AddPlatformRabbitMqEventing(configure ?? (o => o.Exchange = "platform.events"));
        return services.BuildServiceProvider();
    }

    [Fact]
    public async Task Registration_maps_the_publisher_topology_and_factory()
    {
        await using var provider = BuildProvider();

        Assert.IsType<RabbitMqDurableEventPublisher>(provider.GetRequiredService<IDurableEventPublisher>());
        Assert.IsType<UnconfiguredRabbitMqEventTopology>(provider.GetRequiredService<IRabbitMqEventTopology>());
        Assert.IsType<RabbitMqChannelFactory>(provider.GetRequiredService<IRabbitMqChannelFactory>());
    }

    [Fact]
    public async Task Registration_is_idempotent()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformRabbitMqEventing(o => o.Exchange = "platform.events");
        services.AddPlatformRabbitMqEventing(o => o.Exchange = "platform.events");

        await using var provider = services.BuildServiceProvider();

        Assert.IsType<RabbitMqDurableEventPublisher>(provider.GetRequiredService<IDurableEventPublisher>());
    }

    [Fact]
    public async Task Consumer_owned_publisher_wins()
    {
        await using var provider = BuildProvider(seed: services =>
            services.AddSingleton<IDurableEventPublisher>(new StubPublisher()));

        Assert.IsType<StubPublisher>(provider.GetRequiredService<IDurableEventPublisher>());
    }

    [Fact]
    public async Task Consumer_owned_topology_and_factory_win()
    {
        await using var provider = BuildProvider(seed: services =>
        {
            services.AddSingleton<IRabbitMqEventTopology>(new RabbitMqEventTopology().Map("order.created", new RabbitMqEventBinding("order.created")));
            services.AddSingleton<IRabbitMqChannelFactory>(new StubFactory());
        });

        Assert.IsType<RabbitMqEventTopology>(provider.GetRequiredService<IRabbitMqEventTopology>());
        Assert.IsType<StubFactory>(provider.GetRequiredService<IRabbitMqChannelFactory>());
    }

    [Fact]
    public void Invalid_options_fail_at_registration()
    {
        var services = new ServiceCollection();
        services.AddLogging();

        Assert.Throws<ArgumentException>(() => services.AddPlatformRabbitMqEventing());
    }

    [Fact]
    public async Task Publisher_resolves_through_di_and_publishes_through_the_registered_factory()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var provider = BuildProvider(seed: services => services.AddSingleton<IRabbitMqChannelFactory>(factory));
        var topology = provider.GetRequiredService<IRabbitMqEventTopology>();
        Assert.IsType<UnconfiguredRabbitMqEventTopology>(topology);

        var publisher = provider.GetRequiredService<IDurableEventPublisher>();
        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.Equal("eventing.rabbitmq.topology_unconfigured", exception.Failure.Code);
        Assert.Equal(0, factory.CreateCalls);
    }

    private sealed class StubPublisher : IDurableEventPublisher
    {
        public Task PublishAsync(DurableEventEnvelope envelope, CancellationToken cancellationToken = default) => Task.CompletedTask;
    }

    private sealed class StubFactory : IRabbitMqChannelFactory
    {
        public Task<IRabbitMqChannel> CreateAsync(CancellationToken cancellationToken = default) =>
            Task.FromResult<IRabbitMqChannel>(new StubChannel());

        public ValueTask DisposeAsync() => ValueTask.CompletedTask;
    }

    private sealed class StubChannel : IRabbitMqChannel
    {
        public bool IsOpen => true;

        public Task DeclareExchangeAsync(CancellationToken cancellationToken = default) => Task.CompletedTask;

        public Task PublishAsync(RabbitMqOutboundMessage message, CancellationToken cancellationToken = default) => Task.CompletedTask;

        public ValueTask DisposeAsync() => ValueTask.CompletedTask;
    }
}
