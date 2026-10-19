namespace Platform.Quota.Contracts;

/// <summary>Atomic quota check and reservation lifecycle boundary.</summary>
public interface IQuotaStore
{
    /// <summary>Checks capacity without changing state.</summary>
    Task<QuotaDecision> CheckAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, long requested, CancellationToken cancellationToken = default);
    /// <summary>Atomically reserves capacity using an idempotent operation key.</summary>
    Task<QuotaLifecycleResult> ReserveAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, string operationKey, long amount, TimeSpan? reservationLifetime = null, CancellationToken cancellationToken = default);
    /// <summary>Settles a reservation.</summary>
    Task<QuotaLifecycleResult> SettleAsync(string operationKey, CancellationToken cancellationToken = default);
    /// <summary>Releases a reservation.</summary>
    Task<QuotaLifecycleResult> ReleaseAsync(string operationKey, CancellationToken cancellationToken = default);
    /// <summary>Reads current usage for inspection and reconciliation.</summary>
    Task<QuotaSnapshot> GetSnapshotAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, CancellationToken cancellationToken = default);
}

/// <summary>Current quota usage snapshot.</summary>
public sealed record QuotaSnapshot(QuotaSubject Subject, QuotaResource Resource, QuotaWindow Window, long Limit, long Consumed, long Reserved)
{
    /// <summary>Capacity remaining.</summary>
    public long Remaining => Math.Max(0, Limit - Consumed - Reserved);
}
