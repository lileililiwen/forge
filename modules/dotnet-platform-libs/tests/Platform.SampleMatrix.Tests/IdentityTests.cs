using System.Net;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Mvc.Testing;
using Platform.Identity.Sample;

namespace Platform.SampleMatrix.Tests;

/// <summary>Stage 3: credential verification against the application-owned store.</summary>
public sealed class IdentityTests : IClassFixture<WebApplicationFactory<IdentitySampleMarker>>
{
    private readonly WebApplicationFactory<IdentitySampleMarker> _factory;

    /// <summary>Creates the test with the hosted identity sample.</summary>
    /// <param name="factory">The sample host factory.</param>
    public IdentityTests(WebApplicationFactory<IdentitySampleMarker> factory)
    {
        _factory = factory;
    }

    /// <summary>Verifies known credentials authenticate with the expected subject.</summary>
    [Fact]
    public async Task Known_credentials_authenticate()
    {
        var client = _factory.CreateClient();
        var response = await client.PostAsJsonAsync(
            "/sample/login",
            new { identifier = "ada", secret = "correct-horse" });
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("user-ada", body, StringComparison.Ordinal);
    }

    /// <summary>Verifies unknown credentials fail closed without provider internals.</summary>
    [Fact]
    public async Task Unknown_credentials_return_unauthorized()
    {
        var client = _factory.CreateClient();
        var response = await client.PostAsJsonAsync(
            "/sample/login",
            new { identifier = "ada", secret = "wrong" });
        Assert.Equal(HttpStatusCode.Unauthorized, response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.DoesNotContain("user-ada", body, StringComparison.Ordinal);
    }

    /// <summary>Verifies malformed credentials are rejected as a bad request.</summary>
    [Fact]
    public async Task Empty_identifier_is_rejected()
    {
        var client = _factory.CreateClient();
        var response = await client.PostAsJsonAsync(
            "/sample/login",
            new { identifier = "", secret = "correct-horse" });
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }
}
