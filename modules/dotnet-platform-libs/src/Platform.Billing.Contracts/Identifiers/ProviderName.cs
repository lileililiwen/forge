namespace Platform.Billing.Contracts.Identifiers;

/// <summary>
/// Opaque identifier for a payment or subscription provider, such as
/// <c>stripe</c> or <c>paddle</c>. Provider names are
/// application-defined; the platform never interprets them.
/// </summary>
/// <param name="Value">The opaque provider name.</param>
public readonly record struct ProviderName(string Value)
{
    /// <summary>
    /// Creates a <see cref="ProviderName"/> from a non-null string.
    /// </summary>
    /// <param name="value">The provider name.</param>
    /// <returns>The <see cref="ProviderName"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="value"/> is null, empty, or whitespace.</exception>
    public static ProviderName Create(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new ArgumentException("Provider name must be a non-empty string.", nameof(value));
        }
        return new ProviderName(value);
    }
}
