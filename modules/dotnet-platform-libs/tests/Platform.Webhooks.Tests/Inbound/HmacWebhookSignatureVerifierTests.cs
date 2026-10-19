using System.Text;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;

namespace Platform.Webhooks.Tests.Inbound;

public sealed class HmacWebhookSignatureVerifierTests
{
    [Fact]
    public void Verifies_valid_signature_with_timestamp()
    {
        var provider = new WebhookProviderId("test");
        var options = new WebhookOptions();
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", options);
        var secret = Encoding.UTF8.GetBytes("super-secret");
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString();
        var body = Encoding.UTF8.GetBytes("{\"id\":\"evt_1\"}");
        var prefix = Encoding.UTF8.GetBytes(timestamp + ".");
        var combined = new byte[prefix.Length + body.Length];
        Buffer.BlockCopy(prefix, 0, combined, 0, prefix.Length);
        body.CopyTo(combined, prefix.Length);
        using var hmac = new System.Security.Cryptography.HMACSHA256(secret);
        var digest = Convert.ToHexString(hmac.ComputeHash(combined)).ToLowerInvariant();
        var request = new WebhookVerificationRequest(provider, "evt_1", new Dictionary<string, string>
        {
            ["X-Signature"] = digest,
            ["X-Timestamp"] = timestamp,
        }, body);
        var result = verifier.Verify(request, secret);
        Assert.True(result.IsVerified);
    }

    [Fact]
    public void Rejects_signature_with_wrong_secret()
    {
        var provider = new WebhookProviderId("test");
        var options = new WebhookOptions();
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", options);
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString();
        var body = Encoding.UTF8.GetBytes("{\"id\":\"evt_1\"}");
        var prefix = Encoding.UTF8.GetBytes(timestamp + ".");
        var combined = new byte[prefix.Length + body.Length];
        Buffer.BlockCopy(prefix, 0, combined, 0, prefix.Length);
        body.CopyTo(combined, prefix.Length);
        using var hmac = new System.Security.Cryptography.HMACSHA256(Encoding.UTF8.GetBytes("other"));
        var digest = Convert.ToHexString(hmac.ComputeHash(combined)).ToLowerInvariant();
        var request = new WebhookVerificationRequest(provider, "evt_1", new Dictionary<string, string>
        {
            ["X-Signature"] = digest,
            ["X-Timestamp"] = timestamp,
        }, body);
        var result = verifier.Verify(request, Encoding.UTF8.GetBytes("super-secret"));
        Assert.False(result.IsVerified);
        Assert.Equal(WebhookVerificationStatus.SignatureInvalid, result.Status);
    }

    [Fact]
    public void Rejects_timestamp_outside_skew_window()
    {
        var provider = new WebhookProviderId("test");
        var options = new WebhookOptions { MaximumInboundClockSkewSeconds = 60 };
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", options);
        var secret = Encoding.UTF8.GetBytes("super-secret");
        var staleTimestamp = DateTimeOffset.UtcNow.AddHours(-1).ToUnixTimeSeconds().ToString();
        var body = Encoding.UTF8.GetBytes("{}");
        var prefix = Encoding.UTF8.GetBytes(staleTimestamp + ".");
        var combined = new byte[prefix.Length + body.Length];
        Buffer.BlockCopy(prefix, 0, combined, 0, prefix.Length);
        body.CopyTo(combined, prefix.Length);
        using var hmac = new System.Security.Cryptography.HMACSHA256(secret);
        var digest = Convert.ToHexString(hmac.ComputeHash(combined)).ToLowerInvariant();
        var request = new WebhookVerificationRequest(provider, "evt_1", new Dictionary<string, string>
        {
            ["X-Signature"] = digest,
            ["X-Timestamp"] = staleTimestamp,
        }, body);
        var result = verifier.Verify(request, secret);
        Assert.False(result.IsVerified);
        Assert.Equal(WebhookVerificationStatus.TimestampOutOfRange, result.Status);
    }

    [Fact]
    public void Rejects_missing_signature_header()
    {
        var provider = new WebhookProviderId("test");
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", null, new WebhookOptions());
        var request = new WebhookVerificationRequest(provider, "evt_1", new Dictionary<string, string>(), new byte[] { 0x20 });
        var result = verifier.Verify(request, Encoding.UTF8.GetBytes("super-secret"));
        Assert.Equal(WebhookVerificationStatus.SignatureInvalid, result.Status);
    }

    [Fact]
    public void Failure_does_not_leak_secret_or_payload()
    {
        var provider = new WebhookProviderId("test");
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", new WebhookOptions());
        var request = new WebhookVerificationRequest(provider, "evt_1", new Dictionary<string, string>(), Encoding.UTF8.GetBytes("{\"x\":1}"));
        var result = verifier.Verify(request, Encoding.UTF8.GetBytes("super-secret"));
        Assert.NotNull(result.Failure);
        Assert.DoesNotContain("super-secret", result.Failure!.Message);
        Assert.DoesNotContain("{\"x\":1}", result.Failure.Message);
    }
}
