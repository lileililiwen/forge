namespace Platform.Persistence.Multitenancy;

/// <summary>Bounded opt-in configuration for the multitenancy adapter.</summary>
public sealed class MultitenancyOptions
{
    /// <summary>Maximum number of characters accepted in a tenant identifier.</summary>
    public const int DefaultMaxTenantIdLength = 128;

    /// <summary>Enables default-on tenant isolation for opted-in entities.</summary>
    public bool EnableDefaultTenantIsolation { get; set; }

    /// <summary>Enables the ASP.NET Core middleware that installs tenant scopes from HTTP requests.</summary>
    public bool EnableHttpScopeInstallation { get; set; } = true;

    /// <summary>Maximum length of a tenant identifier accepted by the ambient scope.</summary>
    public int MaxTenantIdLength { get; set; } = DefaultMaxTenantIdLength;

    /// <summary>Header that the HTTP middleware inspects to obtain a tenant identifier.</summary>
    public string TenantHeader { get; set; } = "X-Tenant-Id";

    /// <summary>Claim type the HTTP middleware inspects when no header is present.</summary>
    public string TenantClaim { get; set; } = "tenant_id";

    /// <summary>When <c>true</c>, the adapter rejects any tenant-scoped operation that runs without a resolved scope.</summary>
    public bool FailClosedOnMissingScope { get; set; } = true;

    /// <summary>Validates the configured option values and returns human-readable failures.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (MaxTenantIdLength is < 1 or > 1024)
            errors.Add("MaxTenantIdLength must be between 1 and 1024.");
        if (string.IsNullOrWhiteSpace(TenantHeader))
            errors.Add("TenantHeader is required.");
        if (string.IsNullOrWhiteSpace(TenantClaim))
            errors.Add("TenantClaim is required.");
        return errors;
    }
}
