using System.Globalization;

namespace Platform.Auditing.Contracts;

/// <summary>
/// Default masker that redacts values whose key name matches a known-sensitive pattern. The match
/// is case-insensitive and ignores common separators (``_``, ``-``, ``.``). Non-sensitive values
/// are returned unchanged.
/// </summary>
public sealed class DefaultAuditMasker : IAuditMasker
{
    /// <summary>The placeholder returned for redacted values.</summary>
    public const string Redacted = "[REDACTED]";

    private static readonly string[] SensitivePatterns =
    {
        "password", "passwd", "pwd", "secret", "token", "apikey", "api_key", "accesstoken",
        "accesstoken", "refreshtoken", "credential", "ssn", "socialsecurity", "cvv", "cardnumber",
        "card", "pin", "privatekey", "clientsecret", "authorization", "otp", "mfa", "biometric",
    };

    /// <inheritdoc />
    public string Mask(string key, string? value)
    {
        if (string.IsNullOrWhiteSpace(key)) return value ?? string.Empty;
        if (value is null) return string.Empty;
        return IsSensitive(key) ? Redacted : value;
    }

    private static bool IsSensitive(string key)
    {
        var normalized = key.Replace("_", string.Empty, StringComparison.Ordinal)
            .Replace("-", string.Empty, StringComparison.Ordinal)
            .Replace(".", string.Empty, StringComparison.Ordinal)
            .ToLowerInvariant();
        foreach (var pattern in SensitivePatterns)
        {
            if (normalized.Contains(pattern, StringComparison.Ordinal)) return true;
        }

        return false;
    }

    /// <summary>Converts a non-string value to an invariant culture string for masking.</summary>
    /// <param name="value">The value to convert.</param>
    /// <returns>The invariant culture string representation.</returns>
    public static string ToStringValue(object? value) => value switch
    {
        null => string.Empty,
        string s => s,
        _ => Convert.ToString(value, CultureInfo.InvariantCulture) ?? string.Empty,
    };
}
