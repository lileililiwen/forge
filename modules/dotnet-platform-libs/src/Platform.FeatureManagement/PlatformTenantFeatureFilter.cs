using Microsoft.FeatureManagement;
using Microsoft.Extensions.Configuration;

namespace Platform.FeatureManagement;

/// <summary>Feature filter that enables a feature for a bounded set of tenants supplied by the application.</summary>
/// <remarks>Reads the tenant identifier from the application-provided <see cref="IFeatureContextResolver"/> and compares it against the <c>AllowedTenants</c> parameter configured per feature. The platform owns no tenant list; the host supplies both the context and the configuration.</remarks>
[FilterAlias("PlatformTenant")]
public sealed class PlatformTenantFeatureFilter(IFeatureContextResolver contextResolver) : IFeatureFilter
{
    /// <inheritdoc/>
    public async Task<bool> EvaluateAsync(FeatureFilterEvaluationContext context)
    {
        ArgumentNullException.ThrowIfNull(context);

        var featureContext = await contextResolver.ResolveAsync().ConfigureAwait(false);
        if (string.IsNullOrWhiteSpace(featureContext.TenantId))
            return false;

        var allowedTenants = context.Parameters.GetSection("AllowedTenants").Get<string[]>() ?? [];
        return allowedTenants.Contains(featureContext.TenantId, StringComparer.OrdinalIgnoreCase);
    }
}
