using Npgsql;

namespace Platform.Persistence.Postgres.Tests;

/// <summary>
/// Docker-gated proofs for template seeding and per-test clone freshness.
/// </summary>
[Collection(PostgresServerCollection.Name)]
public sealed class TemplateCloneTests(SharedPostgresServerFixture fixture)
{
    private readonly SharedPostgresServerFixture _fixture = fixture;

    [Fact]
    public async Task Server_connection_reaches_a_real_postgresql_server()
    {
        _fixture.SkipIfDockerUnavailable();
        await using var connection = new NpgsqlConnection(_fixture.Server.ServerConnectionString);
        await connection.OpenAsync();
        await using var command = new NpgsqlCommand("SHOW server_version_num;", connection);
        var value = (await command.ExecuteScalarAsync())?.ToString();
        Assert.True(int.TryParse(value, out var version) && version > 0);
    }

    [Fact]
    public async Task Migration_runs_exactly_once_under_concurrent_and_sequential_seeding()
    {
        _fixture.SkipIfDockerUnavailable();
        var server = _fixture.Server;
        var migrationRuns = 0;
        Task Migrate(string connectionString, CancellationToken cancellationToken)
        {
            Interlocked.Increment(ref migrationRuns);
            return Task.CompletedTask;
        }

        await Task.WhenAll(
            server.EnsureTemplateSeededAsync(Migrate, "template_once_probe"),
            server.EnsureTemplateSeededAsync(Migrate, "template_once_probe"),
            server.EnsureTemplateSeededAsync(Migrate, "template_once_probe"));
        await server.EnsureTemplateSeededAsync(Migrate, "template_once_probe");

        Assert.Equal(1, migrationRuns);
        Assert.True(server.IsTemplateSeeded);
    }

    [Fact]
    public async Task Row_written_in_one_clone_is_invisible_in_the_next()
    {
        _fixture.SkipIfDockerUnavailable();
        var server = _fixture.Server;
        await server.EnsureTemplateSeededAsync(MigrateCloneProbeAsync, "template_clone_probe");

        var first = await server.CreateIsolatedDatabaseAsync();
        await ExecuteAsync(first, "INSERT INTO probe_widgets(id, note) VALUES (2, 'clone-a-row');");

        var second = await server.CreateIsolatedDatabaseAsync();
        var notes = await QueryAsync(second, "SELECT note FROM probe_widgets ORDER BY id;");

        Assert.Equal(["seed-row"], notes);
    }

    [Fact]
    public async Task Caller_chosen_names_are_preserved_and_default_names_are_unique()
    {
        _fixture.SkipIfDockerUnavailable();
        var server = _fixture.Server;
        await server.EnsureTemplateSeededAsync(MigrateCloneProbeAsync, "template_clone_probe");

        var chosen = await server.CreateIsolatedDatabaseAsync("chronicleaf_style_db");
        Assert.Equal("chronicleaf_style_db", new NpgsqlConnectionStringBuilder(chosen).Database);

        var first = await server.CreateIsolatedDatabaseAsync();
        var second = await server.CreateIsolatedDatabaseAsync();
        var firstName = new NpgsqlConnectionStringBuilder(first).Database;
        var secondName = new NpgsqlConnectionStringBuilder(second).Database;
        Assert.StartsWith("test_", firstName, StringComparison.Ordinal);
        Assert.NotEqual(firstName, secondName);
    }

    [Theory]
    [InlineData("has-dash")]
    [InlineData("has space")]
    [InlineData("9_starts_with_digit")]
    [InlineData("semi;colon")]
    [InlineData("quote\"break")]
    public async Task Unsafe_database_names_are_rejected_before_touching_the_server(string databaseName)
    {
        _fixture.SkipIfDockerUnavailable();
        await Assert.ThrowsAsync<ArgumentException>(
            () => _fixture.Server.CreateIsolatedDatabaseAsync(databaseName));
    }

    private static async Task MigrateCloneProbeAsync(string templateConnectionString, CancellationToken cancellationToken)
    {
        await using var connection = new NpgsqlConnection(templateConnectionString);
        await connection.OpenAsync(cancellationToken);
        await using var command = new NpgsqlCommand(
            "CREATE TABLE probe_widgets(id INT PRIMARY KEY, note TEXT NOT NULL);" +
            "INSERT INTO probe_widgets(id, note) VALUES (1, 'seed-row');",
            connection);
        await command.ExecuteNonQueryAsync(cancellationToken);
    }

    private static async Task ExecuteAsync(string connectionString, string sql)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync();
        await using var command = new NpgsqlCommand(sql, connection);
        await command.ExecuteNonQueryAsync();
    }

    private static async Task<List<string>> QueryAsync(string connectionString, string sql)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync();
        await using var command = new NpgsqlCommand(sql, connection);
        await using var reader = await command.ExecuteReaderAsync();
        var notes = new List<string>();
        while (await reader.ReadAsync())
        {
            notes.Add(reader.GetString(0));
        }

        return notes;
    }
}
