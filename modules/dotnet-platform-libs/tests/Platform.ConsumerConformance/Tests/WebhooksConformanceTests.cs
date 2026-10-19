using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.ConsumerConformance.Fixtures;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.DependencyInjection;
using Platform.Webhooks.Contracts.Inbound;
using Platform.Webhooks.Contracts.Outbound;
using Platform.Webhooks.Contracts.Security;

namespace Platform.ConsumerConformance.Tests;

public sealed class WebhooksConformanceTests
{
    [Fact]
    public void AddPlatformWebhooks_registers_in_memory_stores_and_default_validator()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformWebhooks();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        Assert.IsType<InMemoryWebhookInboxStore>(provider.GetRequiredService<IWebhookInboxStore>());
        Assert.IsType<InMemoryWebhookSubscriptionStore>(provider.GetRequiredService<IWebhookSubscriptionStore>());
        Assert.IsType<InMemoryWebhookDeliveryStore>(provider.GetRequiredService<IWebhookDeliveryStore>());
        Assert.IsType<InMemoryWebhookBackendStatusProvider>(provider.GetRequiredService<IWebhookBackendStatusProvider>());
    }

    [Fact]
    public void AddPlatformWebhooks_applies_configure_delegate()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformWebhooks(options => { });

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<WebhookOptions>>().Value;
        Assert.Equal(1_048_576L, options.MaximumInboundBodyBytes);
    }

    [Fact]
    public void Hmac_verifier_rejects_mismatched_signature()
    {
        var options = new WebhookOptions();
        var verifier = new HmacWebhookSignatureVerifier(new WebhookProviderId("test"), "X-Signature", null, options);
        var secret = new byte[] { 1, 2, 3, 4 };
        var payload = "payload"u8.ToArray();

        var validSignature = ComputeHmac(secret, payload);
        var validResult = verifier.Verify(new WebhookVerificationRequest(new WebhookProviderId("test"), "evt-1",
            new Dictionary<string, string> { ["X-Signature"] = Convert.ToHexString(validSignature) },
            payload), secret);
        Assert.Equal(WebhookVerificationStatus.Verified, validResult.Status);

        var invalidResult = verifier.Verify(new WebhookVerificationRequest(new WebhookProviderId("test"), "evt-1",
            new Dictionary<string, string> { ["X-Signature"] = Convert.ToHexString(new byte[] { 9, 9, 9 }) },
            payload), secret);
        Assert.Equal(WebhookVerificationStatus.SignatureInvalid, invalidResult.Status);
    }

    [Fact]
    public void Ssrf_validator_rejects_loopback_addresses()
    {
        var options = new WebhookOptions();
        var validator = new SsrfTargetValidator(options);
        var loopback = validator.Validate(new Uri("https://127.0.0.1/secret"));
        Assert.False(loopback.Allowed);
        Assert.Equal(WebhookTargetRejection.Loopback, loopback.Rejection);
    }

    [Fact]
    public void Ssrf_validator_rejects_plain_http()
    {
        var options = new WebhookOptions();
        var validator = new SsrfTargetValidator(options);
        var insecure = validator.Validate(new Uri("http://example.com/hook"));
        Assert.False(insecure.Allowed);
        Assert.Equal(WebhookTargetRejection.InsecureScheme, insecure.Rejection);
    }

    private static byte[] ComputeHmac(byte[] key, byte[] payload)
    {
        using var hmac = new System.Security.Cryptography.HMACSHA256(key);
        return hmac.ComputeHash(payload);
    }
}
