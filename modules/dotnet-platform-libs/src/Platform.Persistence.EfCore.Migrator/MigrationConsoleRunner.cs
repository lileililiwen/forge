namespace Platform.Persistence.EfCore.Migrator;

/// <summary>
/// Thin console-host adapter over <see cref="IMigrationRunner"/>. Owns
/// argument parsing and process exit codes only; connection strings,
/// context factories, locks, and seed implementations stay
/// application-owned through the <see cref="MigrationRunnerRequest"/>
/// produced by the caller-supplied factory. Only secret-free categories
/// and diagnostics are written to the console.
/// </summary>
public static class MigrationConsoleRunner
{
    private const int SuccessCode = 0;
    private const int FailureCode = 1;
    private const int UsageCode = 2;

    /// <summary>Runs the migration command and returns the process exit code.</summary>
    /// <param name="args">The raw console arguments.</param>
    /// <param name="configure">Builds the application-owned request for the parsed command.</param>
    /// <param name="runner">The migration runner. Defaults to <see cref="MigrationRunner"/>.</param>
    /// <param name="output">The output writer. Defaults to <see cref="Console.Out"/>.</param>
    /// <param name="error">The error writer. Defaults to <see cref="Console.Error"/>.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <returns>0 on success or help, 1 on migration/seed/configuration/cancellation failure, 2 on usage error.</returns>
    public static async Task<int> RunAsync(
        string[] args,
        Func<MigrationCommand, MigrationRunnerRequest> configure,
        IMigrationRunner? runner = null,
        TextWriter? output = null,
        TextWriter? error = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(args);
        ArgumentNullException.ThrowIfNull(configure);
        output ??= Console.Out;
        error ??= Console.Error;

        var command = MigrationCommand.Parse(args);
        if (command.Help)
        {
            await output.WriteLineAsync(MigrationCommand.HelpText).ConfigureAwait(false);
            return SuccessCode;
        }

        if (!command.IsValid)
        {
            await error.WriteLineAsync($"Unknown verb '{command.Verb}'.").ConfigureAwait(false);
            await error.WriteLineAsync(MigrationCommand.HelpText).ConfigureAwait(false);
            return UsageCode;
        }

        MigrationRunnerRequest request;
        try
        {
            request = configure(command);
            if (request is null)
            {
                throw new InvalidOperationException("The migration request factory returned no request.");
            }
        }
        catch (Exception exception) when (exception is not OperationCanceledException)
        {
            await error.WriteLineAsync(
                $"Migration configuration failed ({exception.GetType().Name}).").ConfigureAwait(false);
            return FailureCode;
        }

        try
        {
            if (string.Equals(command.Verb, MigrationCommand.ListPending, StringComparison.Ordinal))
            {
                return await ListPendingAsync(runner ?? new MigrationRunner(), request, output, error, cancellationToken)
                    .ConfigureAwait(false);
            }

            return await ApplyAsync(
                    runner ?? new MigrationRunner(),
                    request with { SeedAfterApply = request.SeedAfterApply || command.Seed },
                    output,
                    error,
                    cancellationToken)
                .ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            await error.WriteLineAsync("Migration was canceled.").ConfigureAwait(false);
            return FailureCode;
        }
    }

    private static async Task<int> ListPendingAsync(
        IMigrationRunner runner,
        MigrationRunnerRequest request,
        TextWriter output,
        TextWriter error,
        CancellationToken cancellationToken)
    {
        var result = await runner.ListPendingAsync(request, cancellationToken).ConfigureAwait(false);
        if (!result.Succeeded)
        {
            await WriteFailureAsync(error, result.Failure).ConfigureAwait(false);
            return FailureCode;
        }

        await output.WriteLineAsync($"{result.PendingMigrations.Count} pending migration(s)").ConfigureAwait(false);
        foreach (var name in result.PendingMigrations)
        {
            await output.WriteLineAsync($"  {name}").ConfigureAwait(false);
        }

        return SuccessCode;
    }

    private static async Task<int> ApplyAsync(
        IMigrationRunner runner,
        MigrationRunnerRequest request,
        TextWriter output,
        TextWriter error,
        CancellationToken cancellationToken)
    {
        var result = await runner.ApplyAsync(request, cancellationToken).ConfigureAwait(false);
        if (!result.Succeeded)
        {
            await WriteFailureAsync(error, result.Failure).ConfigureAwait(false);
            return FailureCode;
        }

        await output.WriteLineAsync($"Applied {result.AppliedMigrations.Count} migration(s)").ConfigureAwait(false);
        if (result.SeedExecuted)
        {
            await output.WriteLineAsync("Seed completed").ConfigureAwait(false);
        }

        return SuccessCode;
    }

    private static Task WriteFailureAsync(TextWriter error, MigrationFailure? failure)
    {
        return failure is null
            ? error.WriteLineAsync("Migration failed.")
            : error.WriteLineAsync($"Migration failed: {failure.Category}: {failure.Diagnostic}");
    }
}
