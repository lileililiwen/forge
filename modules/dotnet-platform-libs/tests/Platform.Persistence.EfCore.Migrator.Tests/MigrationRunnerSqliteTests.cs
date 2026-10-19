using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;
using Platform.Persistence.EfCore.Migrator;

namespace Platform.Persistence.EfCore.Migrator.Tests;

public sealed class MigrationRunnerSqliteTests : IDisposable
{
    private static readonly MigrationRunner Runner = new();
    private readonly string _databasePath = Path.Combine(
        Path.GetTempPath(), $"migrator-{Guid.NewGuid():N}.sqlite");

    [Fact]
    public async Task ListPending_returns_identifiers_without_modifying_the_database()
    {
        var request = new MigrationRunnerRequest(CreateContext);

        var result = await Runner.ListPendingAsync(request);

        Assert.True(result.Succeeded);
        Assert.Equal(["20260101000000_CreateNotes"], result.PendingMigrations);
        Assert.Equal(0, CountTables("Notes"));
        Assert.Equal(0, CountTables("__EFMigrationsHistory"));
    }

    [Fact]
    public async Task Apply_migrates_to_head_and_later_inspection_is_empty()
    {
        var request = new MigrationRunnerRequest(CreateContext);

        var applied = await Runner.ApplyAsync(request);
        var pending = await Runner.ListPendingAsync(request);

        Assert.True(applied.Succeeded);
        Assert.Equal(["20260101000000_CreateNotes"], applied.AppliedMigrations);
        Assert.False(applied.SeedExecuted);
        Assert.True(pending.Succeeded);
        Assert.Empty(pending.PendingMigrations);
        Assert.Equal(1, CountTables("Notes"));
    }

    [Fact]
    public async Task Apply_runs_the_seed_callback_inside_exclusive_execution()
    {
        var events = new List<string>();
        var seeder = new InsertSeeder(events);
        var request = new MigrationRunnerRequest(
            CreateContext,
            SeedAfterApply: true,
            Seeder: seeder,
            ExclusiveExecutor: new OrderedExecutor(events));

        var result = await Runner.ApplyAsync(request);

        Assert.True(result.Succeeded);
        Assert.True(result.SeedExecuted);
        Assert.Equal(["enter", "seed", "exit"], events);
        await using var context = CreateSampleContext();
        Assert.Equal("seeded", (await context.Notes.SingleAsync()).Title);
    }

    [Fact]
    public async Task Apply_runs_seed_even_when_already_at_head()
    {
        var first = await Runner.ApplyAsync(new MigrationRunnerRequest(CreateContext));
        var seeder = new InsertSeeder(new List<string>());
        Assert.True(first.Succeeded);

        var second = await Runner.ApplyAsync(new MigrationRunnerRequest(
            CreateContext, SeedAfterApply: true, Seeder: seeder));

        Assert.True(second.Succeeded);
        Assert.Empty(second.AppliedMigrations);
        Assert.True(second.SeedExecuted);
    }

    [Fact]
    public async Task Failing_seed_reports_seed_failed_without_leaking_the_cause()
    {
        const string secret = "seed-secret-value";
        var request = new MigrationRunnerRequest(
            CreateContext,
            SeedAfterApply: true,
            Seeder: new ThrowingSeeder(secret));

        var result = await Runner.ApplyAsync(request);

        Assert.False(result.Succeeded);
        Assert.Equal(MigrationFailureCategory.SeedFailed, result.Failure!.Category);
        Assert.DoesNotContain(secret, result.Failure.Diagnostic, StringComparison.Ordinal);
        Assert.Equal(1, CountTables("Notes"));
    }

    [Fact]
    public async Task Unusable_database_reports_unavailable_without_leaking_the_path()
    {
        await File.WriteAllBytesAsync(_databasePath, [0x00, 0x01, 0x02, 0x03, 0x04]);
        var marker = Path.GetFileName(_databasePath);
        var request = new MigrationRunnerRequest(CreateContext);

        var pending = await Runner.ListPendingAsync(request);
        var applied = await Runner.ApplyAsync(request);

        Assert.False(pending.Succeeded);
        Assert.Equal(MigrationFailureCategory.Unavailable, pending.Failure!.Category);
        Assert.False(applied.Succeeded);
        Assert.Equal(MigrationFailureCategory.Unavailable, applied.Failure!.Category);
        foreach (var failure in new[] { pending.Failure, applied.Failure })
        {
            Assert.DoesNotContain(marker, failure.Diagnostic, StringComparison.Ordinal);
        }
    }

    public void Dispose()
    {
        try
        {
            if (File.Exists(_databasePath))
            {
                File.Delete(_databasePath);
            }
        }
        catch (IOException)
        {
            // Best effort: a locked test database must not fail the suite.
        }
    }

    private Task<DbContext> CreateContext(CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return Task.FromResult<DbContext>(CreateSampleContext());
    }

    private MigratorSampleDbContext CreateSampleContext()
    {
        return new MigratorSampleDbContext(new DbContextOptionsBuilder<MigratorSampleDbContext>()
            .UseSqlite($"Data Source={_databasePath}")
            .Options);
    }

    private int CountTables(string name)
    {
        using var connection = new SqliteConnection($"Data Source={_databasePath}");
        connection.Open();
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = $name";
        command.Parameters.AddWithValue("$name", name);
        return Convert.ToInt32(command.ExecuteScalar());
    }

    private sealed class InsertSeeder(List<string> events) : IMigrationSeeder
    {
        public async Task SeedAsync(DbContext context, CancellationToken cancellationToken = default)
        {
            events.Add("seed");
            var sample = (MigratorSampleDbContext)context;
            sample.Notes.Add(new Note { Title = "seeded" });
            await sample.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
        }
    }

    private sealed class ThrowingSeeder(string secret) : IMigrationSeeder
    {
        public Task SeedAsync(DbContext context, CancellationToken cancellationToken = default)
        {
            throw new InvalidOperationException($"seed exploded: {secret}; DROP TABLE Notes;");
        }
    }

    private sealed class OrderedExecutor(List<string> events) : IMigrationExclusiveExecutor
    {
        public async Task<TResult> ExecuteExclusiveAsync<TResult>(
            Func<CancellationToken, Task<TResult>> operation,
            CancellationToken cancellationToken = default)
        {
            events.Add("enter");
            try
            {
                return await operation(cancellationToken).ConfigureAwait(false);
            }
            finally
            {
                events.Add("exit");
            }
        }
    }
}
