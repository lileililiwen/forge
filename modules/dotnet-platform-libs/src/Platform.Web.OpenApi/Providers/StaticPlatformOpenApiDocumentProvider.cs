namespace Platform.Web.OpenApi.Providers;

/// <summary>Test/document provider that returns a static JSON document for a single named document.</summary>
/// <remarks>The default JSON includes the application title, version, and an empty <c>paths</c> object. The platform does not own a real OpenAPI generator; applications replace this with one based on their chosen library.</remarks>
public sealed class StaticPlatformOpenApiDocumentProvider : IPlatformOpenApiDocumentProvider
{
    /// <summary>The supported documents.</summary>
    public IReadOnlyCollection<string> SupportedDocuments { get; }

    private readonly PlatformWebOpenApiDocumentOptions _options;

    /// <summary>Initializes a new instance of the <see cref="StaticPlatformOpenApiDocumentProvider"/> class.</summary>
    public StaticPlatformOpenApiDocumentProvider(PlatformWebOpenApiDocumentOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        _options = options;
        SupportedDocuments = new[] { options.Name };
    }

    /// <inheritdoc />
    public string? GetDocument(string name)
    {
        if (!string.Equals(name, _options.Name, StringComparison.OrdinalIgnoreCase)) return null;
        var title = string.IsNullOrWhiteSpace(_options.Title) ? _options.Name : _options.Title;
        return "{\"openapi\":\"" + Escape(_options.OpenApiVersion) + "\",\"info\":{\"title\":\"" + Escape(title) + "\",\"version\":\"" + Escape(_options.Name) + "\"},\"paths\":{}}";
    }

    private static string Escape(string value) => value.Replace("\\", "\\\\").Replace("\"", "\\\"");
}
