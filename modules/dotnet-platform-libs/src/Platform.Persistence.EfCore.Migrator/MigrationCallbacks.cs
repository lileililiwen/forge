using Microsoft.EntityFrameworkCore;

namespace Platform.Persistence.EfCore.Migrator;

/// <summary>
/// Application-owned seed seam. Invoked after a successful apply when the
/// request enables seeding. The platform supplies the migrated
/// <see cref="DbContext"/>; seed data, tenant iteration, and idempotency
/// remain application-owned.
/// </summary>
public interface IMigrationSeeder
{
    /// <summary>Seeds the migrated database.</summary>
    /// <param name="context">The migrated application context.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    Task SeedAsync(DbContext context, CancellationToken cancellationToken = default);
}

/// <summary>
/// Application-owned exclusive-execution seam. When a request supplies an
/// executor, the whole migration operation (inspection or apply plus
/// optional seed) runs inside it. Consumers back this with PostgreSQL
/// advisory locks, deployment locks, or test doubles; the platform never
/// acquires a lock itself and imposes none when the seam is absent.
/// </summary>
public interface IMigrationExclusiveExecutor
{
    /// <summary>Executes the operation with exclusive access.</summary>
    /// <typeparam name="TResult">The operation result type.</typeparam>
    /// <param name="operation">The migration operation.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <returns>The operation result.</returns>
    Task<TResult> ExecuteExclusiveAsync<TResult>(
        Func<CancellationToken, Task<TResult>> operation,
        CancellationToken cancellationToken = default);
}
