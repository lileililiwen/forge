using Platform.Core.Results;

namespace Platform.Domain;

/// <summary>
/// Stable error codes for domain failures that have no direct
/// <see cref="Platform.Core.Results.Error"/> factory.
/// </summary>
public static class DomainErrorCodes
{
    /// <summary>
    /// Stable code used when a domain operation conflicts with current state.
    /// </summary>
    public const string Conflict = "domain.conflict";
}
