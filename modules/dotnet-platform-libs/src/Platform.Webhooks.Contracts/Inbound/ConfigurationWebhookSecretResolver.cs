using System.Collections.Concurrent;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>Resolves secrets by <c>provider + key</c> using an in-memory dictionary.</summary>
public sealed class ConfigurationWebhookSecretResolver : IWebhookSecretResolver
{
    private readonly object _gate = new();
    private readonly Dictionary<(string Provider, string Key), byte[]> _secrets = new();

    /// <summary>Adds or replaces a secret.</summary>
    public void Add(WebhookProviderId provider, string key, byte[] secret)
    {
        ArgumentNullException.ThrowIfNull(provider.Value);
        if (string.IsNullOrWhiteSpace(key)) throw new ArgumentException("A secret key is required.", nameof(key));
        if (secret is null || secret.Length == 0) throw new ArgumentException("A non-empty secret is required.", nameof(secret));
        lock (_gate) _secrets[(provider.Value, key)] = secret;
    }

    /// <inheritdoc />
    public byte[]? ResolveSecret(WebhookProviderId provider, string secretKey)
    {
        lock (_gate) return _secrets.TryGetValue((provider.Value, secretKey ?? string.Empty), out var secret) ? secret : null;
    }
}
