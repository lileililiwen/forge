using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Auditing.Contracts.Common;
using Platform.Core.Time;

namespace Platform.Auditing.Contracts.DependencyInjection;

/// <summary>Registration helpers for the platform auditing contracts.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the audit recorder, default masker, default sink, and bounded options.</summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    public static IServiceCollection AddPlatformAuditing(this IServiceCollection services)
        => services.AddPlatformAuditing(_ => { });

    /// <summary>Registers the audit pipeline and configures its options; applications replace the sink, masker, or enrichers before or after this call.</summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The options configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    public static IServiceCollection AddPlatformAuditing(this IServiceCollection services, Action<AuditOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddOptions<AuditOptions>()
            .Configure(configure)
            .Validate(o => o.Validate().Count == 0, "AuditOptions failed platform validation.");
        services.TryAddSingleton<IClock, SystemClock>();
        services.TryAddSingleton<IAuditMasker, DefaultAuditMasker>();
        services.TryAddSingleton<IAuditEnricher, NoOpAuditEnricher>();
        services.TryAddSingleton<IAuditSink, InMemoryAuditSink>();
        services.TryAddSingleton<IAuditDeadLetterSink, NoOpAuditDeadLetterSink>();
        services.TryAddSingleton<IAuditRetentionPolicy, RetainAllAuditRetentionPolicy>();
        services.TryAddSingleton<IAuditProviderStatusSource, DefaultAuditProviderStatusSource>();
        services.TryAddSingleton<IAuditRecorder, DefaultAuditRecorder>();
        return services;
    }
}
