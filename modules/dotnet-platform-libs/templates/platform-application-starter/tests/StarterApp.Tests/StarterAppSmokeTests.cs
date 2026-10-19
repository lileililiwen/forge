using System.Net;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Mvc.Testing;

namespace StarterApp.Tests;

public sealed class StarterAppSmokeTests : IClassFixture<WebApplicationFactory<Program>>
{
    private readonly HttpClient _client;

    public StarterAppSmokeTests(WebApplicationFactory<Program> factory)
    {
        _client = factory.CreateClient();
    }

    [Fact]
    public async Task Root_returns_application_status()
    {
        var response = await _client.GetAsync("/");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var payload = await response.Content.ReadFromJsonAsync<Dictionary<string, string>>();
        Assert.Equal("StarterApp", payload!["application"]);
        Assert.Equal("ok", payload["status"]);
    }

    [Fact]
    public async Task Liveness_endpoint_reports_live()
    {
        var response = await _client.GetAsync("/live");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
    }
}
