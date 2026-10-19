namespace Platform.FeatureManagement;

/// <summary>Carries the application-provided context used while evaluating feature flags.</summary>
/// <remarks>The platform never populates this; the host supplies tenant, subject, and custom values through an <see cref="IFeatureContextResolver"/>.</remarks>
/// <param name="TenantId">The current tenant identifier, or <c>null</c> when not tenant-scoped.</param>
/// <param name="Subject">The current subject/user identifier, or <c>null</c> when unavailable.</param>
/// <param name="Properties">Optional bounded custom evaluation values. Never contains secrets or rollout state.</param>
public sealed record FeatureContext(
    string? TenantId = null,
    string? Subject = null,
    IReadOnlyDictionary<string, string>? Properties = null);
