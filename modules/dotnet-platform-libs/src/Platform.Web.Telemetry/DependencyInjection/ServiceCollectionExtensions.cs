using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;

namespace Platform.Web.Telemetry.DependencyInjection;

/// <summary>Registration helpers for the platform web telemetry package.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the redaction-safe web telemetry sink with default options.</summary>
    public static IServiceCollection AddPlatformWebTelemetry(this IServiceCollection services) => services.AddPlatformWebTelemetry(_ => { });

    /// <summary>Registers the redaction-safe web telemetry sink and configures its options.</summary>
    public static IServiceCollection AddPlatformWebTelemetry(this IServiceCollection services, Action<PlatformWebTelemetryOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddValidatedOptions(configure);
        services.TryAddSingleton<IPlatformWebTelemetryRedactor, DefaultPlatformWebTelemetryRedactor>();
        services.TryAddSingleton<IPlatformWebTelemetry, DefaultPlatformWebTelemetry>();
        return services;
    }

    /// <summary>Registers a validated options instance using the platform's standard validation pipeline.</summary>
    /// <typeparam name="TOptions">The options type.</typeparam>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The configuration delegate.</param>
    /// <param name="errorMessage">The error message used when validation fails.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    public static IServiceCollection AddValidatedOptions<TOptions>(this IServiceCollection services, Action<TOptions> configure, string? errorMessage = null) where TOptions : class
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        var builder = services.AddOptions<TOptions>().Configure(configure);
        builder.Validate(options => PlatformOptionsValidator.Validate(options), errorMessage ?? $"{typeof(TOptions).Name} failed platform validation.");
        return services;
    }
}
