using System.Security.Cryptography;
using System.Text;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>
/// HMAC-SHA256 signature verifier. The header value is expected to be either the raw hex-encoded
/// digest, or the <c>t=...,v1=...</c> envelope with a Unix timestamp. Timestamps outside the
/// configured tolerance are rejected with a <see cref="WebhookVerificationStatus.TimestampOutOfRange"/> decision.
/// </summary>
public sealed class HmacWebhookSignatureVerifier : IWebhookSignatureVerifier
{
    private readonly WebhookOptions _options;
    private readonly string _signatureHeader;
    private readonly string? _timestampHeader;

    /// <summary>Creates a verifier that reads the supplied signature and optional timestamp headers.</summary>
    public HmacWebhookSignatureVerifier(WebhookProviderId provider, string signatureHeader, string? timestampHeader, WebhookOptions options)
    {
        if (string.IsNullOrWhiteSpace(signatureHeader)) throw new ArgumentException("A signature header is required.", nameof(signatureHeader));
        if (string.IsNullOrWhiteSpace(provider.Value)) throw new ArgumentException("A provider identifier is required.", nameof(provider));
        Provider = provider; _signatureHeader = signatureHeader; _timestampHeader = timestampHeader;
        _options = options ?? throw new ArgumentNullException(nameof(options));
        _options.Validate();
    }
    /// <inheritdoc />
    public WebhookProviderId Provider { get; }
    /// <inheritdoc />
    public WebhookVerificationResult Verify(WebhookVerificationRequest request, byte[] secret)
    {
        ArgumentNullException.ThrowIfNull(request);
        if (secret is null || secret.Length == 0) throw new ArgumentException("A non-empty secret is required.", nameof(secret));
        if (!request.Headers.TryGetValue(_signatureHeader, out var headerValue) || string.IsNullOrWhiteSpace(headerValue)) return WebhookVerificationResult.FailureResult(WebhookVerificationStatus.SignatureInvalid, new WebhookFailure("webhook.signature_missing", "The signature header is required.", false));
        long? timestamp = null;
        if (_timestampHeader is not null)
        {
            if (!request.Headers.TryGetValue(_timestampHeader, out var rawTimestamp) || !long.TryParse(rawTimestamp, out var parsed)) return WebhookVerificationResult.FailureResult(WebhookVerificationStatus.HeaderMalformed, new WebhookFailure("webhook.timestamp_missing", "The timestamp header is required.", false));
            timestamp = parsed;
            var seconds = Math.Abs((DateTimeOffset.UtcNow - DateTimeOffset.FromUnixTimeSeconds(parsed)).TotalSeconds);
            if (seconds > _options.MaximumInboundClockSkewSeconds) return WebhookVerificationResult.FailureResult(WebhookVerificationStatus.TimestampOutOfRange, new WebhookFailure("webhook.timestamp_out_of_range", "The supplied timestamp is outside the tolerated skew window.", true));
        }
        if (!TryExtractSignature(headerValue, out var providedSignature)) return WebhookVerificationResult.FailureResult(WebhookVerificationStatus.SignatureInvalid, new WebhookFailure("webhook.signature_malformed", "The signature header could not be parsed.", false));
        var payload = timestamp is null ? request.Body : ApplyTimestamp(timestamp.Value, request.Body);
        using var hmac = new HMACSHA256(secret);
        var expected = hmac.ComputeHash(payload.ToArray());
        if (!FixedTimeEquals(expected, providedSignature)) return WebhookVerificationResult.FailureResult(WebhookVerificationStatus.SignatureInvalid, new WebhookFailure("webhook.signature_invalid", "The supplied signature did not match the expected value.", false));
        return WebhookVerificationResult.Verified();
    }

    private static byte[] ApplyTimestamp(long timestamp, ReadOnlyMemory<byte> body)
    {
        var prefix = Encoding.UTF8.GetBytes(timestamp + ".");
        var buffer = new byte[prefix.Length + body.Length];
        Buffer.BlockCopy(prefix, 0, buffer, 0, prefix.Length);
        body.Span.CopyTo(buffer.AsSpan(prefix.Length));
        return buffer;
    }

    private static bool TryExtractSignature(string header, out byte[] signature)
    {
        signature = Array.Empty<byte>();
        if (string.IsNullOrWhiteSpace(header)) return false;
        var span = header.AsSpan().Trim();
        if (span.Length > 2 && span[0] == 'v' && span[1] == '1' && span[2] == '=') { span = span[3..]; }
        else if (span.Contains(','))
        {
            foreach (var part in span.ToString().Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries))
            {
                if (part.StartsWith("v1=", StringComparison.Ordinal)) { span = part.AsSpan(3); break; }
                if (part.StartsWith("t=", StringComparison.Ordinal)) continue;
            }
        }
        if (span.IsEmpty) return false;
        return TryDecodeHex(span, out signature);
    }

    private static bool TryDecodeHex(ReadOnlySpan<char> source, out byte[] bytes)
    {
        bytes = Array.Empty<byte>();
        if ((source.Length & 1) != 0) return false;
        var buffer = new byte[source.Length / 2];
        for (var i = 0; i < buffer.Length; i++)
        {
            if (!TryDecodeNibble(source[i * 2], out var high) || !TryDecodeNibble(source[i * 2 + 1], out var low)) return false;
            buffer[i] = (byte)((high << 4) | low);
        }
        bytes = buffer;
        return true;
    }

    private static bool TryDecodeNibble(char c, out int value)
    {
        if (c >= '0' && c <= '9') { value = c - '0'; return true; }
        if (c >= 'a' && c <= 'f') { value = c - 'a' + 10; return true; }
        if (c >= 'A' && c <= 'F') { value = c - 'A' + 10; return true; }
        value = 0; return false;
    }

    private static bool FixedTimeEquals(ReadOnlySpan<byte> expected, ReadOnlySpan<byte> provided)
    {
        if (expected.Length != provided.Length) return false;
        return CryptographicOperations.FixedTimeEquals(expected, provided);
    }
}
