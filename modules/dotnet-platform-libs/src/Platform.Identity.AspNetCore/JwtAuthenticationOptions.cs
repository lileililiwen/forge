using Microsoft.Extensions.Options;

namespace Platform.Identity.AspNetCore;

/// <summary>Application-selected JWT authentication configuration. The platform validates presence only; it does not issue tokens.</summary>
public sealed class PlatformIdentityJwtOptions
{
    /// <summary>Gets or sets whether JWT authentication configuration is selected.</summary>
    public bool Enabled { get; set; }
    /// <summary>Gets or sets the expected token issuer.</summary>
    public string? Issuer { get; set; }
    /// <summary>Gets or sets the expected token audience.</summary>
    public string? Audience { get; set; }
    /// <summary>Gets or sets the application-provided signing key. Treated as a secret and never written to diagnostics.</summary>
    public string? SigningKey { get; set; }

    /// <summary>Returns a secret-free diagnostic view suitable for logs and health surfaces.</summary>
    public string GetDiagnosticName()
    {
        if (!Enabled)
            return "jwt(disabled)";
        return $"jwt(issuer={Mask(Issuer)}, audience={Mask(Audience)}, signingKey={(string.IsNullOrWhiteSpace(SigningKey) ? "unset" : "set")})";
    }

    private static string Mask(string? value) => string.IsNullOrWhiteSpace(value) ? "unset" : "set";
}

internal sealed class PlatformIdentityJwtOptionsValidator : IValidateOptions<PlatformIdentityJwtOptions>
{
    public ValidateOptionsResult Validate(string? name, PlatformIdentityJwtOptions options)
    {
        if (!options.Enabled)
            return ValidateOptionsResult.Success;

        var failures = new List<string>(3);
        // Messages intentionally exclude the secret value.
        if (string.IsNullOrWhiteSpace(options.SigningKey))
            failures.Add("JWT signing key is required when JWT authentication is enabled.");
        if (string.IsNullOrWhiteSpace(options.Issuer))
            failures.Add("JWT issuer is required when JWT authentication is enabled.");
        if (string.IsNullOrWhiteSpace(options.Audience))
            failures.Add("JWT audience is required when JWT authentication is enabled.");

        return failures.Count == 0 ? ValidateOptionsResult.Success : ValidateOptionsResult.Fail(failures);
    }
}
