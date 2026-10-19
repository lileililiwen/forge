namespace Platform.Auditing.Contracts;

/// <summary>How the recorder dispatches events to the sink.</summary>
public enum AuditPublishMode
{
    /// <summary>Await the sink inline. Simplest and fully deterministic for tests.</summary>
    Synchronous = 0,

    /// <summary>
    /// Enqueue onto a bounded channel and publish on a background reader. Dropped when the channel
    /// is full; the business request is never blocked.
    /// </summary>
    BoundedAsync = 1,
}
