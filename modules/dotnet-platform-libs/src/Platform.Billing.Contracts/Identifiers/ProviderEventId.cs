namespace Platform.Billing.Contracts.Identifiers;

/// <summary>
/// Opaque identifier for a provider webhook event, used to ensure
/// idempotent processing of redelivered events.
/// </summary>
/// <param name="Value">The opaque event identifier.</param>
public readonly record struct ProviderEventId(string Value)
{
    /// <summary>
    /// Creates a <see cref="ProviderEventId"/> from a non-null string.
    /// </summary>
    /// <param name="value">The opaque event identifier.</param>
    /// <returns>The <see cref="ProviderEventId"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="value"/> is null, empty, or whitespace.</exception>
    public static ProviderEventId Create(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new ArgumentException("Provider event id must be a non-empty string.", nameof(value));
        }
        return new ProviderEventId(value);
    }
}
