namespace Platform.Mailing;

/// <summary>
/// Opaque, type-safe template identifier. The platform never interprets
/// the value; consumers route the identifier to their preferred
/// templating engine. Implicit conversions to and from <see cref="string"/>
/// are provided so logging and configuration binding stay concise.
/// </summary>
public readonly record struct MailTemplateId
{
    /// <summary>
    /// Initializes a new <see cref="MailTemplateId"/> from the supplied
    /// non-empty string.
    /// </summary>
    /// <param name="value">The opaque template identifier.</param>
    /// <exception cref="ArgumentException"><paramref name="value"/> is null, empty, or whitespace.</exception>
    public MailTemplateId(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new ArgumentException("Template id must be a non-empty string.", nameof(value));
        }

        Value = value;
    }

    /// <summary>Gets the opaque template identifier.</summary>
    public string Value { get; }

    /// <summary>
    /// Implicitly converts the supplied <paramref name="value"/> to a
    /// <see cref="MailTemplateId"/>. Throws <see cref="ArgumentException"/>
    /// when the value is null, empty, or whitespace.
    /// </summary>
    public static implicit operator MailTemplateId(string value) => new(value);

    /// <summary>
    /// Implicitly converts the <see cref="MailTemplateId"/> to its
    /// underlying <see cref="string"/> value.
    /// </summary>
    public static implicit operator string(MailTemplateId id) => id.Value;

    /// <inheritdoc />
    public override string ToString() => Value;
}
