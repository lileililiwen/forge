namespace Platform.Jobs;

/// <summary>
/// Contract for a dispatcher that enqueues a <see cref="JobPayload"/> to
/// a scheduling engine. Implementations live in the consumer; the
/// platform exposes only the contract so any engine (Hangfire, Quartz,
/// a hosted service, …) can satisfy it without changing call sites.
/// </summary>
public interface IJobDispatcher
{
    /// <summary>
    /// Enqueues the supplied <paramref name="payload"/> for execution.
    /// Implementations MUST read the current time from an injected
    /// <see cref="Platform.Core.Time.IClock"/> rather than calling
    /// <see cref="DateTimeOffset.UtcNow"/> directly so the dispatch
    /// remains deterministic in tests.
    /// </summary>
    /// <param name="payload">The documented payload to dispatch.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <exception cref="ArgumentNullException"><paramref name="payload"/> is <c>null</c>.</exception>
    Task EnqueueAsync(JobPayload payload, CancellationToken cancellationToken = default);
}
