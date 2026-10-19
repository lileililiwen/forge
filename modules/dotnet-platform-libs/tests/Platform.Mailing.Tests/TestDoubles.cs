namespace Platform.Mailing.Tests;

public sealed class RecordingMailService : IMailService
{
    public List<MailMessage> Sent { get; } = new();

    public Task<MailSendResult> SendAsync(MailMessage message, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(message);
        Sent.Add(message);
        return Task.FromResult(new MailSendResult(MailSendOutcome.Sent, "msg-1"));
    }
}
