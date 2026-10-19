namespace Platform.Billing.Contracts.Identifiers;

/// <summary>
/// Opaque identifier for an entitlement subject. The subject may be a
/// user, a tenant, or a composite of both depending on the consuming
/// application.
/// </summary>
/// <param name="Value">The opaque subject identifier.</param>
public readonly record struct SubjectKey(string Value)
{
    /// <summary>
    /// Creates a <see cref="SubjectKey"/> from a non-null string.
    /// </summary>
    /// <param name="value">The opaque subject identifier.</param>
    /// <returns>The <see cref="SubjectKey"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="value"/> is null, empty, or whitespace.</exception>
    public static SubjectKey Create(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new ArgumentException("Subject key must be a non-empty string.", nameof(value));
        }
        return new SubjectKey(value);
    }

    /// <summary>
    /// The subject key used when no caller is observed. Distinct from
    /// <c>null</c> so feature checks can return a stable
    /// non-authenticated decision without null checks at every call
    /// site.
    /// </summary>
    public static SubjectKey Anonymous { get; } = new("anonymous");

    /// <summary>
    /// Gets a value indicating whether the subject key is the
    /// platform's anonymous marker.
    /// </summary>
    public bool IsAnonymous => Value == Anonymous.Value;
}
