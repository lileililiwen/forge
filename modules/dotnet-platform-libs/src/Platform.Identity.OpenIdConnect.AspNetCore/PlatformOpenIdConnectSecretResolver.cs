using Microsoft.Extensions.Configuration;

namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>
/// Resolves a client secret from a reference. The adapter never holds a secret value in
/// its options and never logs one.
/// </summary>
public interface IPlatformOpenIdConnectSecretResolver
{
    /// <summary>Resolves the secret behind a reference, or <c>null</c> when it is unknown.</summary>
    string? Resolve(string reference);
}

/// <summary>
/// The default resolver. It reads <c>config:Section:Key</c> references from the
/// application's configuration and <c>env:VARIABLE_NAME</c> references from the
/// environment.
/// </summary>
public sealed class PlatformOpenIdConnectSecretResolver : IPlatformOpenIdConnectSecretResolver
{
    private readonly IConfiguration? _configuration;

    /// <summary>Creates a resolver bound to the application's configuration, when present.</summary>
    public PlatformOpenIdConnectSecretResolver(IConfiguration? configuration = null)
    {
        _configuration = configuration;
    }

    /// <inheritdoc />
    public string? Resolve(string reference)
    {
        if (string.IsNullOrWhiteSpace(reference))
        {
            return null;
        }

        if (reference.StartsWith("config:", StringComparison.OrdinalIgnoreCase))
        {
            var key = reference["config:".Length..].Trim();
            if (key.Length == 0)
            {
                return null;
            }

            var value = _configuration?[key];
            return string.IsNullOrWhiteSpace(value) ? null : value;
        }

        if (reference.StartsWith("env:", StringComparison.OrdinalIgnoreCase))
        {
            var name = reference["env:".Length..].Trim();
            if (name.Length == 0)
            {
                return null;
            }

            var value = Environment.GetEnvironmentVariable(name);
            return string.IsNullOrWhiteSpace(value) ? null : value;
        }

        return null;
    }
}
