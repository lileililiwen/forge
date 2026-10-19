namespace Platform.Billing.Contracts.Identifiers;

/// <summary>
/// Opaque identifier for a feature gate. Feature keys are
/// application-defined; the platform never interprets them.
/// </summary>
/// <param name="Value">The opaque feature key.</param>
public readonly record struct FeatureKey(string Value)
{
    /// <summary>
    /// Creates a <see cref="FeatureKey"/> from a non-null string.
    /// </summary>
    /// <param name="value">The opaque feature key.</param>
    /// <returns>The <see cref="FeatureKey"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="value"/> is null, empty, or whitespace.</exception>
    public static FeatureKey Create(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new ArgumentException("Feature key must be a non-empty string.", nameof(value));
        }
        return new FeatureKey(value);
    }
}
