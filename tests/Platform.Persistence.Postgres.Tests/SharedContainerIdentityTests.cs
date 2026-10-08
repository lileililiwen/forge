namespace Platform.Persistence.Postgres.Tests;

/// <summary>
/// Proves one container is shared across test classes in a single run: both
/// classes resolve the same collection fixture and must observe the same
/// container id and name.
/// </summary>
[Collection(PostgresServerCollection.Name)]
public sealed class SharedContainerIdentityTestsA(SharedPostgresServerFixture fixture)
{
    private readonly SharedPostgresServerFixture _fixture = fixture;

    [Fact]
    public void First_class_observes_the_shared_container()
    {
        _fixture.SkipIfDockerUnavailable();
        SharedPostgresServerFixture.RecordSharedContainerObservation(GetType(), _fixture.Server);
    }
}

[Collection(PostgresServerCollection.Name)]
public sealed class SharedContainerIdentityTestsB(SharedPostgresServerFixture fixture)
{
    private readonly SharedPostgresServerFixture _fixture = fixture;

    [Fact]
    public void Second_class_observes_the_same_shared_container()
    {
        _fixture.SkipIfDockerUnavailable();
        SharedPostgresServerFixture.RecordSharedContainerObservation(GetType(), _fixture.Server);
    }
}
