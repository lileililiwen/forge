using Microsoft.EntityFrameworkCore;
using Platform.Persistence.EfCore.Migrator;

namespace Platform.Persistence.EfCore.Migrator.Tests;

public class MigrationConsoleRunnerTests
{
    [Fact]
    public void Command_parsing_supports_verbs_flags_and_help()
    {
        Assert.Equal("apply", MigrationCommand.Parse([]).Verb);
        Assert.Equal("apply", MigrationCommand.Parse(["--seed"]).Verb);
        Assert.True(MigrationCommand.Parse(["--seed"]).Seed);
        Assert.Equal("list-pending", MigrationCommand.Parse(["LIST-PENDING"]).Verb);
        Assert.True(MigrationCommand.Parse(["-h"]).Help);
        Assert.True(MigrationCommand.Parse(["--help"]).Help);
        Assert.False(MigrationCommand.Parse(["explode"]).IsValid);
        Assert.Throws<ArgumentNullException>(() => MigrationCommand.Parse(null!));
    }

    [Fact]
    public async Task Help_returns_zero_with_usage()
    {
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            ["--help"], _ => throw new InvalidOperationException("must not configure"), output: output, error: error);

        Assert.Equal(0, code);
        Assert.Contains("Usage", output.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public async Task Unknown_verb_returns_usage_error()
    {
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            ["explode"], _ => throw new InvalidOperationException("must not configure"), output: output, error: error);

        Assert.Equal(2, code);
        Assert.Contains("explode", error.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public async Task List_pending_prints_identifiers_and_returns_zero()
    {
        var runner = new StubRunner(
            pending: MigrationPendingResult.Success(["a", "b"]),
            run: MigrationRunResult.Success([], seedExecuted: false));
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            ["list-pending"], _ => TestRequest(), runner, output, error);

        Assert.Equal(0, code);
        Assert.Contains("2 pending", output.ToString(), StringComparison.Ordinal);
        Assert.Contains("a", output.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public async Task Failed_inspection_returns_one_with_redacted_diagnostic()
    {
        var runner = new StubRunner(
            pending: MigrationPendingResult.Failed(Unavailable()),
            run: MigrationRunResult.Success([], seedExecuted: false));
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            ["list-pending"], _ => TestRequest(), runner, output, error);

        Assert.Equal(1, code);
        Assert.Contains("Unavailable", error.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public async Task Apply_reports_counts_and_seed_state()
    {
        var runner = new StubRunner(
            pending: MigrationPendingResult.Success([]),
            run: MigrationRunResult.Success(["a"], seedExecuted: true));
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            ["apply", "--seed"], command => TestRequest(), runner, output, error);

        Assert.Equal(0, code);
        Assert.Contains("Applied 1", output.ToString(), StringComparison.Ordinal);
        Assert.Contains("Seed completed", output.ToString(), StringComparison.Ordinal);
        Assert.True(runner.LastApplyRequest!.SeedAfterApply);
    }

    [Fact]
    public async Task Seed_flag_enables_seed_even_when_request_disables_it()
    {
        var runner = new StubRunner(
            pending: MigrationPendingResult.Success([]),
            run: MigrationRunResult.Success([], seedExecuted: true));
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            ["--seed"], _ => TestRequest(seed: false), runner, output, error);

        Assert.Equal(0, code);
        Assert.True(runner.LastApplyRequest!.SeedAfterApply);
    }

    [Fact]
    public async Task Failed_apply_returns_one_without_secrets()
    {
        var runner = new StubRunner(
            pending: MigrationPendingResult.Success([]),
            run: MigrationRunResult.Failed(new MigrationFailure(
                MigrationFailureCategory.MigrationFailed, "Unable to apply pending migrations.", "SqliteException")));
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            [], _ => TestRequest(), runner, output, error);

        Assert.Equal(1, code);
        Assert.Contains("MigrationFailed", error.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public async Task Throwing_configuration_returns_one_without_leaking_messages()
    {
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            [], _ => throw new InvalidOperationException("connection Password=topsecret blew up"),
            output: output, error: error);

        Assert.Equal(1, code);
        Assert.DoesNotContain("topsecret", error.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public async Task Cancellation_returns_one()
    {
        var runner = new CancelRunner();
        var output = new StringWriter();
        var error = new StringWriter();

        var code = await MigrationConsoleRunner.RunAsync(
            [], _ => TestRequest(), runner, output, error);

        Assert.Equal(1, code);
        Assert.Contains("canceled", error.ToString(), StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Runner_rejects_null_arguments()
    {
        await Assert.ThrowsAsync<ArgumentNullException>(() =>
            MigrationConsoleRunner.RunAsync(null!, _ => TestRequest()));
        await Assert.ThrowsAsync<ArgumentNullException>(() =>
            MigrationConsoleRunner.RunAsync([], null!));
    }

    private static MigrationRunnerRequest TestRequest(bool seed = false)
    {
        return new MigrationRunnerRequest(
            _ => Task.FromResult<DbContext>(null!),
            SeedAfterApply: seed);
    }

    private static MigrationFailure Unavailable()
    {
        return new MigrationFailure(MigrationFailureCategory.Unavailable, "Database connection is unavailable.");
    }

    private sealed class StubRunner(
        MigrationPendingResult pending,
        MigrationRunResult run) : IMigrationRunner
    {
        public MigrationRunnerRequest? LastApplyRequest { get; private set; }

        public Task<MigrationPendingResult> ListPendingAsync(
            MigrationRunnerRequest request,
            CancellationToken cancellationToken = default)
        {
            return Task.FromResult(pending);
        }

        public Task<MigrationRunResult> ApplyAsync(
            MigrationRunnerRequest request,
            CancellationToken cancellationToken = default)
        {
            LastApplyRequest = request;
            return Task.FromResult(run);
        }
    }

    private sealed class CancelRunner : IMigrationRunner
    {
        public Task<MigrationPendingResult> ListPendingAsync(
            MigrationRunnerRequest request,
            CancellationToken cancellationToken = default)
        {
            throw new OperationCanceledException();
        }

        public Task<MigrationRunResult> ApplyAsync(
            MigrationRunnerRequest request,
            CancellationToken cancellationToken = default)
        {
            throw new OperationCanceledException();
        }
    }
}
