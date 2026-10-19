namespace Platform.Jobs;

#pragma warning disable CA1716 // Intentional: "Error" matches the platform's failure-description type name in this style of API.

/// <summary>
/// Documented telemetry surface that consumers wire to their existing
/// observability layer. The platform does NOT ship a default
/// implementation; consumers opt in by registering their own. The
/// surface is invoked by <see cref="IRecurringJobRegistry.Register"/>
/// and by <see cref="IJobDispatcher.EnqueueAsync"/> when the consumer
/// chooses to call it.
/// </summary>
public interface IJobTelemetry
{
    /// <summary>
    /// Records that a recurring job was registered.
    /// </summary>
    /// <param name="descriptor">The descriptor that was registered.</param>
    /// <param name="registeredAt">The UTC time the registration was captured.</param>
    void JobRegistered(RecurringJobDescriptor descriptor, DateTimeOffset registeredAt);

    /// <summary>
    /// Records that a job was enqueued for execution.
    /// </summary>
    /// <param name="payload">The payload that was enqueued.</param>
    /// <param name="enqueuedAt">The UTC time the enqueue was captured.</param>
    void JobEnqueued(JobPayload payload, DateTimeOffset enqueuedAt);

    /// <summary>
    /// Records that a job completed successfully.
    /// </summary>
    /// <param name="name">The job name.</param>
    /// <param name="executedAt">The UTC time the execution completed.</param>
    void JobExecuted(string name, DateTimeOffset executedAt);

    /// <summary>
    /// Records that a job failed.
    /// </summary>
    /// <param name="name">The job name.</param>
    /// <param name="error">The failure description.</param>
    /// <param name="failedAt">The UTC time the failure was captured.</param>
    void JobFailed(string name, Platform.Core.Results.Error error, DateTimeOffset failedAt);
}

#pragma warning restore CA1716
