using Microsoft.Extensions.DependencyInjection;
using Platform.ConsumerConformance.Fixtures;
using Platform.Storage.Contracts;
using Platform.Storage.DependencyInjection;
using Platform.Storage.Keys;

namespace Platform.ConsumerConformance.Tests;

public sealed class StorageRegistrationTests
{
    [Fact]
    public void AddPlatformStorage_registers_consumer_supplied_object_storage()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformStorage<InMemoryObjectStorage>();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        using var scope = provider.CreateScope();
        var storage = scope.ServiceProvider.GetRequiredService<IObjectStorage>();
        Assert.IsType<InMemoryObjectStorage>(storage);
    }

    [Fact]
    public void AddPlatformStorage_throws_when_no_consumer_implementation_is_registered()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        using var scope = provider.CreateScope();
        Assert.Throws<InvalidOperationException>(() => scope.ServiceProvider.GetRequiredService<IObjectStorage>());
    }

    private sealed class InMemoryObjectStorage : IObjectStorage
    {
        public Task<StorageOutcome> UploadAsync(StorageUploadRequest request, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public Task<StorageDownloadResult> DownloadAsync(StorageObjectKey key, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public Task<StorageObjectMetadata?> GetMetadataAsync(StorageObjectKey key, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public Task<StorageOutcome> DeleteAsync(StorageObjectKey key, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public Task<(StorageOutcome Outcome, PresignedOperation? Operation)> PresignAsync(PresignRequest request, CancellationToken cancellationToken = default) => throw new NotImplementedException();
    }
}
