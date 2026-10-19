using System.Collections.Concurrent;
using System.Net.Http;
using System.Net.Sockets;
using Platform.Persistence.Postgres.Testing;

namespace Platform.Persistence.Postgres.Tests;

/// <summary>
/// Collection name for every Docker-gated PostgreSQL test: one
/// <see cref="PostgresTestServer"/> per test assembly.
/// </summary>
public static class PostgresServerCollection
{
    public const string Name = "PostgresSharedServer";
}

[CollectionDefinition(PostgresServerCollection.Name)]
public sealed class PostgresServerCollectionDefinition : ICollectionFixture<SharedPostgresServerFixture>
{
}

/// <summary>
/// Assembly-shared PostgreSQL server for the Docker-gated tests. When Docker
/// is unavailable the server stays null and each test skips with the reason
/// instead of passing or failing on infrastructure.
/// </summary>
public sealed class SharedPostgresServerFixture : IAsyncLifetime
{
    private static readonly ConcurrentDictionary<string, (string Id, string Name)> ObservedContainers = new();

    private PostgresTestServer? _server;
    private string? _unavailableReason;

    public PostgresTestServer Server =>
        _server ?? throw new InvalidOperationException("Docker is unavailable; call SkipIfDockerUnavailable() first.");

    public async Task InitializeAsync()
    {
        try
        {
            _server = await PostgresTestServer.StartAsync(
                new PostgresTestServerOptions { Password = "platform-test-pw-1" }).ConfigureAwait(false);
        }
        catch (Exception exception) when (LooksLikeMissingDocker(exception))
        {
            _unavailableReason = exception.Message;
        }
    }

    public async Task DisposeAsync()
    {
        if (_server is not null)
        {
            await _server.DisposeAsync().ConfigureAwait(false);
        }
    }

    /// <summary>Skips the calling test when Docker is unavailable.</summary>
    public void SkipIfDockerUnavailable()
    {
        if (_server is null)
        {
            throw Xunit.Sdk.SkipException.ForSkip(
                "Docker is unavailable, PostgreSQL fixture behavior was not verified. " + _unavailableReason);
        }
    }

    /// <summary>
    /// Records the container observed by a test class and, once two classes
    /// have reported, asserts both saw the same container.
    /// </summary>
    public static void RecordSharedContainerObservation(Type testClass, PostgresTestServer server)
    {
        Assert.False(string.IsNullOrWhiteSpace(server.ContainerId));
        Assert.False(string.IsNullOrWhiteSpace(server.ContainerName));
        ObservedContainers[testClass.FullName ?? testClass.Name] = (server.ContainerId, server.ContainerName);
        if (ObservedContainers.Count >= 2)
        {
            Assert.Single(ObservedContainers.Values.Select(observed => observed.Id).Distinct());
            Assert.Single(ObservedContainers.Values.Select(observed => observed.Name).Distinct());
        }
    }

    private static bool LooksLikeMissingDocker(Exception exception)
    {
        for (var current = exception; current is not null; current = current.InnerException)
        {
            var typeName = current.GetType().FullName ?? string.Empty;
            if (typeName.Contains("Docker", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            if (current is SocketException or HttpRequestException)
            {
                return true;
            }
        }

        return exception.Message.Contains("docker", StringComparison.OrdinalIgnoreCase);
    }
}
