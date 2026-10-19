using System.Globalization;
using System.Security.Cryptography;
using System.Text;

namespace Platform.Idempotency;

/// <summary>
/// Documented request-fingerprint helper. The same inputs always
/// produce the same fingerprint across processes because the helper
/// is deterministic and process-agnostic.
/// </summary>
public static class RequestFingerprint
{
    /// <summary>
    /// Computes the documented fingerprint for a request identified
    /// by the supplied <paramref name="method"/>,
    /// <paramref name="route"/>, and <paramref name="bodyHash"/>.
    /// </summary>
    /// <param name="method">The HTTP method (uppercased before hashing).</param>
    /// <param name="route">The route template (e.g. <c>/v1/orders</c>).</param>
    /// <param name="bodyHash">The hex-encoded body hash (e.g. SHA-256 of the body bytes).</param>
    /// <returns>The stable, hex-encoded fingerprint string.</returns>
    /// <exception cref="ArgumentException">A required argument is null, empty, or whitespace.</exception>
    public static string Compute(string method, string route, string bodyHash)
    {
        if (string.IsNullOrWhiteSpace(method))
        {
            throw new ArgumentException("Method must be a non-empty string.", nameof(method));
        }
        if (string.IsNullOrWhiteSpace(route))
        {
            throw new ArgumentException("Route must be a non-empty string.", nameof(route));
        }
        if (string.IsNullOrWhiteSpace(bodyHash))
        {
            throw new ArgumentException("Body hash must be a non-empty string.", nameof(bodyHash));
        }

        var normalised = string.Create(
            method.Length + 1 + route.Length + 1 + bodyHash.Length,
            (method, route, bodyHash),
            static (span, state) =>
            {
                var (m, r, h) = state;
                m.AsSpan().ToUpperInvariant(span);
                span[m.Length] = '|';
                r.AsSpan().CopyTo(span[(m.Length + 1)..]);
                span[m.Length + 1 + r.Length] = '|';
                h.AsSpan().CopyTo(span[(m.Length + 1 + r.Length + 1)..]);
            });

        Span<byte> hash = stackalloc byte[32];
        SHA256.HashData(Encoding.UTF8.GetBytes(normalised), hash);
        return Convert.ToHexString(hash).ToLowerInvariant();
    }

    /// <summary>
    /// Computes the SHA-256 hash of the supplied <paramref name="body"/>
    /// bytes and returns it as a lower-case hex string.
    /// </summary>
    /// <param name="body">The body bytes.</param>
    /// <returns>The hex-encoded SHA-256 hash.</returns>
    public static string ComputeBodyHash(ReadOnlySpan<byte> body)
    {
        Span<byte> hash = stackalloc byte[32];
        SHA256.HashData(body, hash);
        return Convert.ToHexString(hash).ToLowerInvariant();
    }

    /// <summary>
    /// Computes the SHA-256 hash of the supplied
    /// <paramref name="body"/> string and returns it as a lower-case
    /// hex string.
    /// </summary>
    /// <param name="body">The body string.</param>
    /// <returns>The hex-encoded SHA-256 hash.</returns>
    public static string ComputeBodyHash(string body)
    {
        ArgumentNullException.ThrowIfNull(body);
        return ComputeBodyHash(Encoding.UTF8.GetBytes(body));
    }
}
