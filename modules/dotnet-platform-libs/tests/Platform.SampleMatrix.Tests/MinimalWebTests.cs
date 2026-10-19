using System.Net;
using Microsoft.AspNetCore.Mvc.Testing;
using Platform.MinimalWeb.Sample;

namespace Platform.SampleMatrix.Tests;

/// <summary>Stage 1: the minimal web runtime answers the application route and liveness.</summary>
public sealed class MinimalWebTests : IClassFixture<WebApplicationFactory<MinimalWebSampleMarker>>
{
    private readonly WebApplicationFactory<MinimalWebSampleMarker> _factory;

    /// <summary>Creates the test with the hosted minimal-web sample.</summary>
    /// <param name="factory">The sample host factory.</param>
    public MinimalWebTests(WebApplicationFactory<MinimalWebSampleMarker> factory)
    {
        _factory = factory;
    }

    /// <summary>Verifies the application-owned root route.</summary>
    [Fact]
    public async Task Root_returns_application_status()
    {
        var client = _factory.CreateClient();
        var response = await client.GetAsync("/");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("Platform.MinimalWeb.Sample", body, StringComparison.Ordinal);
        Assert.Contains("ok", body, StringComparison.Ordinal);
    }

    /// <summary>Verifies the platform-owned liveness endpoint.</summary>
    [Fact]
    public async Task Live_reports_live()
    {
        var client = _factory.CreateClient();
        var response = await client.GetAsync("/live");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Contains("live", await response.Content.ReadAsStringAsync(), StringComparison.Ordinal);
    }
}
