namespace Platform.Core.Tenancy;

/// <summary>
/// Application-supplied tenant metadata. The platform never owns the
/// tenant entity; applications populate this contract from their catalog
/// or resolver.
/// </summary>
public interface ITenantInfo
{
    /// <summary>Gets the stable tenant identifier used in queries and filters.</summary>
    string Id { get; }

    /// <summary>Gets the optional human-readable name, used only for diagnostics.</summary>
    string? Name { get; }
}
