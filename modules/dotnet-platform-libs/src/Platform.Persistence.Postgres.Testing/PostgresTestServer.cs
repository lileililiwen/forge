using System.Text.RegularExpressions;
using Npgsql;
using Testcontainers.PostgreSql;

namespace Platform.Persistence.Postgres.Testing;

/// <summary>
/// Assembly-shared PostgreSQL test server: boots one pinned-image container
/// per test assembly and provisions isolated databases by cloning a migrated
/// template database. Hold a single instance in an xUnit
/// <c>ICollectionFixture</c>; dispose it to stop and remove the container.
/// </summary>
public sealed partial class PostgresTestServer : IAsyncDisposable
{
    /// <summary>Default template database name used by <c>EnsureTemplateSeededAsync</c>.</summary>
    public const string DefaultTemplateDatabase = "platform_template";

    private readonly SemaphoreSlim _seedGate = new(1, 1);

    private readonly PostgreSqlContainer _container;
    private readonly string _host;
    private readonly int _port;
    private readonly string _username;
    private readonly string _password;
    private readonly string _maintenanceDatabase;
    private readonly HashSet<string> _seededTemplates = new(StringComparer.Ordinal);
    private bool _disposed;

    private PostgresTestServer(
        PostgreSqlContainer container,
        string host,
        int port,
        string username,
        string password,
        string maintenanceDatabase)
    {
        _container = container;
        _host = host;
        _port = port;
        _username = username;
        _password = password;
        _maintenanceDatabase = maintenanceDatabase;
    }

    /// <summary>Full container id of the backing PostgreSQL container.</summary>
    public string ContainerId => _container.Id ?? string.Empty;

    /// <summary>Docker name of the backing PostgreSQL container.</summary>
    public string ContainerName => _container.Name;

    /// <summary>Image the container was booted from (always explicitly tagged).</summary>
    public string Image { get; private set; } = PostgresTestServerOptions.DefaultImage;

    /// <summary>Connection string to the maintenance database on the real server.</summary>
    public string ServerConnectionString => BuildDatabaseConnectionString(_maintenanceDatabase);

    /// <summary>Whether at least one template database has been seeded.</summary>
    public bool IsTemplateSeeded => _seededTemplates.Count > 0;

    /// <summary>
    /// Boots one PostgreSQL container from a pinned image and probes it with
    /// <c>SHOW server_version_num</c>: a returned server has answered a real
    /// PostgreSQL instance, never an in-memory fake.
    /// </summary>
    public static async Task<PostgresTestServer> StartAsync(
        PostgresTestServerOptions? options = null,
        CancellationToken cancellationToken = default)
    {
        options ??= new PostgresTestServerOptions();
        var image = RequirePinnedImage(options.Image);
        ArgumentException.ThrowIfNullOrWhiteSpace(options.Username);
        ArgumentException.ThrowIfNullOrWhiteSpace(options.Password);
        ArgumentException.ThrowIfNullOrWhiteSpace(options.MaintenanceDatabase);

        var container = new PostgreSqlBuilder(image)
            .WithUsername(options.Username)
            .WithPassword(options.Password)
            .WithDatabase(options.MaintenanceDatabase)
            .Build();

        try
        {
            await container.StartAsync(cancellationToken).ConfigureAwait(false);
            var server = new PostgresTestServer(
                container,
                container.Hostname,
                container.GetMappedPublicPort(PostgreSqlBuilder.PostgreSqlPort),
                options.Username,
                options.Password,
                options.MaintenanceDatabase)
            {
                Image = image,
            };
            await server.ProveRealServerAsync(cancellationToken).ConfigureAwait(false);
            return server;
        }
        catch
        {
            await container.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }

    /// <summary>Builds a connection string for the named database on this server.</summary>
    public string BuildDatabaseConnectionString(string database)
    {
        var name = RequireDatabaseName(database);
        return new NpgsqlConnectionStringBuilder
        {
            Host = _host,
            Port = _port,
            Database = name,
            Username = _username,
            Password = _password,
        }.ConnectionString;
    }

    /// <summary>
    /// Creates the template database when missing and runs
    /// <paramref name="migrateTemplateAsync"/> against it exactly once per
    /// template name, even under concurrent calls. The delegate receives the
    /// template connection string and must dispose its connections before
    /// returning so later clones are not blocked.
    /// </summary>
    public async Task EnsureTemplateSeededAsync(
        Func<string, CancellationToken, Task> migrateTemplateAsync,
        string templateDatabase = DefaultTemplateDatabase,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(migrateTemplateAsync);
        var template = RequireDatabaseName(templateDatabase);

        await _seedGate.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (!_seededTemplates.Add(template))
            {
                return;
            }

            await using var admin = OpenMaintenanceConnection();
            await admin.OpenAsync(cancellationToken).ConfigureAwait(false);
            if (!await DatabaseExistsAsync(admin, template, cancellationToken).ConfigureAwait(false))
            {
                await ExecuteAdminAsync(admin, $"CREATE DATABASE {QuoteIdentifier(template)}", cancellationToken).ConfigureAwait(false);
            }

            try
            {
                await migrateTemplateAsync(BuildDatabaseConnectionString(template), cancellationToken).ConfigureAwait(false);
            }
            catch
            {
                _seededTemplates.Remove(template);
                throw;
            }
        }
        finally
        {
            _seedGate.Release();
        }
    }

