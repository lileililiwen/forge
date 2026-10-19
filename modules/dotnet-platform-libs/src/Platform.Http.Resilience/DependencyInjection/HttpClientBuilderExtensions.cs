using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Http;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.Http.Resilience.DependencyInjection;

/// <summary>Registration helpers for the platform HTTP resilience package.</summary>
public static class HttpClientBuilderExtensions
{
    /// <summary>Adds the platform resilience pipeline (retry, timeout, circuit breaker, concurrency) to the named <see cref="HttpClient"/> registration with default options.</summary>
    /// <param name="builder">The HTTP client builder.</param>
    /// <returns>The same builder for chaining.</returns>
    public static IHttpClientBuilder AddPlatformHttpResilience(this IHttpClientBuilder builder) =>
        builder.AddPlatformHttpResilience(_ => { });

    /// <summary>Adds the platform resilience pipeline to the named <see cref="HttpClient"/> registration with the supplied bounded options.</summary>
    /// <param name="builder">The HTTP client builder.</param>
    /// <param name="configure">The resilience options configuration delegate.</param>
    /// <returns>The same builder for chaining.</returns>
    public static IHttpClientBuilder AddPlatformHttpResilience(this IHttpClientBuilder builder, Action<PlatformHttpResilienceOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(builder);
        ArgumentNullException.ThrowIfNull(configure);

        builder.Services.AddPlatformWebTelemetry();
        builder.Services.AddValidatedOptions(configure);
        builder.Services.TryAddSingleton<IPlatformHttpResilienceTelemetry, DefaultPlatformHttpResilienceTelemetry>();
        builder.Services.TryAddTransient<PlatformHttpResilienceHandler>();

        return builder.AddHttpMessageHandler<PlatformHttpResilienceHandler>();
    }
}
