using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>Verifies the raw bytes and headers of an inbound webhook using a provider-supplied algorithm.</summary>
public interface IWebhookSignatureVerifier
{
    /// <summary>Provider identity used to look up secrets and choose canonicalization rules.</summary>
    WebhookProviderId Provider { get; }
    /// <summary>Validates the raw request bytes against the configured signature.</summary>
    WebhookVerificationResult Verify(WebhookVerificationRequest request, byte[] secret);
}