    /// <summary>
    /// Creates an isolated database and returns its connection string. When a
    /// template has been seeded, the database is cloned from it
    /// (<c>CREATE DATABASE ... TEMPLATE</c>) so the test observes the migrated
    /// schema without replaying migrations; otherwise an empty database is
    /// created. The name is caller-chosen when supplied, otherwise unique.
    /// </summary>
    public async Task<string> CreateIsolatedDatabaseAsync(
        string? databaseName = null,
        CancellationToken cancellationToken = default)
    {
        var name = databaseName is null ? NewDatabaseName() : RequireDatabaseName(databaseName);
        var template = _seededTemplates.FirstOrDefault();

        await using var admin = OpenMaintenanceConnection();
        await admin.OpenAsync(cancellationToken).ConfigureAwait(false);
        if (template is null)
        {
            await ExecuteAdminAsync(admin, $"CREATE DATABASE {QuoteIdentifier(name)}", cancellationToken).ConfigureAwait(false);
        }
        else
        {
            await TerminateBackendsAsync(admin, template, cancellationToken).ConfigureAwait(false);
            await ExecuteAdminAsync(
                admin,
                $"CREATE DATABASE {QuoteIdentifier(name)} TEMPLATE {QuoteIdentifier(template)}",
                cancellationToken).ConfigureAwait(false);
        }

        return BuildDatabaseConnectionString(name);
    }

    /// <summary>Stops and removes the backing container.</summary>
    public async ValueTask DisposeAsync()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _seedGate.Dispose();
        await _container.DisposeAsync().ConfigureAwait(false);
    }

    internal static string RequirePinnedImage(string image)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(image);
        var repositoryAndTag = image.Split('/').Last();
        var tagIndex = repositoryAndTag.LastIndexOf(':');
        var tag = tagIndex < 0 ? string.Empty : repositoryAndTag[(tagIndex + 1)..];
        if (string.IsNullOrEmpty(tag) || tag.Equals("latest", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException(
                $"PostgreSQL test image '{image}' must carry an explicit non-latest tag (for example '{PostgresTestServerOptions.DefaultImage}').",
                nameof(image));
        }

        return image;
    }

    internal static string RequireDatabaseName(string database)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(database);
        if (!DatabaseNamePattern().IsMatch(database))
        {
            throw new ArgumentException(
                $"Database name '{database}' must be a safe PostgreSQL identifier (start with a letter or underscore, then letters, digits, underscore, or dollar sign; max 63 characters).",
                nameof(database));
        }

        return database;
    }

    private static string NewDatabaseName() => $"test_{Guid.NewGuid():N}";

    private static string QuoteIdentifier(string identifier) =>
        $"\"{identifier.Replace("\"", "\"\"", StringComparison.Ordinal)}\"";

    private NpgsqlConnection OpenMaintenanceConnection() =>
        new(new NpgsqlConnectionStringBuilder
        {
            Host = _host,
            Port = _port,
            Database = _maintenanceDatabase,
            Username = _username,
            Password = _password,
        }.ConnectionString);

    private async Task ProveRealServerAsync(CancellationToken cancellationToken)
    {
        await using var connection = OpenMaintenanceConnection();
        await connection.OpenAsync(cancellationToken).ConfigureAwait(false);
        await using var command = new NpgsqlCommand("SHOW server_version_num;", connection);
        var value = (await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false))?.ToString();
        if (!int.TryParse(value, out var version) || version <= 0)
        {
            throw new InvalidOperationException("The PostgreSQL test server did not report a server version; refusing to masquerade it as real.");
        }
    }

    private static async Task<bool> DatabaseExistsAsync(
        NpgsqlConnection admin,
        string database,
        CancellationToken cancellationToken)
    {
        await using var command = new NpgsqlCommand("SELECT 1 FROM pg_database WHERE datname = @name;", admin);
        command.Parameters.AddWithValue("name", database);
        return await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false) is not null;
    }

    private static async Task ExecuteAdminAsync(
        NpgsqlConnection admin,
        string sql,
        CancellationToken cancellationToken)
    {
        await using var command = new NpgsqlCommand(sql, admin);
        await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private static async Task TerminateBackendsAsync(
        NpgsqlConnection admin,
        string database,
        CancellationToken cancellationToken)
    {
        try
        {
            await using var command = new NpgsqlCommand(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = @name AND pid <> pg_backend_pid();",
                admin);
            command.Parameters.AddWithValue("name", database);
            await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
        }
        catch (NpgsqlException)
        {
            // Best effort: the clone itself surfaces a clear error when blocked.
        }
    }

    [GeneratedRegex("^[A-Za-z_][A-Za-z0-9_$]{0,62}$")]
    private static partial Regex DatabaseNamePattern();
}
