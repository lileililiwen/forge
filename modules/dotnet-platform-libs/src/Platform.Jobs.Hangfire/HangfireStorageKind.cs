namespace Platform.Jobs.Hangfire;

/// <summary>
/// The storage backends the adapter can wire when the host opts in. The
/// application selects the kind and supplies any required configuration;
/// the platform never owns connection strings or credentials.
/// </summary>
public enum HangfireStorageKind
{
    /// <summary>The official in-process, non-durable Hangfire storage. Useful for development and tests.</summary>
    InMemory,

    /// <summary>PostgreSQL-backed Hangfire storage. Requires an application-supplied connection string.</summary>
    PostgreSql,
}
