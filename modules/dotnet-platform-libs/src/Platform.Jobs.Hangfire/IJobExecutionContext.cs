namespace Platform.Jobs.Hangfire;

/// <summary>
/// Application-owned bridge between the platform job pipeline and the
/// host's ambient tenant/subject context. The adapter captures a
/// <see cref="JobContextSnapshot"/> when a job is created and restores it
/// inside the job's execution scope; the application decides how the
/// ambient context is read and re-established (for example through an
/// ambient tenant scope and a current-user initializer).
/// </summary>
public interface IJobExecutionContext
{
    /// <summary>
    /// Captures the current ambient tenant and subject identity, or
    /// returns <c>null</c> when no context is active and the job should
    /// run context-free.
    /// </summary>
    /// <returns>The captured snapshot, or <c>null</c> when no context is active.</returns>
    JobContextSnapshot? Capture();

    /// <summary>
    /// Restores the supplied <paramref name="snapshot"/> as the ambient
    /// context for the job execution. The returned disposable MUST undo
    /// the restoration; the adapter disposes it when the job scope ends.
    /// </summary>
    /// <param name="snapshot">The snapshot captured at job creation.</param>
    /// <returns>A disposable that undoes the restoration.</returns>
    IDisposable Restore(JobContextSnapshot snapshot);
}
