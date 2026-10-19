using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Web.OpenApi;
using Platform.Web.OpenApi.DependencyInjection;

namespace Platform.Web.Edge.Tests;

public sealed class OpenApiIntegrationTests
{
    [Fact]
    public async Task Registered_document_is_served()
    {
        await using var app = BuildApp(options =>
        {
            options.Documents.Add(new PlatformWebOpenApiDocumentOptions
            {
                Name = "v1",
                Path = "/openapi/v1.json",
                Title = "Demo"
            });
        });
        var client = app.GetTestClient();
        var response = await client.GetAsync("/openapi/v1.json");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"openapi\":\"3.0.3\"", body);
        Assert.Contains("\"title\":\"Demo\"", body);
    }

    [Fact]
    public async Task Unregistered_document_returns_not_found()
    {
        await using var app = BuildApp(options =>
        {
            options.Documents.Add(new PlatformWebOpenApiDocumentOptions { Name = "v1", Path = "/openapi/v1.json" });
        });
        var client = app.GetTestClient();
        var response = await client.GetAsync("/openapi/v2.json");

        Assert.Equal(StatusCodes.Status404NotFound, (int)response.StatusCode);
    }

    [Fact]
    public async Task Map_helpers_resolve_only_registered_documents()
    {
        await using var app = BuildApp(options =>
        {
            options.Documents.Add(new PlatformWebOpenApiDocumentOptions { Name = "v1", Path = "/openapi/v1.json" });
            options.Documents.Add(new PlatformWebOpenApiDocumentOptions { Name = "v2", Path = "/openapi/v2.json", Title = "V2" });
        });
        var client = app.GetTestClient();
        var v1 = await client.GetAsync("/openapi/v1.json");
        var v2 = await client.GetAsync("/openapi/v2.json");
        var v3 = await client.GetAsync("/openapi/v3.json");

        Assert.Equal(StatusCodes.Status200OK, (int)v1.StatusCode);
        Assert.Equal(StatusCodes.Status200OK, (int)v2.StatusCode);
        Assert.Equal(StatusCodes.Status404NotFound, (int)v3.StatusCode);
        Assert.Contains("\"title\":\"V2\"", await v2.Content.ReadAsStringAsync());
    }

    private static WebApplication BuildApp(Action<PlatformWebOpenApiOptions> configure)
    {
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformWebOpenApi(configure);
        builder.Services.AddSingleton<IPlatformOpenApiDocumentProvider>(sp =>
        {
            var options = sp.GetRequiredService<Microsoft.Extensions.Options.IOptions<PlatformWebOpenApiOptions>>().Value;
            return new MultiDocumentProvider(options.Documents);
        });
        var app = builder.Build();
        app.MapPlatformOpenApiDocuments();
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }

    private sealed class MultiDocumentProvider : IPlatformOpenApiDocumentProvider
    {
        private readonly Dictionary<string, PlatformWebOpenApiDocumentOptions> _options;
        public MultiDocumentProvider(IEnumerable<PlatformWebOpenApiDocumentOptions> options) => _options = options.ToDictionary(o => o.Name, StringComparer.OrdinalIgnoreCase);
        public IReadOnlyCollection<string> SupportedDocuments => _options.Keys.ToArray();
        public string? GetDocument(string name) => _options.TryGetValue(name, out var option)
            ? "{\"openapi\":\"" + option.OpenApiVersion + "\",\"info\":{\"title\":\"" + (option.Title ?? option.Name) + "\",\"version\":\"" + option.Name + "\"},\"paths\":{}}"
            : null;
    }
}
