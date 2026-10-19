using Microsoft.EntityFrameworkCore;
using System.Data.Common;

namespace Platform.Persistence.EfCore.Migrations;

/// <summary>Reads status from an EF Core context without changing the database.</summary>
public sealed class EfCoreMigrationStatusReader(DbContext dbContext) : IMigrationStatusReader
{
    /// <inheritdoc />
    public async Task<MigrationStatus> GetStatusAsync(CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(dbContext);
        try
        {
            if (!await dbContext.Database.CanConnectAsync(cancellationToken))
                return new MigrationStatus(false, false, 0, "Database connection is unavailable.");
            if (!dbContext.Database.IsRelational())
                return new MigrationStatus(true, true, 0);
            var pending = (await dbContext.Database.GetPendingMigrationsAsync(cancellationToken)).Count();
            return new MigrationStatus(true, pending == 0, pending);
        }
        catch (Exception exception) when (exception is DbException or InvalidOperationException)
        {
            return new MigrationStatus(false, false, 0, exception.Message);
        }
    }
}
