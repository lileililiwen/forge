namespace Platform.Admin.AspNetCore;

/// <summary>Configures the opt-in administration endpoint surface.</summary>
public sealed class AdminOptions
{
    /// <summary>Maximum page size accepted by administrative queries.</summary>
    public int MaximumPageSize { get; set; } = 100;
    /// <summary>Endpoint route prefix.</summary>
    public string RoutePrefix { get; set; } = "/admin";
    /// <summary>Whether application-provided impersonation is enabled.</summary>
    public bool EnableImpersonation { get; set; }
    /// <summary>Maximum impersonation lifetime.</summary>
    public TimeSpan MaximumImpersonationLifetime { get; set; } = TimeSpan.FromMinutes(15);
    /// <summary>Validates safe option values.</summary>
    public bool IsValid() => MaximumPageSize is >= 1 and <= 1000
        && !string.IsNullOrWhiteSpace(RoutePrefix) && RoutePrefix.StartsWith('/')
        && MaximumImpersonationLifetime > TimeSpan.Zero && MaximumImpersonationLifetime <= TimeSpan.FromHours(24);
}
