using Platform.Jobs;

namespace Platform.Notifications;

/// <summary>Stable job names and scheduling helpers for notifications.</summary>
public static class NotificationScheduling
{
    /// <summary>Stable job name consumed by application job handlers.</summary>
    public const string DeliveryJobName = "platform.notifications.deliver";
    /// <summary>Enqueues an intent without coupling the package to a job engine.</summary>
    public static Task EnqueueAsync(IJobDispatcher dispatcher, NotificationIntent intent, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(dispatcher);
        ArgumentNullException.ThrowIfNull(intent);
        return dispatcher.EnqueueAsync(JobPayload.Create(DeliveryJobName, new Dictionary<string, object?> { ["notification_id"] = intent.Id, ["idempotency_key"] = intent.IdempotencyKey }), cancellationToken);
    }
}
