using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>Resolves the secret used to verify an inbound webhook signature for the given provider and key.</summary>
public interface IWebhookSecretResolver
{
    /// <summary>Returns the secret bytes for the supplied <paramref name="secretKey"/>, or <c>null</c> when the key is unknown.</summary>
    byte[]? ResolveSecret(WebhookProviderId provider, string secretKey);
}
