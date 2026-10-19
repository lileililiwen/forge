using Amazon.S3;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Storage.Contracts;

namespace Platform.Storage.S3.DependencyInjection;

/// <summary>Registration helpers for the optional S3 adapter.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers an application-owned S3 client and storage options.</summary>
    public static IServiceCollection AddPlatformStorageS3(this IServiceCollection services, IAmazonS3 client, S3StorageOptions options)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(client);
        ArgumentNullException.ThrowIfNull(options);
        services.TryAddSingleton(client);
        services.AddSingleton(options);
        services.AddSingleton<S3Storage>();
        services.AddSingleton<IObjectStorage>(sp => sp.GetRequiredService<S3Storage>());
        services.AddSingleton<IStorageProviderStatus>(sp => sp.GetRequiredService<S3Storage>());
        return services;
    }
}
