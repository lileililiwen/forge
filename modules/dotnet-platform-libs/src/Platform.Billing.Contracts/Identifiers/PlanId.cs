namespace Platform.Billing.Contracts.Identifiers;

/// <summary>
/// Opaque identifier for a subscription plan. Plan identifiers are
/// application-defined; the platform never interprets them.
/// </summary>
/// <param name="Value">The opaque plan identifier.</param>
public readonly record struct PlanId(string Value)
{
    /// <summary>
    /// Creates a <see cref="PlanId"/> from a non-null string.
    /// </summary>
    /// <param name="value">The opaque plan identifier.</param>
    /// <returns>The <see cref="PlanId"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="value"/> is null, empty, or whitespace.</exception>
    public static PlanId Create(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new ArgumentException("Plan id must be a non-empty string.", nameof(value));
        }
        return new PlanId(value);
    }
}
