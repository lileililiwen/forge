using Platform.Persistence.Postgres.Testing;

namespace Platform.Persistence.Postgres.Tests;

/// <summary>Docker-free unit tests for the fixture guardrails.</summary>
public sealed class PostgresTestServerOptionsTests
{
    [Fact]
    public void Default_image_is_pinned_never_floating()
    {
        Assert.Equal("postgres:17.6-alpine", PostgresTestServerOptions.DefaultImage);
        Assert.Contains(":", PostgresTestServerOptions.DefaultImage, StringComparison.Ordinal);
        Assert.DoesNotContain("latest", PostgresTestServerOptions.DefaultImage, StringComparison.OrdinalIgnoreCase);
        Assert.Equal(PostgresTestServerOptions.DefaultImage, new PostgresTestServerOptions().Image);
    }

    [Fact]
    public void Username_and_password_are_overridable()
    {
        var options = new PostgresTestServerOptions { Username = "app", Password = "s3cret" };
        Assert.Equal("app", options.Username);
        Assert.Equal("s3cret", options.Password);
    }

    [Theory]
    [InlineData("postgres:latest")]
    [InlineData("postgres:LATEST")]
    [InlineData("postgres")]
    [InlineData("localhost:5000/postgres")]
    public async Task Floating_or_missing_image_tags_fail_before_touching_docker(string image)
    {
        var options = new PostgresTestServerOptions { Image = image };
        var exception = await Assert.ThrowsAsync<ArgumentException>(
            () => PostgresTestServer.StartAsync(options));
        Assert.Contains("explicit non-latest tag", exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Missing_image_fails_before_touching_docker()
    {
        var options = new PostgresTestServerOptions { Image = string.Empty };
        await Assert.ThrowsAsync<ArgumentException>(
            () => PostgresTestServer.StartAsync(options));
    }
}
