using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.Persistence.EfCore;
using Platform.Persistence.EfCore.Audit;
using Platform.Persistence.EfCore.DependencyInjection;
using Platform.Persistence.EfCore.Interceptors;

namespace Platform.ConsumerConformance.Tests;

public sealed class PersistenceRegistrationTests
{
    [Fact]
    public void AddPlatformPersistenceEfCore_registers_options_clock_and_actor_accessor()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformPersistenceEfCore();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<PersistenceOptions>>().Value;
        Assert.False(options.EnableAuditInterception);
        Assert.False(options.EnableSoftDeleteInterception);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<SystemClock>(clock);
        var actor = provider.GetRequiredService<IActorAccessor>();
        Assert.Null(actor.SubjectId);
        var interceptor = provider.GetRequiredService<PlatformSaveChangesInterceptor>();
        Assert.NotNull(interceptor);
    }

    [Fact]
    public void AddPlatformPersistenceEfCore_applies_configure_delegate()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformPersistenceEfCore(options => options.EnableAuditInterception = true);

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<PersistenceOptions>>().Value;
        Assert.True(options.EnableAuditInterception);
    }

    [Fact]
    public void AddPlatformPersistenceEfCore_honours_consumer_clock()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        services.AddPlatformPersistenceEfCore();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<FixedClock>(clock);
    }
}
