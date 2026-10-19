namespace Platform.Persistence.Postgres.Testing;

/// <summary>Options for <see cref="PostgresTestServer"/>.</summary>
public sealed class PostgresTestServerOptions
{
    /// <summary>Default pinned PostgreSQL image (patch-level, never floating).</summary>
    public const string DefaultImage = "postgres:17.6-alpine";

    /// <summary>Container image. Must carry an explicit non-<c>latest</c> tag.</summary>
    public string Image { get; set; } = DefaultImage;

    /// <summary>Superuser name created in the container.</summary>
    public string Username { get; set; } = "postgres";

    /// <summary>Superuser password. Override per run when needed.</summary>
    public string Password { get; set; } = "postgres";

    /// <summary>Maintenance database used for administrative commands.</summary>
    public string MaintenanceDatabase { get; set; } = "postgres";
}
