namespace Platform.Mailing;

/// <summary>
/// Documented value type returned by <see cref="IMailTemplateRenderer{TModel}"/>.
/// At least one of <see cref="TextBody"/> or <see cref="HtmlBody"/> must
/// be supplied; the constructor enforces the invariant so consumers
/// never receive an empty template.
/// </summary>
/// <param name="Subject">The rendered subject line.</param>
/// <param name="TextBody">The optional rendered plain-text body.</param>
/// <param name="HtmlBody">The optional rendered HTML body.</param>
public sealed record RenderedMailTemplate(
    string Subject,
    string? TextBody = null,
    string? HtmlBody = null)
{
    /// <summary>
    /// Creates a <see cref="RenderedMailTemplate"/>, validating that
    /// the subject is non-empty and that at least one of the two
    /// bodies is supplied.
    /// </summary>
    /// <exception cref="ArgumentException"><paramref name="subject"/> is null, empty, or whitespace, or neither body is supplied.</exception>
    public static RenderedMailTemplate Create(
        string subject,
        string? textBody = null,
        string? htmlBody = null)
    {
        if (string.IsNullOrWhiteSpace(subject))
        {
            throw new ArgumentException("Subject must be a non-empty string.", nameof(subject));
        }
        if (string.IsNullOrWhiteSpace(textBody) && string.IsNullOrWhiteSpace(htmlBody))
        {
            throw new ArgumentException(
                "At least one of TextBody or HtmlBody must be supplied.",
                nameof(textBody));
        }

        return new RenderedMailTemplate(subject, textBody, htmlBody);
    }
}
