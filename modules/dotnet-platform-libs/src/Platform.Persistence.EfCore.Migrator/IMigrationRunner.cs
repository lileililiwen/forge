namespace Platform.Persistence.EfCore.Migrator;

/// <summary>
/// Application-owned migration execution boundary. Operates only on the
/// context supplied by the request factory; never creates platform-owned
/// contexts, migrations, connection strings, locks, tenant iteration, or
/// seed data. Cancellation is honored by propagating
/// <see cref="OperationCanceledException"/> rather than returning a
/// failure result.
/// </summary>
public interface IMigrationRunner
{
    /// <summary>Returns the pending migration identifiers without modifying the database.</summary>
    /// <param name="request">The application-owned request.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    Task<MigrationPendingResult> ListPendingAsync(
        MigrationRunnerRequest request,
        CancellationToken cancellationToken = default);

    /// <summary>Applies pending migrations and runs the optional seed callback.</summary>
    /// <param name="request">The application-owned request.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    Task<MigrationRunResult> ApplyAsync(
        MigrationRunnerRequest request,
        CancellationToken cancellationToken = default);
}
