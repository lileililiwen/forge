using Microsoft.Extensions.DependencyInjection;
using Platform.ConsumerConformance.Fixtures;
using Platform.Mailing;
using Platform.Mailing.DependencyInjection;
using Platform.RateLimiting;
using Platform.RateLimiting.DependencyInjection;
using Platform.Storage.Contracts;
using Platform.Storage.DependencyInjection;
using Platform.Storage.Keys;

namespace Platform.ConsumerConformance.Tests;

public sealed class OptInBoundaryTests
{
    [Fact]
    public void Calling_AddPlatformRateLimiting_only_registers_rate_limit_services()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformRateLimiting();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        Assert.NotNull(provider.GetService<IRateLimiter>());
        Assert.Null(provider.GetService<IMailService>());
        Assert.Null(provider.GetService<IObjectStorage>());
    }

    [Fact]
    public void Calling_AddPlatformMailing_only_registers_mailing_services()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformMailing();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        Assert.NotNull(provider.GetService<Microsoft.Extensions.Options.IOptions<MailingOptions>>());
        Assert.Null(provider.GetService<IRateLimiter>());
        Assert.Null(provider.GetService<IObjectStorage>());
    }

    [Fact]
    public void Calling_AddPlatformStorage_does_not_register_a_default_implementation()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformStorage<InMemoryObjectStorage>();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        using var scope = provider.CreateScope();
        Assert.NotNull(scope.ServiceProvider.GetService<IObjectStorage>());
        Assert.Null(provider.GetService<IMailService>());
    }

    [Fact]
    public void Combining_optional_packages_yields_aggregate_without_double_registration()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformRateLimiting();
        services.AddPlatformMailing();
        services.AddPlatformStorage<InMemoryObjectStorage>();
        services.AddSingleton<IMailService, StubMailService>();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        using var scope = provider.CreateScope();
        Assert.NotNull(provider.GetService<IRateLimiter>());
        Assert.NotNull(provider.GetService<IMailService>());
        Assert.NotNull(scope.ServiceProvider.GetService<IObjectStorage>());
    }

    private sealed class InMemoryObjectStorage : IObjectStorage
    {
        public Task<StorageOutcome> UploadAsync(StorageUploadRequest request, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public Task<StorageDownloadResult> DownloadAsync(StorageObjectKey key, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public Task<StorageObjectMetadata?> GetMetadataAsync(StorageObjectKey key, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public Task<StorageOutcome> DeleteAsync(StorageObjectKey key, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public Task<(StorageOutcome Outcome, PresignedOperation? Operation)> PresignAsync(PresignRequest request, CancellationToken cancellationToken = default) => throw new NotImplementedException();
    }

    private sealed class StubMailService : IMailService
    {
        public Task<MailSendResult> SendAsync(MailMessage message, CancellationToken cancellationToken = default) =>
            Task.FromResult(new MailSendResult(MailSendOutcome.Sent, "stub-id"));
    }
}
