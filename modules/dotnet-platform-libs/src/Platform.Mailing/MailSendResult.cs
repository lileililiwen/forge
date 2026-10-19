namespace Platform.Mailing;

/// <summary>
/// Documented result returned by <see cref="IMailService.SendAsync"/>. The
/// provider populates <see cref="Outcome"/>; the other fields are
/// optional and are populated when the provider supplies them.
/// </summary>
/// <param name="Outcome">The documented outcome of the send attempt.</param>
/// <param name="ProviderMessageId">The provider-assigned message identifier, or <c>null</c> when the provider did not assign one.</param>
/// <param name="ErrorCode">A stable error code on failure, or <c>null</c> on success.</param>
/// <param name="ErrorMessage">A safe error message on failure, or <c>null</c> on success.</param>
public sealed record MailSendResult(
    MailSendOutcome Outcome,
    string? ProviderMessageId = null,
    string? ErrorCode = null,
    string? ErrorMessage = null)
{
    /// <summary>
    /// Gets a value indicating whether the send was accepted by the
    /// provider.
    /// </summary>
    public bool IsAccepted => Outcome is MailSendOutcome.Sent or MailSendOutcome.Bounced;
}
