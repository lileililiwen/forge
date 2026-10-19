namespace Platform.Mailing;

/// <summary>
/// Documented value type for an outgoing mail message. The construction
/// validates that at least one of <see cref="TextBody"/> or
/// <see cref="HtmlBody"/> is supplied so providers never receive an
/// empty message.
/// </summary>
public sealed record MailMessage
{
    /// <summary>
    /// Initializes a new <see cref="MailMessage"/> with the supplied
    /// addresses, subject, and bodies.
    /// </summary>
    /// <param name="from">The sender address.</param>
    /// <param name="to">The recipient addresses.</param>
    /// <param name="subject">The subject line.</param>
    /// <param name="textBody">The optional plain-text body.</param>
    /// <param name="htmlBody">The optional HTML body.</param>
    /// <param name="attachments">The optional attachments.</param>
    /// <param name="correlationId">The optional correlation identifier.</param>
    /// <exception cref="ArgumentNullException">A required argument is <c>null</c>.</exception>
    /// <exception cref="ArgumentException">A required string is empty or whitespace, or neither <paramref name="textBody"/> nor <paramref name="htmlBody"/> is supplied.</exception>
    public MailMessage(
        MailAddress from,
        IReadOnlyList<MailAddress> to,
        string subject,
        string? textBody = null,
        string? htmlBody = null,
        IReadOnlyList<MailAttachment>? attachments = null,
        string? correlationId = null)
    {
        ArgumentNullException.ThrowIfNull(from);
        ArgumentNullException.ThrowIfNull(to);
        if (string.IsNullOrEmpty(subject))
        {
            throw new ArgumentException("Subject must be a non-empty string.", nameof(subject));
        }
        if (to.Count == 0)
        {
            throw new ArgumentException("At least one recipient is required.", nameof(to));
        }
        if (string.IsNullOrWhiteSpace(textBody) && string.IsNullOrWhiteSpace(htmlBody))
        {
            throw new ArgumentException(
                "At least one of TextBody or HtmlBody must be supplied.",
                nameof(textBody));
        }

        From = from;
        To = to;
        Subject = subject;
        TextBody = textBody;
        HtmlBody = htmlBody;
        Attachments = attachments ?? Array.Empty<MailAttachment>();
        CorrelationId = correlationId;
    }

    /// <summary>Gets the sender address.</summary>
    public MailAddress From { get; }

    /// <summary>Gets the recipient addresses.</summary>
    public IReadOnlyList<MailAddress> To { get; }

    /// <summary>Gets the subject line.</summary>
    public string Subject { get; }

    /// <summary>Gets the plain-text body, or <c>null</c> when only an HTML body is supplied.</summary>
    public string? TextBody { get; }

    /// <summary>Gets the HTML body, or <c>null</c> when only a plain-text body is supplied.</summary>
    public string? HtmlBody { get; }

    /// <summary>Gets the attachments; an empty list when none are attached.</summary>
    public IReadOnlyList<MailAttachment> Attachments { get; }

    /// <summary>Gets the optional correlation identifier.</summary>
    public string? CorrelationId { get; }
}
