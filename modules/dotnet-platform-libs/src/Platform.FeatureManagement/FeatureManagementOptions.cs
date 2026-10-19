using Platform.Web.Telemetry;

namespace Platform.FeatureManagement;

/// <summary>Configures the platform feature-management integration.</summary>
/// <remarks>All values are bounded. The platform never reads feature flag state itself; the host owns the <c>FeatureManagement</c> configuration section and the rollout rules.</remarks>
public sealed class FeatureManagementOptions : IValidatablePlatformOptions
{
    /// <summary>The default configuration section read by <c>AddFeatureManagement</c>.</summary>
    public const string DefaultSectionName = "FeatureManagement";

    /// <summary>The default HTTP status returned when a gated endpoint's feature is disabled.</summary>
    public const int DefaultDisabledStatusCode = 404;

    /// <summary>Gets or sets whether feature evaluation and endpoint gating are enabled. Default: <c>true</c>.</summary>
    public bool Enabled { get; set; } = true;

    /// <summary>Gets or sets the configuration section containing feature definitions. Default: <c>FeatureManagement</c>.</summary>
    public string SectionName { get; set; } = DefaultSectionName;

    /// <summary>Gets or sets the HTTP status returned when a gated endpoint is disabled. Default: <c>404</c>.</summary>
    public int DisabledStatusCode { get; set; } = DefaultDisabledStatusCode;

    /// <summary>Gets or sets the safe problem title returned when a gated endpoint is disabled. Never carries rollout or flag state.</summary>
    public string DisabledTitle { get; set; } = "Feature disabled";

    /// <inheritdoc/>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (string.IsNullOrWhiteSpace(SectionName))
            errors.Add("SectionName must be a non-empty configuration section name.");
        if (DisabledStatusCode is < 100 or > 599)
            errors.Add("DisabledStatusCode must be a valid HTTP status code (100-599).");
        if (string.IsNullOrWhiteSpace(DisabledTitle))
            errors.Add("DisabledTitle must be a non-empty problem title.");
        return errors;
    }
}
