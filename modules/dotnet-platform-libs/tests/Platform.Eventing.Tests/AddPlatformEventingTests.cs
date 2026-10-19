using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Eventing.DependencyInjection;

namespace Platform.Eventing.Tests;

public class AddPlatformEventingTests
{
    [Fact]
    public void Default_registration_binds_options_with_documented_defaults()
    {
        var services = new ServiceCollection();

        services.AddPlatformEventing();

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<EventingOptions>>().Value;

        Assert.Equal(1024, options.InProcessBoundedCapacity);
    }

    [Fact]
    public void Configure_delegate_overrides_defaults()
    {
        var services = new ServiceCollection();

        services.AddPlatformEventing(options => options.InProcessBoundedCapacity = 256);

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<EventingOptions>>().Value;

        Assert.Equal(256, options.InProcessBoundedCapacity);
    }

    [Fact]
    public void Registration_resolves_clock_serializer_and_deserializer()
    {
        var services = new ServiceCollection();

        services.AddPlatformEventing();

        using var provider = services.BuildServiceProvider();
        Assert.NotNull(provider.GetRequiredService<IClock>());
        Assert.NotNull(provider.GetRequiredService<IIntegrationEventEnvelopeSerializer>());
        Assert.NotNull(provider.GetRequiredService<IIntegrationEventEnvelopeDeserializer>());
    }

    [Fact]
    public void Registration_preserves_existing_clock()
    {
        var services = new ServiceCollection();
        var preExisting = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        services.AddSingleton<IClock>(preExisting);

        services.AddPlatformEventing();

        using var provider = services.BuildServiceProvider();
        Assert.Same(preExisting, provider.GetRequiredService<IClock>());
    }

    [Fact]
    public async Task In_process_registration_registers_IEventBus()
    {
        var services = new ServiceCollection();
        services.AddLogging();

        services.AddPlatformEventing();
        services.AddPlatformEventingInProcess();

        await using var provider = services.BuildServiceProvider();
        var bus = provider.GetRequiredService<IEventBus>();

        Assert.NotNull(bus);
        Assert.IsType<InProcessEventBus>(bus);
    }

    [Fact]
    public async Task In_process_registration_can_be_disposed()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformEventing();
        services.AddPlatformEventingInProcess();

        await using var provider = services.BuildServiceProvider();
        var bus = (InProcessEventBus)provider.GetRequiredService<IEventBus>();
        await bus.DisposeAsync();
    }

    [Fact]
    public void Null_services_throws()
    {
        Assert.Throws<ArgumentNullException>(() => ((IServiceCollection)null!).AddPlatformEventing());
        Assert.Throws<ArgumentNullException>(
            () => ((IServiceCollection)null!).AddPlatformEventing(_ => { }));
        Assert.Throws<ArgumentNullException>(
            () => new ServiceCollection().AddPlatformEventing(configure: null!));
        Assert.Throws<ArgumentNullException>(
            () => ((IServiceCollection)null!).AddPlatformEventingInProcess());
    }
}
