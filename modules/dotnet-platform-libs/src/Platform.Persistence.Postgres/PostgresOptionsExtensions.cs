using Microsoft.EntityFrameworkCore;

namespace Platform.Persistence.Postgres;

/// <summary>PostgreSQL-specific options setup kept outside the provider-neutral package.</summary>
public static class PostgresOptionsExtensions
{
    /// <summary>Configures Npgsql using the supplied connection string.</summary>
    public static DbContextOptionsBuilder UsePlatformPostgres(this DbContextOptionsBuilder builder, string connectionString)
    {
        ArgumentNullException.ThrowIfNull(builder);
        ArgumentException.ThrowIfNullOrWhiteSpace(connectionString);
        return builder.UseNpgsql(connectionString, options => options.MigrationsHistoryTable("__PlatformMigrations"));
    }

    /// <summary>Configures Npgsql on a typed context options builder.</summary>
    public static DbContextOptionsBuilder<TContext> UsePlatformPostgres<TContext>(this DbContextOptionsBuilder<TContext> builder, string connectionString)
        where TContext : DbContext
    {
        ArgumentNullException.ThrowIfNull(builder);
        ArgumentException.ThrowIfNullOrWhiteSpace(connectionString);
        builder.UseNpgsql(connectionString, options => options.MigrationsHistoryTable("__PlatformMigrations"));
        return builder;
    }
}
