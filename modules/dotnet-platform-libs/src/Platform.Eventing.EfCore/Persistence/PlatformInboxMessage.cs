using Platform.Eventing.Contracts;

namespace Platform.Eventing.EfCore;

/// <summary>EF Core persistence model for an inbox message.</summary>
public sealed class PlatformInboxMessage
{
    /// <summary>Stable message identifier.</summary>
    public string MessageId { get; set; } = string.Empty;
    /// <summary>Serialized payload type.</summary>
    public string PayloadType { get; set; } = string.Empty;
    /// <summary>Serialized payload JSON.</summary>
    public string PayloadJson { get; set; } = string.Empty;
    /// <summary>Event occurrence time.</summary>
    public DateTimeOffset OccurredAt { get; set; }
    /// <summary>Receive time.</summary>
    public DateTimeOffset ReceivedAt { get; set; }
    /// <summary>Optional tenant identifier.</summary>
    public string? TenantId { get; set; }
    /// <summary>Optional correlation identifier.</summary>
    public string? CorrelationId { get; set; }
    /// <summary>Processing state.</summary>
    public DurableMessageState State { get; set; }
    /// <summary>Number of claim attempts.</summary>
    public int AttemptCount { get; set; }
    /// <summary>Next eligible retry time.</summary>
    public DateTimeOffset? NextAttemptAt { get; set; }
    /// <summary>Current lease owner.</summary>
    public string? LeaseOwnerId { get; set; }
    /// <summary>Current lease expiry.</summary>
    public DateTimeOffset? LeaseExpiresAt { get; set; }
    /// <summary>Last safe failure code.</summary>
    public string? LastFailureCode { get; set; }
    /// <summary>Last safe failure message.</summary>
    public string? LastFailureMessage { get; set; }
    /// <summary>Whether the last failure is permanent.</summary>
    public bool LastFailurePermanent { get; set; }
}
