namespace Platform.Jobs.Hangfire;

/// <summary>
/// Application-owned handler invoked for dispatched
/// <see cref="JobPayload"/> instances. The adapter resolves a single
/// registered handler inside the job's execution scope and routes every
/// dispatched payload to it; the application decides how payload names map
/// to work items.
/// </summary>
public interface IJobPayloadHandler
{
    /// <summary>
    /// Handles the supplied <paramref name="payload"/>. Implementations
    /// MUST honour the supplied <paramref name="cancellationToken"/>,
    /// which is bound to the Hangfire shutdown token.
    /// </summary>
    /// <param name="payload">The dispatched payload.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <returns>A task that completes when the payload is handled.</returns>
    Task HandleAsync(JobPayload payload, CancellationToken cancellationToken);
}
