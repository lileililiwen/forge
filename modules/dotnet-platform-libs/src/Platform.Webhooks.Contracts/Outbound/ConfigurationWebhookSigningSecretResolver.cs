namespace Platform.Webhooks.Contracts.Outbound;

/// <summary>Resolves signing secrets from an in-memory dictionary keyed by secret key.</summary>
public sealed class ConfigurationWebhookSigningSecretResolver : IWebhookSigningSecretResolver
{
    private readonly object _gate = new();
    private readonly Dictionary<string, byte[]> _secrets = new(StringComparer.Ordinal);

    /// <summary>Adds or replaces a secret.</summary>
    public void Add(string key, byte[] secret)
    {
        if (string.IsNullOrWhiteSpace(key)) throw new ArgumentException("A secret key is required.", nameof(key));
        if (secret is null || secret.Length == 0) throw new ArgumentException("A non-empty secret is required.", nameof(secret));
        lock (_gate) _secrets[key] = secret;
    }

    /// <inheritdoc />
    public byte[]? ResolveSecret(string secretKey)
    {
        if (string.IsNullOrWhiteSpace(secretKey)) return null;
        lock (_gate) return _secrets.TryGetValue(secretKey, out var secret) ? secret : null;
    }
}
