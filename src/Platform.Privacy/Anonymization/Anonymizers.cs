using System.Security.Cryptography;
using System.Text;

namespace Platform.Privacy.Anonymization;

/// <summary>Deterministic, keyless anonymization helpers.</summary>
public static class Anonymizers
{
    /// <summary>Masks an email local part, keeping domain and first/last characters.</summary>
    public static string MaskEmail(string email)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(email);
        var at = email.IndexOf('@');
        if (at <= 0 || at == email.Length - 1)
        {
            throw new ArgumentException("Not an email address.", nameof(email));
        }

        var local = email[..at];
        var masked = local.Length == 1 ? "*" : local[0] + new string('*', local.Length - 2) + local[^1];
        return masked + email[at..];
    }

    /// <summary>Stable HMAC-SHA256 pseudonym, hex-encoded. The key is application-owned per call.</summary>
    public static string StablePseudonym(string value, byte[] key)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(value);
        ArgumentNullException.ThrowIfNull(key);
        if (key.Length == 0)
        {
            throw new ArgumentException("Key must not be empty.", nameof(key));
        }

        return Convert.ToHexString(HMACSHA256.HashData(key, Encoding.UTF8.GetBytes(value))).ToLowerInvariant();
    }
}
