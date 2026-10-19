using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.ConsumerConformance.Fixtures;
using Platform.Quota;
using Platform.Quota.Contracts;
using Platform.Quota.DependencyInjection;
using Platform.Quota.Stores;
using Platform.Quota.Testing;

namespace Platform.ConsumerConformance.Tests;

public sealed class QuotaRegistrationTests
{
    [Fact]
    public void AddPlatformQuota_registers_in_memory_store_and_options()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformQuota();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var store = provider.GetRequiredService<IQuotaStore>();
        Assert.IsType<InMemoryQuotaStore>(store);
    }

    [Fact]
    public async Task AddPlatformQuota_enforces_bounded_reservation_capacity()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformQuota();

        await using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var store = provider.GetRequiredService<IQuotaStore>();
        var subject = new QuotaSubject("user-a");
        var resource = new QuotaResource("api-calls");
        var window = new QuotaWindow(
            new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero),
            new DateTimeOffset(2026, 12, 31, 0, 0, 0, TimeSpan.Zero));

        var first = await store.ReserveAsync(subject, resource, window, 3, "op-1", 1, null, CancellationToken.None);
        var second = await store.ReserveAsync(subject, resource, window, 3, "op-2", 1, null, CancellationToken.None);
        var third = await store.ReserveAsync(subject, resource, window, 3, "op-3", 1, null, CancellationToken.None);
        var fourth = await store.ReserveAsync(subject, resource, window, 3, "op-4", 1, null, CancellationToken.None);

        Assert.Equal(QuotaLifecycleStatus.Reserved, first.Status);
        Assert.Equal(QuotaLifecycleStatus.Reserved, second.Status);
        Assert.Equal(QuotaLifecycleStatus.Reserved, third.Status);
        Assert.Equal(QuotaLifecycleStatus.Denied, fourth.Status);
    }

    [Fact]
    public void Quota_testing_builder_is_available_separately()
    {
        var builder = new QuotaScenarioBuilder()
            .WithSubject("user-a")
            .WithResource("api-calls")
            .WithLimit(50);
        Assert.Equal("user-a", builder.Subject.Value);
        Assert.Equal("api-calls", builder.Resource.Value);
        Assert.Equal(50, builder.Limit);
    }
}
