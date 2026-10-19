namespace Platform.Persistence.EfCore.Migrator;

/// <summary>
/// Stable failure categories for migration operations. Categories are safe
/// to log, display, and return from console hosts; they never carry
/// connection strings, SQL, exception messages, or provider response
/// bodies.
/// </summary>
public enum MigrationFailureCategory
{
    /// <summary>The database could not be reached or inspected.</summary>
    Unavailable = 0,

    /// <summary>Pending migrations could not be applied.</summary>
    MigrationFailed = 1,

    /// <summary>Migrations applied but the application seed callback failed.</summary>
    SeedFailed = 2,
}
