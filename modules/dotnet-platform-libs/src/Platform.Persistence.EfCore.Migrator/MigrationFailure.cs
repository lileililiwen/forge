namespace Platform.Persistence.EfCore.Migrator;

/// <summary>
/// Secret-free migration failure. <see cref="Diagnostic"/> is a fixed
/// template naming the failure category and the exception type only;
/// it never contains connection strings, SQL, exception messages, or
/// provider response bodies.
/// </summary>
/// <param name="Category">The stable failure category.</param>
/// <param name="Diagnostic">The secret-free diagnostic text.</param>
/// <param name="ExceptionType">The CLR type name of the underlying exception, when one was caught.</param>
public sealed record MigrationFailure(
    MigrationFailureCategory Category,
    string Diagnostic,
    string? ExceptionType = null);
