namespace Platform.Auditing.Contracts;

/// <summary>
/// Masks a metadata or entity property value before it is recorded. Implementations decide what is
/// sensitive based on the key name and value, never on caller intent.
/// </summary>
public interface IAuditMasker
{
    /// <summary>Returns the value that should be stored for <paramref name="key"/>.</summary>
    /// <param name="key">The metadata or property name.</param>
    /// <param name="value">The raw value, or <c>null</c>.</param>
    /// <returns>A safe value; sensitive values are replaced with a redaction placeholder.</returns>
    string Mask(string key, string? value);
}
