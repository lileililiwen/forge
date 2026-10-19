namespace Platform.Quota.Contracts;

/// <summary>Explains a quota capacity decision.</summary>
public sealed record QuotaDecision
{
    /// <summary>Creates a decision with full capacity context.</summary>
    public QuotaDecision(bool allowed, QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, long consumed, long reserved, long requested)
    {
        if (limit < 0 || consumed < 0 || reserved < 0 || requested < 0) throw new ArgumentOutOfRangeException(nameof(limit));
        Allowed = allowed; Subject = subject; Resource = resource; Window = window; Limit = limit; Consumed = consumed; Reserved = reserved; Requested = requested;
    }
    /// <summary>Whether the requested amount is accepted.</summary>
    public bool Allowed { get; }
    /// <summary>Quota subject.</summary>
    public QuotaSubject Subject { get; }
    /// <summary>Quota resource.</summary>
    public QuotaResource Resource { get; }
    /// <summary>Accounting window.</summary>
    public QuotaWindow Window { get; }
    /// <summary>Configured capacity.</summary>
    public long Limit { get; }
    /// <summary>Settled amount.</summary>
    public long Consumed { get; }
    /// <summary>Currently reserved amount.</summary>
    public long Reserved { get; }
    /// <summary>Requested amount.</summary>
    public long Requested { get; }
    /// <summary>Capacity remaining after settled and reserved amounts.</summary>
    public long Remaining => Math.Max(0, Limit - Consumed - Reserved);
}
