using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Microsoft.FeatureManagement;

namespace Platform.FeatureManagement;

/// <summary>Endpoint filter that gates access behind a feature flag using an application-owned <see cref="IFeatureManager"/>.</summary>
/// <remarks>When the feature is disabled the filter returns a safe, consistent response built from <see cref="FeatureManagementOptions"/> without exposing flag or rollout state.</remarks>
public sealed class FeatureGateEndpointFilter(string featureName) : IEndpointFilter
{
    /// <inheritdoc/>
    public async ValueTask<object?> InvokeAsync(EndpointFilterInvocationContext context, EndpointFilterDelegate next)
    {
        ArgumentNullException.ThrowIfNull(context);
        ArgumentNullException.ThrowIfNull(next);

        var featureManager = context.HttpContext.RequestServices.GetRequiredService<IFeatureManager>();
        if (await featureManager.IsEnabledAsync(featureName).ConfigureAwait(false))
            return await next(context).ConfigureAwait(false);

        var options = context.HttpContext.RequestServices.GetService<IOptions<FeatureManagementOptions>>()?.Value;
        var statusCode = options?.DisabledStatusCode ?? FeatureManagementOptions.DefaultDisabledStatusCode;
        var title = options?.DisabledTitle ?? "Feature disabled";
        return TypedResults.Problem(title: title, statusCode: statusCode);
    }
}

/// <summary>Registration helpers for endpoint feature gating.</summary>
public static class RequireFeatureExtensions
{
    /// <summary>Gates the endpoint behind a feature flag. Returns the configured safe response when the feature is disabled.</summary>
    /// <param name="builder">The endpoint route builder.</param>
    /// <param name="featureName">The application-owned feature name to evaluate.</param>
    /// <returns>The same builder for chaining.</returns>
    public static RouteHandlerBuilder RequireFeature(this RouteHandlerBuilder builder, string featureName)
    {
        ArgumentNullException.ThrowIfNull(builder);
        ArgumentNullException.ThrowIfNull(featureName);
        return builder.AddEndpointFilter(new FeatureGateEndpointFilter(featureName));
    }
}
