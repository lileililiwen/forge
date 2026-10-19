using Microsoft.Extensions.Options;

namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>
/// Validates the adapter options on startup. A missing or unsafe issuer, client,
/// callback, scope allowlist or secret reference refuses the scheme instead of
/// starting an insecure one.
/// </summary>
public sealed class PlatformOpenIdConnectOptionsValidator : IValidateOptions<PlatformOpenIdConnectOptions>
{
    private readonly IPlatformOpenIdConnectSecretResolver _secretResolver;

    /// <summary>Creates the validator with the application's secret resolver.</summary>
    public PlatformOpenIdConnectOptionsValidator(IPlatformOpenIdConnectSecretResolver secretResolver)
    {
        _secretResolver = secretResolver ?? throw new ArgumentNullException(nameof(secretResolver));
    }

    /// <inheritdoc />
    public ValidateOptionsResult Validate(string? name, PlatformOpenIdConnectOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);

        var failures = new List<string>();

        ValidateIssuer(options, failures);
        ValidateClient(options, failures);
        ValidateCallback(options, failures);
        ValidateScopes(options, failures);
        ValidateTiming(options, failures);
        ValidateRoleClaimType(options, failures);

        return failures.Count == 0
            ? ValidateOptionsResult.Success
            : ValidateOptionsResult.Fail(failures);
    }

    private static void ValidateIssuer(PlatformOpenIdConnectOptions options, List<string> failures)
    {
        if (string.IsNullOrWhiteSpace(options.Issuer))
        {
            failures.Add("Issuer is required.");
            return;
        }

        if (options.Issuer.Contains('{') || options.Issuer.Contains('}'))
        {
            failures.Add("Issuer must be a concrete absolute URI without placeholders.");
            return;
        }

        if (!Uri.TryCreate(options.Issuer, UriKind.Absolute, out var issuer))
        {
            failures.Add("Issuer must be an absolute URI.");
            return;
        }

        if (!string.IsNullOrEmpty(issuer.Query) || !string.IsNullOrEmpty(issuer.Fragment))
        {
            failures.Add("Issuer must not carry a query string or fragment.");
            return;
        }

        if (!string.IsNullOrEmpty(issuer.UserInfo))
        {
            failures.Add("Issuer must not carry user information.");
            return;
        }

        if (issuer.Scheme == Uri.UriSchemeHttps)
        {
            return;
        }

        if (issuer.Scheme != Uri.UriSchemeHttp)
        {
            failures.Add("Issuer must use the HTTPS scheme.");
            return;
        }

        if (!options.AllowInsecureHttpIssuer)
        {
            failures.Add("Issuer must use HTTPS. Plain HTTP is allowed only for an explicit local development issuer.");
            return;
        }

        if (!IsLoopback(issuer))
        {
            failures.Add("A plain HTTP issuer is allowed only for a loopback host in local development.");
        }
    }

    private void ValidateClient(PlatformOpenIdConnectOptions options, List<string> failures)
    {
        if (string.IsNullOrWhiteSpace(options.ClientId))
        {
            failures.Add("ClientId is required.");
        }
        else if (options.ClientId.Any(char.IsWhiteSpace))
        {
            failures.Add("ClientId must not contain whitespace.");
        }

        if (string.IsNullOrWhiteSpace(options.SignInScheme))
        {
            failures.Add("SignInScheme is required.");
        }

        if (options.ClientAuthentication != PlatformOpenIdConnectClientAuthentication.ClientSecret)
        {
            return;
        }

        if (string.IsNullOrWhiteSpace(options.ClientSecretReference))
        {
            failures.Add("ClientSecretReference is required for a confidential client.");
            return;
        }

        if (_secretResolver.Resolve(options.ClientSecretReference) is null)
        {
            failures.Add("ClientSecretReference could not be resolved.");
        }
    }

    private static void ValidateCallback(PlatformOpenIdConnectOptions options, List<string> failures)
    {
        if (string.IsNullOrWhiteSpace(options.CallbackPath))
        {
            failures.Add("CallbackPath is required.");
            return;
        }

        if (!options.CallbackPath.StartsWith('/'))
        {
            failures.Add("CallbackPath must be an absolute path.");
            return;
        }

        if (options.CallbackPath.Contains('?') || options.CallbackPath.Contains('#'))
        {
            failures.Add("CallbackPath must not contain a query string or fragment.");
            return;
        }

        if (options.CallbackPath.Contains("//"))
        {
            failures.Add("CallbackPath must not contain an empty segment.");
        }
    }

    private static void ValidateScopes(PlatformOpenIdConnectOptions options, List<string> failures)
    {
        if (options.Scopes.Count == 0)
        {
            failures.Add("Scopes must contain an explicit allowlist that includes openid.");
            return;
        }

        if (!options.Scopes.Contains("openid", StringComparer.Ordinal))
        {
            failures.Add("Scopes must include openid.");
        }

        if (options.Scopes.Any(scope => string.IsNullOrWhiteSpace(scope) || scope.Any(char.IsWhiteSpace)))
        {
            failures.Add("Scopes must not contain an empty scope or whitespace.");
        }

        if (options.Scopes.Distinct(StringComparer.Ordinal).Count() != options.Scopes.Count)
        {
            failures.Add("Scopes must not repeat a scope.");
        }
    }

    private static void ValidateTiming(PlatformOpenIdConnectOptions options, List<string> failures)
    {
        if (options.ClockSkew < TimeSpan.Zero || options.ClockSkew > TimeSpan.FromMinutes(5))
        {
            failures.Add("ClockSkew must be between zero and five minutes.");
        }

        if (options.StateLifetime <= TimeSpan.Zero || options.StateLifetime > TimeSpan.FromMinutes(30))
        {
            failures.Add("StateLifetime must be between one second and thirty minutes.");
        }
    }

    private static void ValidateRoleClaimType(PlatformOpenIdConnectOptions options, List<string> failures)
    {
        if (options.RoleClaimType is not null && string.IsNullOrWhiteSpace(options.RoleClaimType))
        {
            failures.Add("RoleClaimType must be null or a non-empty claim type.");
        }
    }

    private static bool IsLoopback(Uri issuer) =>
        string.Equals(issuer.Host, "localhost", StringComparison.OrdinalIgnoreCase)
        || issuer.IsLoopback;
}
