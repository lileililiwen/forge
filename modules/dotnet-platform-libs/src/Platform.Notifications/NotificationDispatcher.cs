using System.Security.Cryptography;
using System.Text;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Idempotency;
using Platform.Mailing;

namespace Platform.Notifications;

/// <summary>Dispatches normalized intents to application-owned providers.</summary>
public sealed class NotificationDispatcher : INotificationDispatcher, IDisposable
{
    private readonly IMailService[] mailServices;
    private readonly ISmsSender[] smsSenders;
    private readonly IIdempotencyStore idempotency;
    private readonly IClock clock;
    private readonly NotificationOptions options;
    private readonly SemaphoreSlim gate = new(1, 1);

    /// <summary>Initializes the dispatcher with explicit provider and storage dependencies.</summary>
    public NotificationDispatcher(IEnumerable<IMailService> mailServices, IEnumerable<ISmsSender> smsSenders, IIdempotencyStore idempotency, IClock clock, IOptions<NotificationOptions> options)
    {
        this.mailServices = mailServices?.ToArray() ?? throw new ArgumentNullException(nameof(mailServices));
        this.smsSenders = smsSenders?.ToArray() ?? throw new ArgumentNullException(nameof(smsSenders));
        this.idempotency = idempotency ?? throw new ArgumentNullException(nameof(idempotency));
        this.clock = clock ?? throw new ArgumentNullException(nameof(clock));
        this.options = options?.Value ?? throw new ArgumentNullException(nameof(options));
    }

    /// <inheritdoc />
    public async Task<NotificationDeliveryResult> SendAsync(NotificationIntent intent, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(intent);
        await gate.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            var key = intent.IdempotencyKey;
            if (await idempotency.TryGetAsync(key, cancellationToken).ConfigureAwait(false) is not null) return NotificationDeliveryResult.Duplicate();
            var result = await SendWithRetryAsync(intent, cancellationToken).ConfigureAwait(false);
            if (result.Outcome == NotificationDeliveryOutcome.Accepted)
            {
                await idempotency.SaveAsync(new IdempotencyRecord(key, Fingerprint(intent), 200, "application/json", "accepted", clock.UtcNow), cancellationToken).ConfigureAwait(false);
            }
            return result;
        }
        finally { gate.Release(); }
    }

    private async Task<NotificationDeliveryResult> SendWithRetryAsync(NotificationIntent intent, CancellationToken cancellationToken)
    {
        if (intent.Channel == NotificationChannel.Email)
        {
            if (mailServices.Length == 0) return NotificationDeliveryResult.Failed(NotificationFailureCategory.Configuration, "email_provider_unconfigured");
            var message = new MailMessage(MailAddress.Create("no-reply@platform.invalid"), [MailAddress.Create(intent.Recipient)], intent.Subject!, intent.Body);
            for (var attempt = 1; attempt <= options.MaxAttempts; attempt++)
            {
                var result = await mailServices[0].SendAsync(message, cancellationToken).ConfigureAwait(false);
                if (result.Outcome == MailSendOutcome.Sent || result.Outcome == MailSendOutcome.Bounced) return NotificationDeliveryResult.Accepted(result.ProviderMessageId, attempt);
                if (result.Outcome == MailSendOutcome.PermanentFailure) return NotificationDeliveryResult.Failed(NotificationFailureCategory.Permanent, result.ErrorCode ?? "email_delivery_failed");
                if (attempt == options.MaxAttempts) return NotificationDeliveryResult.Failed(NotificationFailureCategory.Transient, result.ErrorCode ?? "email_provider_unavailable");
            }
        }
        else
        {
            if (smsSenders.Length == 0) return NotificationDeliveryResult.Failed(NotificationFailureCategory.Configuration, "sms_provider_unconfigured");
            for (var attempt = 1; attempt <= options.MaxAttempts; attempt++)
            {
                var result = await smsSenders[0].SendAsync(SmsMessage.Create(intent.Recipient, intent.Body), cancellationToken).ConfigureAwait(false);
                if (result.Outcome == NotificationDeliveryOutcome.Accepted) return result with { Attempts = attempt };
                if (result.Failure?.Category is NotificationFailureCategory.InvalidAddress or NotificationFailureCategory.Permanent or NotificationFailureCategory.Authentication or NotificationFailureCategory.Configuration) return result with { Attempts = attempt };
                if (attempt == options.MaxAttempts) return result with { Attempts = attempt };
            }
        }
        return NotificationDeliveryResult.Failed(NotificationFailureCategory.Transient, "delivery_failed");
    }

    private static string Fingerprint(NotificationIntent intent)
    {
        var bytes = SHA256.HashData(Encoding.UTF8.GetBytes($"{intent.Channel}|{intent.Recipient}|{intent.Subject}|{intent.Body}"));
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }

    /// <summary>Releases the dispatcher gate.</summary>
    public void Dispose() => gate.Dispose();
}
