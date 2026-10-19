using System.Security.Claims;

namespace Platform.Identity.AspNetCore;

/// <summary>Explicit authentication scheme ownership for platform wiring.</summary>
public sealed class PlatformIdentityOptions
{
    /// <summary>Default scheme used by platform authentication registration.</summary>
    public string AuthenticationScheme { get; set; } = "PlatformIdentity";
    /// <summary>Claim type containing permission keys.</summary>
    public string PermissionClaimType { get; set; } = "permission";
    /// <summary>Claim type containing tenant identity.</summary>
    public string TenantClaimType { get; set; } = "tenant_id";
    /// <summary>Subject claim type. Falls back to the conventional <c>sub</c> short claim when absent.</summary>
    public string SubjectClaimType { get; set; } = ClaimTypes.NameIdentifier;
    /// <summary>Email claim type. Falls back to the conventional <c>email</c> short claim when absent.</summary>
    public string EmailClaimType { get; set; } = ClaimTypes.Email;
    /// <summary>Validates required options.</summary>
    public bool IsValid() => !string.IsNullOrWhiteSpace(AuthenticationScheme)
        && !string.IsNullOrWhiteSpace(PermissionClaimType)
        && !string.IsNullOrWhiteSpace(TenantClaimType)
        && !string.IsNullOrWhiteSpace(SubjectClaimType)
        && !string.IsNullOrWhiteSpace(EmailClaimType);
}
