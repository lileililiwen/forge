using System.Security.Claims;
using Microsoft.AspNetCore.Authentication;
using Platform.Identity.Contracts;

namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>The validated external identity returned by the provider.</summary>
/// <param name="Issuer">The provider issuer that authenticated the subject.</param>
/// <param name="Subject">The provider's stable subject identifier for the user.</param>
public sealed record PlatformOpenIdConnectExternalIdentity(string Issuer, string Subject)
{
    /// <summary>Projects the external identity onto the shared identity contract.</summary>
    public ExternalIdentity ToExternalIdentity() => new(Issuer, Subject);
}

/// <summary>How the application resolved a validated external identity.</summary>
public enum PlatformOpenIdConnectIdentityStatus
{
    /// <summary>The application matched the external identity to one of its own local users.</summary>
    Authenticated = 0,

    /// <summary>The application refused the external identity, for example when enrollment is disabled.</summary>
    Denied = 1,

    /// <summary>The external identity conflicts with an existing local identity.</summary>
    Conflict = 2,
}

/// <summary>The application's answer to a validated external identity.</summary>
/// <param name="Status">The resolution outcome.</param>
/// <param name="LocalSubject">The application's own local user identifier, when authenticated.</param>
/// <param name="Reason">A stable machine-readable reason code.</param>
public sealed record PlatformOpenIdConnectIdentityResolution(
    PlatformOpenIdConnectIdentityStatus Status,
    string? LocalSubject,
    string Reason)
{
    /// <summary>Creates an authenticated resolution for a local user.</summary>
    public static PlatformOpenIdConnectIdentityResolution Authenticated(string localSubject) =>
        new(PlatformOpenIdConnectIdentityStatus.Authenticated, localSubject, "authenticated");

    /// <summary>Creates a denied resolution with a stable reason code.</summary>
    public static PlatformOpenIdConnectIdentityResolution Denied(string reason) =>
        new(PlatformOpenIdConnectIdentityStatus.Denied, null, reason);

    /// <summary>Creates a conflict resolution with a stable reason code.</summary>
    public static PlatformOpenIdConnectIdentityResolution Conflict(string reason) =>
        new(PlatformOpenIdConnectIdentityStatus.Conflict, null, reason);
}

/// <summary>
/// The application's own external identity seam. The adapter never creates, links or
/// merges a local identity: it only hands the validated issuer and subject to the
/// application and honours the application's answer.
/// </summary>
public interface IPlatformOpenIdConnectIdentityResolver
{
    /// <summary>Resolves one validated external identity against the application's own store.</summary>
    Task<PlatformOpenIdConnectIdentityResolution> ResolveAsync(
        PlatformOpenIdConnectExternalIdentity identity,
        ClaimsPrincipal principal,
        CancellationToken cancellationToken);
}

/// <summary>Reads the validated external identity from a principal or its ticket properties.</summary>
public static class PlatformOpenIdConnectPrincipal
{
    /// <summary>The claim type carrying the provider issuer.</summary>
    public const string IssuerClaimType = "iss";

    /// <summary>The claim type carrying the provider subject.</summary>
    public const string SubjectClaimType = "sub";

    /// <summary>The property key carrying the validated issuer.</summary>
    public const string IssuerPropertyKey = "platform.oidc.issuer";

    /// <summary>The property key carrying the validated subject.</summary>
    public const string SubjectPropertyKey = "platform.oidc.subject";

    /// <summary>
    /// Reads the issuer and subject from a validated principal. The subject is required;
    /// a principal without one is not a usable external identity.
    /// </summary>
    public static bool TryGetExternalIdentity(
        ClaimsPrincipal principal,
        out PlatformOpenIdConnectExternalIdentity? identity) =>
        TryGetExternalIdentity(principal, properties: null, out identity);

    /// <summary>
    /// Reads the validated external identity from the ticket properties, or from the
    /// principal's claims. The ASP.NET Core OpenID Connect handler removes the protocol
    /// claims such as <c>iss</c> from the signed-in principal, so the adapter stores the
    /// validated issuer and subject in the ticket properties.
    /// </summary>
    public static bool TryGetExternalIdentity(
        ClaimsPrincipal principal,
        AuthenticationProperties? properties,
        out PlatformOpenIdConnectExternalIdentity? identity)
    {
        ArgumentNullException.ThrowIfNull(principal);

        var issuer = ReadProperty(properties, IssuerPropertyKey)
            ?? principal.FindFirst(IssuerClaimType)?.Value;
        var subject = ReadProperty(properties, SubjectPropertyKey)
            ?? principal.FindFirst(SubjectClaimType)?.Value;

        if (string.IsNullOrWhiteSpace(issuer) || string.IsNullOrWhiteSpace(subject))
        {
            identity = null;
            return false;
        }

        identity = new PlatformOpenIdConnectExternalIdentity(issuer, subject);
        return true;
    }

    private static string? ReadProperty(AuthenticationProperties? properties, string key)
    {
        if (properties is null || !properties.Items.TryGetValue(key, out var value))
        {
            return null;
        }

        return string.IsNullOrWhiteSpace(value) ? null : value;
    }
}
