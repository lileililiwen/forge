namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>The classified reason an OpenID Connect sign-in failed.</summary>
public enum PlatformOpenIdConnectFailureKind
{
    /// <summary>The adapter configuration is missing or unsafe.</summary>
    InvalidConfiguration = 0,

    /// <summary>The provider response, token, state or nonce was rejected.</summary>
    InvalidResponse = 1,

    /// <summary>The provider refused the request or the user denied the request.</summary>
    AccessDenied = 2,

    /// <summary>The provider could not be reached or returned a temporary failure.</summary>
    ProviderUnavailable = 3,

    /// <summary>The application refused to map the validated external identity.</summary>
    ApplicationMapping = 4,
}

/// <summary>
/// A redacted OpenID Connect failure. The message never carries a code, token, secret,
/// or raw claim value.
/// </summary>
public sealed class PlatformOpenIdConnectException : Exception
{
    /// <summary>Creates a failure with a classified kind and a stable reason code.</summary>
    public PlatformOpenIdConnectException(PlatformOpenIdConnectFailureKind kind, string reason, string message)
        : base(message)
    {
        Kind = kind;
        Reason = reason;
    }

    /// <summary>The classified failure kind.</summary>
    public PlatformOpenIdConnectFailureKind Kind { get; }

    /// <summary>A stable machine-readable reason code.</summary>
    public string Reason { get; }
}
