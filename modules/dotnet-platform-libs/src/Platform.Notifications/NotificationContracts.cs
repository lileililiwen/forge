namespace Platform.Notifications;

/// <summary>Supported notification channels in this capability slice.</summary>
public enum NotificationChannel
{
    /// <summary>Email delivery.</summary>
    Email,
    /// <summary>SMS delivery.</summary>
    Sms,
}

/// <summary>Normalized failure categories safe for application diagnostics.</summary>
public enum NotificationFailureCategory
{
    /// <summary>Destination address or number is invalid.</summary>
    InvalidAddress,
    /// <summary>Provider configuration is missing or invalid.</summary>
    Configuration,
    /// <summary>Provider authentication failed.</summary>
    Authentication,
    /// <summary>Provider may succeed on a later attempt.</summary>
    Transient,
    /// <summary>Provider rejected the message permanently.</summary>
    Permanent,
}

/// <summary>Outcome of a notification delivery request.</summary>
public enum NotificationDeliveryOutcome
{
    /// <summary>Provider accepted the notification.</summary>
    Accepted,
    /// <summary>Idempotency prevented a repeated delivery.</summary>
    Duplicate,
    /// <summary>Delivery did not succeed.</summary>
    Failed,
}

/// <summary>Safe notification failure details.</summary>
public sealed record NotificationFailure(NotificationFailureCategory Category, string Code, string Message);

/// <summary>Result returned from a notification delivery attempt.</summary>
public sealed record NotificationDeliveryResult(
    NotificationDeliveryOutcome Outcome,
    NotificationFailure? Failure = null,
    string? ProviderMessageId = null,
    int Attempts = 0)
{
    /// <summary>Creates an accepted result.</summary>
    public static NotificationDeliveryResult Accepted(string? providerMessageId, int attempts) => new(NotificationDeliveryOutcome.Accepted, null, providerMessageId, attempts);
    /// <summary>Creates a duplicate result.</summary>
    public static NotificationDeliveryResult Duplicate() => new(NotificationDeliveryOutcome.Duplicate);
    /// <summary>Creates a failure with category-safe text.</summary>
    public static NotificationDeliveryResult Failed(NotificationFailureCategory category, string code, string? detail = null) => new(NotificationDeliveryOutcome.Failed, new NotificationFailure(category, code, SafeMessage(category, detail)));
    private static string SafeMessage(NotificationFailureCategory category, string? detail) => category switch
    {
        NotificationFailureCategory.InvalidAddress => "The destination is invalid.",
        NotificationFailureCategory.Configuration => "The notification provider is not configured.",
        NotificationFailureCategory.Authentication => "The notification provider rejected authentication.",
        NotificationFailureCategory.Transient => "The notification provider is temporarily unavailable.",
        _ => "The notification provider rejected delivery.",
    };
}

/// <summary>Provider-neutral intent for email or SMS delivery.</summary>
public sealed record NotificationIntent
{
    private NotificationIntent(string id, NotificationChannel channel, string recipient, string? subject, string body, string? idempotencyKey)
    {
        if (string.IsNullOrWhiteSpace(id)) throw new ArgumentException("Notification id is required.", nameof(id));
        if (string.IsNullOrWhiteSpace(recipient)) throw new ArgumentException("Recipient is required.", nameof(recipient));
        if (string.IsNullOrWhiteSpace(body)) throw new ArgumentException("Body is required.", nameof(body));
        if (channel == NotificationChannel.Email && string.IsNullOrWhiteSpace(subject)) throw new ArgumentException("Email subject is required.", nameof(subject));
        Id = id; Channel = channel; Recipient = recipient; Subject = subject; Body = body; IdempotencyKey = string.IsNullOrWhiteSpace(idempotencyKey) ? id : idempotencyKey;
    }
    /// <summary>Gets the application-owned notification identifier.</summary>
    public string Id { get; }
    /// <summary>Gets the delivery channel.</summary>
    public NotificationChannel Channel { get; }
    /// <summary>Gets the normalized destination.</summary>
    public string Recipient { get; }
    /// <summary>Gets the email subject, or null for SMS.</summary>
    public string? Subject { get; }
    /// <summary>Gets the rendered message body.</summary>
    public string Body { get; }
    /// <summary>Gets the stable duplicate-suppression key.</summary>
    public string IdempotencyKey { get; init; }
    /// <summary>Creates an email intent.</summary>
    public static NotificationIntent Email(string id, string address, string subject, string body) => new(id, NotificationChannel.Email, address, subject, body, null);
    /// <summary>Creates an SMS intent.</summary>
    public static NotificationIntent Sms(string id, string phoneNumber, string body) => new(id, NotificationChannel.Sms, phoneNumber, null, body, null);
}

/// <summary>Provider-neutral normalized SMS message.</summary>
public sealed record SmsMessage(string To, string Body)
{
    /// <summary>Creates and validates an SMS message.</summary>
    public static SmsMessage Create(string to, string body)
    {
        if (string.IsNullOrWhiteSpace(to)) throw new ArgumentException("Phone number is required.", nameof(to));
        if (string.IsNullOrWhiteSpace(body)) throw new ArgumentException("Body is required.", nameof(body));
        return new SmsMessage(to, body);
    }
}

/// <summary>Provider-neutral SMS sender seam.</summary>
public interface ISmsSender
{
    /// <summary>Sends an SMS message.</summary>
    Task<NotificationDeliveryResult> SendAsync(SmsMessage message, CancellationToken cancellationToken = default);
}

/// <summary>Application-facing channel-neutral sender.</summary>
public interface INotificationDispatcher
{
    /// <summary>Delivers an intent with duplicate suppression and bounded retries.</summary>
    Task<NotificationDeliveryResult> SendAsync(NotificationIntent intent, CancellationToken cancellationToken = default);
}

/// <summary>Describes configured notification-provider availability.</summary>
public sealed record NotificationProviderStatus(string Name, bool Available, string? Detail = null);

/// <summary>Supplies safe notification provider health state.</summary>
public interface INotificationProviderStatusSource
{
    /// <summary>Gets provider statuses.</summary>
    IReadOnlyList<NotificationProviderStatus> GetStatuses();
}
