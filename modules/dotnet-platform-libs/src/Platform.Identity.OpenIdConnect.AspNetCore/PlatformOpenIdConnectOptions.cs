using Microsoft.AspNetCore.Authentication.OpenIdConnect;

namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>How the adapter authenticates itself against the provider's token endpoint.</summary>
public enum PlatformOpenIdConnectClientAuthentication
{
    /// <summary>Public client: no client secret, PKCE only.</summary>
    None = 0,

    /// <summary>Confidential client: a client secret resolved from a secret reference.</summary>
    ClientSecret = 1,
}

/// <summary>How the adapter treats the application's external identity link.</summary>
public enum PlatformOpenIdConnectIdentityResolutionMode
{
    /// <summary>
    /// The adapter returns the validated external principal and the application resolves
    /// the local user itself. The adapter never creates, links or merges an identity.
    /// </summary>
    Disabled = 0,

    /// <summary>
    /// The adapter calls the application's registered identity resolver and fails the
    /// sign-in when the application denies or reports a conflict.
    /// </summary>
    Required = 1,
}

/// <summary>Options for the opt-in ASP.NET Core OpenID Connect adapter.</summary>
public sealed class PlatformOpenIdConnectOptions
{
    /// <summary>The default authentication scheme name registered by the adapter.</summary>
    public const string DefaultSchemeName = "PlatformOidc";

    /// <summary>The default callback path registered by the adapter.</summary>
    public const string DefaultCallbackPath = "/signin-oidc";

    /// <summary>
    /// The provider's issuer identifier. Must be an absolute HTTPS URI that equals the
    /// issuer in the provider's discovery document. No implicit fallback to another
    /// issuer is performed.
    /// </summary>
    public string Issuer { get; set; } = string.Empty;

    /// <summary>The OAuth client identifier registered with the provider.</summary>
    public string ClientId { get; set; } = string.Empty;

    /// <summary>The absolute path the provider redirects back to after sign-in.</summary>
    public string CallbackPath { get; set; } = DefaultCallbackPath;

    /// <summary>The application's own sign-in scheme that receives the validated principal.</summary>
    public string SignInScheme { get; set; } = string.Empty;

    /// <summary>
    /// The explicit scope allowlist. It must contain <c>openid</c>; the adapter never
    /// adds scopes on the application's behalf.
    /// </summary>
    public IReadOnlyList<string> Scopes { get; set; } = Array.Empty<string>();

    /// <summary>How the adapter authenticates itself against the provider.</summary>
    public PlatformOpenIdConnectClientAuthentication ClientAuthentication { get; set; }
        = PlatformOpenIdConnectClientAuthentication.None;

    /// <summary>
    /// A reference to the client secret, never the secret itself. The reference format is
    /// <c>config:Section:Key</c> for application configuration or <c>env:VARIABLE_NAME</c>
    /// for an environment variable.
    /// </summary>
    public string? ClientSecretReference { get; set; }

    /// <summary>
    /// Allows a plain HTTP issuer. This is a development-only escape hatch: it is accepted
    /// only for a loopback issuer and never for a remote host.
    /// </summary>
    public bool AllowInsecureHttpIssuer { get; set; }

    /// <summary>
    /// The provider claim type that the application explicitly maps to local roles. When
    /// this is not set, provider role claims grant no local permission.
    /// </summary>
    public string? RoleClaimType { get; set; }

    /// <summary>The allowed clock skew when validating token time bounds.</summary>
    public TimeSpan ClockSkew { get; set; } = TimeSpan.FromMinutes(1);

    /// <summary>How long a consumed callback state is remembered for replay rejection.</summary>
    public TimeSpan StateLifetime { get; set; } = TimeSpan.FromMinutes(10);

    /// <summary>How the adapter treats the application's external identity link.</summary>
    public PlatformOpenIdConnectIdentityResolutionMode IdentityResolution { get; set; }
        = PlatformOpenIdConnectIdentityResolutionMode.Disabled;

    /// <summary>The authentication scheme name the adapter registers.</summary>
    public string SchemeName { get; set; } = DefaultSchemeName;

    /// <summary>
    /// An optional handler for the provider's discovery, key and token calls. An
    /// application uses this for a proxy, a pinned certificate or a test issuer.
    /// </summary>
    public HttpMessageHandler? BackchannelHttpHandler { get; set; }

    /// <summary>
    /// An optional application hook for the adapter's OIDC events. The adapter's own
    /// validation always runs first, so an application can observe events but cannot
    /// skip the adapter's guarantees.
    /// </summary>
    public Action<OpenIdConnectEvents>? ConfigureEvents { get; set; }
}
