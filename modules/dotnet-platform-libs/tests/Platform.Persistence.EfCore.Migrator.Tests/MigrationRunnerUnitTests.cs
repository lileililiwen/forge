using Microsoft.EntityFrameworkCore;
using Platform.Persistence.EfCore.Migrator;

namespace Platform.Persistence.EfCore.Migrator.Tests;

public class MigrationRunnerUnitTests
{
    private static readonly MigrationRunner Runner = new();

    [Fact]
    public async Task Operations_reject_null_requests()
    {
        await Assert.ThrowsAsync<ArgumentNullException>(() => Runner.ListPendingAsync(null!));
        await Assert.ThrowsAsync<ArgumentNullException>(() => Runner.ApplyAsync(null!));
    }

    [Fact]
    public async Task Operations_reject_missing_factories()
    {
        var request = new MigrationRunnerRequest(null!);

        await Assert.ThrowsAsync<ArgumentException>(() => Runner.ListPendingAsync(request));
        await Assert.ThrowsAsync<ArgumentException>(() => Runner.ApplyAsync(request));
    }

    [Fact]
    public async Task Operations_propagate_cancellation()
    {
        using var canceled = new CancellationTokenSource();
        await canceled.CancelAsync();
        var request = new MigrationRunnerRequest(_ => Task.FromResult<DbContext>(CreateMemoryContext()));

        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => Runner.ListPendingAsync(request, canceled.Token));
        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => Runner.ApplyAsync(request, canceled.Token));
    }

    [Fact]
    public async Task Non_relational_context_is_unavailable_without_touching_seed_or_lock()
    {
        var seeder = new RecordingSeeder();
        var executor = new RecordingExecutor();
        var request = new MigrationRunnerRequest(
            _ => Task.FromResult<DbContext>(CreateMemoryContext()),
            SeedAfterApply: true,
            Seeder: seeder,
            ExclusiveExecutor: executor);

        var pending = await Runner.ListPendingAsync(request);
        var applied = await Runner.ApplyAsync(request);

        Assert.False(pending.Succeeded);
        Assert.Equal(MigrationFailureCategory.Unavailable, pending.Failure!.Category);
        Assert.False(applied.Succeeded);
        Assert.Equal(MigrationFailureCategory.Unavailable, applied.Failure!.Category);
        Assert.False(seeder.SeedCalled);
        Assert.Equal(2, executor.Invocations.Count);
    }

    [Fact]
    public async Task Exclusive_executor_wraps_the_operation_and_passes_the_result_through()
    {
        var executor = new RecordingExecutor();
        var inner = new List<string>();
        var request = new MigrationRunnerRequest(
            token =>
            {
                inner.Add("factory");
                token.ThrowIfCancellationRequested();
                return Task.FromResult<DbContext>(CreateMemoryContext());
            },
            ExclusiveExecutor: executor);

        var result = await Runner.ListPendingAsync(request);

        Assert.False(result.Succeeded);
        Assert.Single(executor.Invocations);
        Assert.Single(inner);
    }

    [Fact]
    public async Task Throwing_factory_fails_fast_without_leaking_instance_data()
    {
        const string secret = "Server=secret-host;Password=hunter2";
        var request = new MigrationRunnerRequest(
            _ => throw new InvalidOperationException($"connect {secret} failed; SELECT * FROM notes"));

        var exception = await Assert.ThrowsAsync<InvalidOperationException>(
            () => Runner.ListPendingAsync(request));

        Assert.DoesNotContain("secret-host", exception.Message, StringComparison.Ordinal);
        Assert.DoesNotContain("hunter2", exception.Message, StringComparison.Ordinal);
        Assert.DoesNotContain("SELECT", exception.Message, StringComparison.Ordinal);
        Assert.Contains(nameof(InvalidOperationException), exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Null_factory_result_fails_fast()
    {
        var request = new MigrationRunnerRequest(_ => Task.FromResult<DbContext>(null!));

        var exception = await Assert.ThrowsAsync<InvalidOperationException>(
            () => Runner.ApplyAsync(request));

        Assert.Contains("no context", exception.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Result_factories_reject_null_arguments()
    {
        Assert.Throws<ArgumentNullException>(() => MigrationPendingResult.Success(null!));
        Assert.Throws<ArgumentNullException>(() => MigrationPendingResult.Failed(null!));
        Assert.Throws<ArgumentNullException>(() => MigrationRunResult.Success(null!, seedExecuted: false));
        Assert.Throws<ArgumentNullException>(() => MigrationRunResult.Failed(null!));
    }

    private static DbContext CreateMemoryContext()
    {
        return new DbContext(new DbContextOptionsBuilder()
            .UseInMemoryDatabase(Guid.NewGuid().ToString("N"))
            .Options);
    }

    private sealed class RecordingSeeder : IMigrationSeeder
    {
        public bool SeedCalled { get; private set; }

        public Task SeedAsync(DbContext context, CancellationToken cancellationToken = default)
        {
            SeedCalled = true;
            return Task.CompletedTask;
        }
    }

    private sealed class RecordingExecutor : IMigrationExclusiveExecutor
    {
        public List<string> Invocations { get; } = [];

        public async Task<TResult> ExecuteExclusiveAsync<TResult>(
            Func<CancellationToken, Task<TResult>> operation,
            CancellationToken cancellationToken = default)
        {
            Invocations.Add("exclusive");
            return await operation(cancellationToken).ConfigureAwait(false);
        }
    }
}
