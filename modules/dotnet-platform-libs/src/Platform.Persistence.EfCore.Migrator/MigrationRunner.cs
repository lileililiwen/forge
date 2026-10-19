using System.Data.Common;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Infrastructure;
using Microsoft.EntityFrameworkCore.Migrations;
using Microsoft.EntityFrameworkCore.Storage;

namespace Platform.Persistence.EfCore.Migrator;

/// <summary>
/// Default <see cref="IMigrationRunner"/> implementation. Executes
/// pending inspection and apply against the application-supplied context,
/// routes the whole operation through the optional exclusive-execution
/// callback, and classifies failures into secret-free results.
/// </summary>
public sealed class MigrationRunner : IMigrationRunner
{
    private const string UnavailableDiagnostic = "Database connection is unavailable.";
    private const string InspectionDiagnostic = "Unable to read pending migrations.";
    private const string NonRelationalDiagnostic = "The supplied context does not use a relational provider.";
    private const string ApplyDiagnostic = "Unable to apply pending migrations.";
    private const string SeedDiagnostic = "Migration seed callback failed.";

    /// <inheritdoc />
    public async Task<MigrationPendingResult> ListPendingAsync(
        MigrationRunnerRequest request,
        CancellationToken cancellationToken = default)
    {
        Validate(request);
        cancellationToken.ThrowIfCancellationRequested();
        if (request.ExclusiveExecutor is { } executor)
        {
            return await executor
                .ExecuteExclusiveAsync(token => ListPendingCoreAsync(request, token), cancellationToken)
                .ConfigureAwait(false);
        }

        return await ListPendingCoreAsync(request, cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async Task<MigrationRunResult> ApplyAsync(
        MigrationRunnerRequest request,
        CancellationToken cancellationToken = default)
    {
        Validate(request);
        cancellationToken.ThrowIfCancellationRequested();
        if (request.ExclusiveExecutor is { } executor)
        {
            return await executor
                .ExecuteExclusiveAsync(token => ApplyCoreAsync(request, token), cancellationToken)
                .ConfigureAwait(false);
        }

        return await ApplyCoreAsync(request, cancellationToken).ConfigureAwait(false);
    }

    private static async Task<MigrationPendingResult> ListPendingCoreAsync(
        MigrationRunnerRequest request,
        CancellationToken cancellationToken)
    {
        await using var context = await CreateContextAsync(request, cancellationToken).ConfigureAwait(false);
        var guard = EnsureRelational(context);
        if (guard is not null)
        {
            return MigrationPendingResult.Failed(guard);
        }

        var pending = await ReadPendingAsync(context, cancellationToken).ConfigureAwait(false);
        return pending.Succeeded
            ? MigrationPendingResult.Success(pending.Pending)
            : MigrationPendingResult.Failed(pending.Failure!);
    }

    private static async Task<MigrationRunResult> ApplyCoreAsync(
        MigrationRunnerRequest request,
        CancellationToken cancellationToken)
    {
        await using var context = await CreateContextAsync(request, cancellationToken).ConfigureAwait(false);
        var guard = EnsureRelational(context);
        if (guard is not null)
        {
            return MigrationRunResult.Failed(guard);
        }

        var pending = await ReadPendingAsync(context, cancellationToken).ConfigureAwait(false);
        if (!pending.Succeeded)
        {
            return MigrationRunResult.Failed(pending.Failure!);
        }

        if (pending.Pending.Count > 0)
        {
            try
            {
                await context.Database.MigrateAsync(cancellationToken).ConfigureAwait(false);
            }
            catch (Exception exception) when (exception is not OperationCanceledException)
            {
                return MigrationRunResult.Failed(Classify(exception, ApplyDiagnostic));
            }
        }

        if (request is { SeedAfterApply: true, Seeder: not null })
        {
            try
            {
                await request.Seeder.SeedAsync(context, cancellationToken).ConfigureAwait(false);
            }
            catch (Exception exception) when (exception is not OperationCanceledException)
            {
                return MigrationRunResult.Failed(
                    new MigrationFailure(MigrationFailureCategory.SeedFailed, SeedDiagnostic, exception.GetType().Name));
            }

            return MigrationRunResult.Success(pending.Pending, seedExecuted: true);
        }

        return MigrationRunResult.Success(pending.Pending, seedExecuted: false);
    }

    /// <summary>
    /// Reads pending migrations provider-neutrally. A database that does
    /// not exist yet reports every defined migration as pending; only a
    /// genuinely unreachable database reports <see cref="MigrationFailureCategory.Unavailable"/>.
    /// </summary>
    private static async Task<(bool Succeeded, IReadOnlyList<string> Pending, MigrationFailure? Failure)> ReadPendingAsync(
        DbContext context,
        CancellationToken cancellationToken)
    {
        bool exists;
        try
        {
            exists = await context.Database
                .GetService<IRelationalDatabaseCreator>()
                .ExistsAsync(cancellationToken)
                .ConfigureAwait(false);
        }
        catch (Exception exception) when (exception is not OperationCanceledException)
        {
            return (false, [], Classify(exception, InspectionDiagnostic));
        }

        if (!exists)
        {
            return (true, DefinedMigrations(context), null);
        }

        try
        {
            var pending = (await context.Database
                .GetPendingMigrationsAsync(cancellationToken)
                .ConfigureAwait(false)).ToArray();
            return (true, pending, null);
        }
        catch (Exception exception) when (exception is not OperationCanceledException)
        {
            return (false, [], Classify(exception, InspectionDiagnostic));
        }
    }

    private static string[] DefinedMigrations(DbContext context)
    {
        return context.GetService<IMigrationsAssembly>().Migrations
            .Select(migration => migration.Key)
            .OrderBy(id => id, StringComparer.Ordinal)
            .ToArray();
    }

    private static void Validate(MigrationRunnerRequest request)
    {
        ArgumentNullException.ThrowIfNull(request);
        if (request.CreateContext is null)
        {
            throw new ArgumentException("A context factory is required.", nameof(request));
        }
    }

    private static async Task<DbContext> CreateContextAsync(
        MigrationRunnerRequest request,
        CancellationToken cancellationToken)
    {
        DbContext? context;
        try
        {
            context = await request.CreateContext(cancellationToken).ConfigureAwait(false);
        }
        catch (Exception exception) when (exception is not OperationCanceledException)
        {
            throw new InvalidOperationException(
                $"The migration context factory failed with {exception.GetType().Name}.",
                exception);
        }

        return context ?? throw new InvalidOperationException("The migration context factory returned no context.");
    }

    private static MigrationFailure? EnsureRelational(DbContext context)
    {
        try
        {
            return context.Database.IsRelational() ? null : Unavailable(NonRelationalDiagnostic);
        }
        catch (Exception exception) when (exception is not OperationCanceledException)
        {
            return Classify(exception, InspectionDiagnostic);
        }
    }

    private static MigrationFailure Unavailable(string? diagnostic = null)
        => new(MigrationFailureCategory.Unavailable, diagnostic ?? UnavailableDiagnostic);

    private static MigrationFailure Classify(Exception exception, string diagnostic)
    {
        var category = exception is DbException or InvalidOperationException
            ? MigrationFailureCategory.Unavailable
            : MigrationFailureCategory.MigrationFailed;
        return new MigrationFailure(category, diagnostic, exception.GetType().Name);
    }
}
