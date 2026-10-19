namespace Platform.Mailing;

/// <summary>
/// Documented value type for a single mail address. The display name is
/// optional; the address is always required.
/// </summary>
/// <param name="Address">The email address. MUST be non-null and non-empty.</param>
/// <param name="DisplayName">The optional human-readable name.</param>
public sealed record MailAddress(string Address, string? DisplayName = null)
{
    /// <summary>
    /// Creates a <see cref="MailAddress"/> with the supplied address and
    /// no display name.
    /// </summary>
    /// <param name="address">The email address.</param>
    /// <returns>The constructed <see cref="MailAddress"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="address"/> is null, empty, or whitespace.</exception>
    public static MailAddress Create(string address)
    {
        if (string.IsNullOrWhiteSpace(address))
        {
            throw new ArgumentException("Address must be a non-empty string.", nameof(address));
        }

        return new MailAddress(address);
    }
}
