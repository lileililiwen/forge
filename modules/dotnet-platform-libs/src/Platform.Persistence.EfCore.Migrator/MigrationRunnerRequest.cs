using Microsoft.EntityFrameworkCore;

namespace Platform.Persistence.EfCore.Migrator;

/// <summary>
/// Application-owned migration request. The platform never creates
/// contexts, connection strings, migrations, locks, or seed data; every
/// operational choice arrives through this request.
/// </summary>
/// <param name="CreateContext">Factory that creates the application-owned context. Must return a non-null instance; the runner disposes it.</param>
/// <param name="SeedAfterApply">When true and <see cref="Seeder"/> is supplied, the seeder runs after a successful apply.</param>
/// <param name="Seeder">Optional application seed callback.</param>
/// <param name="ExclusiveExecutor">Optional application exclusive-execution callback. Absent means no lock is taken.</param>
public sealed record MigrationRunnerRequest(
    Func<CancellationToken, Task<DbContext>> CreateContext,
    bool SeedAfterApply = false,
    IMigrationSeeder? Seeder = null,
    IMigrationExclusiveExecutor? ExclusiveExecutor = null);
