using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.DependencyInjection;
using Platform.Webhooks.Contracts.Inbound;
using Platform.Webhooks.Contracts.Outbound;
using Platform.Webhooks.Contracts.Security;
using Platform.Webhooks.AspNetCore.Outbound;

namespace Platform.Webhooks.AspNetCore.DependencyInjection;

/// <summary>Registration helpers for the optional ASP.NET Core webhook integration.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the default outbound HTTP sender and ensures webhook contracts are wired.</summary>
    public static IServiceCollection AddPlatformWebhooksAspNetCore(this IServiceCollection services, Action<WebhookOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);
        services.AddPlatformWebhooks(configure);
        services.AddHttpClient<IWebhookHttpSender, HttpClientWebhookSender>();
        return services;
    }
}

/// <summary>Maps the <c>/_webhooks/health</c> endpoint that reports the webhook status surface.</summary>
public static class WebhookEndpointRouteBuilderExtensions
{
    /// <summary>Maps a minimal status endpoint that returns the SSRF validator policy and the configured options.</summary>
    public static IEndpointRouteBuilder MapPlatformWebhookStatus(this IEndpointRouteBuilder endpoints, string pattern = "/_webhooks/health")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        endpoints.MapGet(pattern, ([FromServices] ISsrfTargetValidator validator, [FromServices] IWebhookBackendStatusProvider status) => Results.Ok(new
        {
            backend = status.GetStatus(),
            ssrf = new
            {
                allowsLoopback = SsrfTargetValidatorExtensions.IsLoopbackAllowedValidatorType(validator)
            }
        }));
        return endpoints;
    }
}

/// <summary>Internal accessor used by the status endpoint to surface the loopback policy.</summary>
internal static class SsrfTargetValidatorExtensions
{
    public static bool IsLoopbackAllowedValidatorType(this ISsrfTargetValidator validator) => validator is SsrfTargetValidator;
}
