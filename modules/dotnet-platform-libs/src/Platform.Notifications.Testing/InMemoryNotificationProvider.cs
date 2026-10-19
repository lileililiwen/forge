using Platform.Mailing;
using Platform.Notifications;

namespace Platform.Notifications.Testing;

/// <summary>Deterministic email/SMS provider that records attempts.</summary>
public sealed class InMemoryNotificationProvider : IMailService, ISmsSender, INotificationProviderStatusSource
{
    /// <summary>Recorded normalized attempts.</summary>
    public List<NotificationAttempt> Attempts { get; } = [];
    /// <summary>When set, the next calls return transient failures.</summary>
    public int TransientFailuresRemaining { get; set; }
    /// <inheritdoc />
    public Task<MailSendResult> SendAsync(MailMessage message, CancellationToken cancellationToken = default)
    {
        Attempts.Add(new NotificationAttempt(NotificationChannel.Email, message.To[0].Address, message.TextBody ?? message.HtmlBody ?? string.Empty));
        if (TransientFailuresRemaining-- > 0) return Task.FromResult(new MailSendResult(MailSendOutcome.TransientFailure, ErrorCode: "fake_transient"));
        return Task.FromResult(new MailSendResult(MailSendOutcome.Sent, "fake-email-" + Attempts.Count));
    }
    /// <inheritdoc />
    public Task<NotificationDeliveryResult> SendAsync(SmsMessage message, CancellationToken cancellationToken = default)
    {
        Attempts.Add(new NotificationAttempt(NotificationChannel.Sms, message.To, message.Body));
        if (TransientFailuresRemaining-- > 0) return Task.FromResult(NotificationDeliveryResult.Failed(NotificationFailureCategory.Transient, "fake_transient"));
        return Task.FromResult(NotificationDeliveryResult.Accepted("fake-sms-" + Attempts.Count, 1));
    }
    /// <inheritdoc />
    public IReadOnlyList<NotificationProviderStatus> GetStatuses() => [new("fake", true)];
}

/// <summary>Safe record of a fake delivery attempt.</summary>
public sealed record NotificationAttempt(NotificationChannel Channel, string Recipient, string Body);
