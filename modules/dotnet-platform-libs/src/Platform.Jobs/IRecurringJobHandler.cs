namespace Platform.Jobs;

/// <summary>
/// Marker interface implemented by recurring-job handlers. The
/// dispatcher resolves a registered handler through
/// <see cref="IRecurringJobRegistry"/> and invokes
/// <see cref="ExecuteAsync"/>.
/// </summary>
public interface IRecurringJobHandler
{
    /// <summary>
    /// Executes the recurring job. Implementations MUST honour the
    /// supplied <paramref name="cancellationToken"/>.
    /// </summary>
    /// <param name="cancellationToken">The cancellation token.</param>
    Task ExecuteAsync(CancellationToken cancellationToken);
}
