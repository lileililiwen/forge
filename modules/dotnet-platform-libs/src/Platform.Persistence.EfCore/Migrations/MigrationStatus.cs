namespace Platform.Persistence.EfCore.Migrations;

/// <summary>Read-only database migration and connection status.</summary>
public sealed record MigrationStatus(bool IsAvailable, bool IsReady, int PendingMigrationCount, string? Error = null);

/// <summary>Reads migration state without applying migrations.</summary>
public interface IMigrationStatusReader
{
    /// <summary>Gets current connection and migration status.</summary>
    Task<MigrationStatus> GetStatusAsync(CancellationToken cancellationToken = default);
}
