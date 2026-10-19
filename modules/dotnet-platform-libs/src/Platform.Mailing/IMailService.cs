namespace Platform.Mailing;

/// <summary>
/// Contract for an outbound mail service. Implementations live in the
/// consumer; the platform exposes only the contract so any provider
/// (SendGrid, Mailgun, Postmark, SMTP, …) can satisfy it without
/// changing call sites.
/// </summary>
public interface IMailService
{
    /// <summary>
    /// Sends the supplied <paramref name="message"/>. Implementations
    /// MUST honour the <paramref name="cancellationToken"/> and return
    /// a <see cref="MailSendResult"/> that describes the provider's
    /// outcome.
    /// </summary>
    /// <param name="message">The message to send.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <exception cref="ArgumentNullException"><paramref name="message"/> is <c>null</c>.</exception>
    Task<MailSendResult> SendAsync(MailMessage message, CancellationToken cancellationToken = default);
}
