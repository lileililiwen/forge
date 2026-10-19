namespace Platform.Persistence.EfCore.Migrator;

/// <summary>Outcome of the read-only pending-migration inspection. The database is never modified.</summary>
/// <param name="Succeeded">Whether inspection completed.</param>
/// <param name="PendingMigrations">The pending migration identifiers, in apply order. Empty when inspection failed.</param>
/// <param name="Failure">The secret-free failure, or null on success.</param>
public sealed record MigrationPendingResult(
    bool Succeeded,
    IReadOnlyList<string> PendingMigrations,
    MigrationFailure? Failure = null)
{
    /// <summary>Creates a successful inspection result.</summary>
    public static MigrationPendingResult Success(IReadOnlyList<string> pendingMigrations)
    {
        ArgumentNullException.ThrowIfNull(pendingMigrations);
        return new MigrationPendingResult(true, pendingMigrations);
    }

    /// <summary>Creates a failed inspection result.</summary>
    public static MigrationPendingResult Failed(MigrationFailure failure)
    {
        ArgumentNullException.ThrowIfNull(failure);
        return new MigrationPendingResult(false, [], failure);
    }
}

/// <summary>Outcome of applying pending migrations with an optional seed step.</summary>
/// <param name="Succeeded">Whether apply (and the requested seed) completed.</param>
/// <param name="AppliedMigrations">The pending identifiers captured before apply; these were applied on success. Empty on failure.</param>
/// <param name="SeedExecuted">Whether the application seed callback ran to completion.</param>
/// <param name="Failure">The secret-free failure, or null on success.</param>
public sealed record MigrationRunResult(
    bool Succeeded,
    IReadOnlyList<string> AppliedMigrations,
    bool SeedExecuted,
    MigrationFailure? Failure = null)
{
    /// <summary>Creates a successful apply result.</summary>
    public static MigrationRunResult Success(IReadOnlyList<string> appliedMigrations, bool seedExecuted)
    {
        ArgumentNullException.ThrowIfNull(appliedMigrations);
        return new MigrationRunResult(true, appliedMigrations, seedExecuted);
    }

    /// <summary>Creates a failed apply result.</summary>
    public static MigrationRunResult Failed(MigrationFailure failure)
    {
        ArgumentNullException.ThrowIfNull(failure);
        return new MigrationRunResult(false, [], false, failure);
    }
}
