namespace Platform.Web.OpenApi;

/// <summary>Configures the platform OpenAPI integration.</summary>
public sealed class PlatformWebOpenApiOptions
{
    /// <summary>The default document identifier used when the application does not pass an explicit one.</summary>
    public const string DefaultDocumentName = "v1";

    /// <summary>The default content type for the served OpenAPI document.</summary>
    public const string DefaultContentType = "application/json; charset=utf-8";

    /// <summary>Gets or sets the documents to expose. Each document must be registered with a provider before mapping.</summary>
    public IList<PlatformWebOpenApiDocumentOptions> Documents { get; set; } = new List<PlatformWebOpenApiDocumentOptions>();

    /// <summary>Gets or sets the route prefix for mapped OpenAPI document endpoints. Defaults to <c>/openapi</c>.</summary>
    public string RoutePrefix { get; set; } = "/openapi";

    /// <summary>Validates option values and returns human-readable failures.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (string.IsNullOrWhiteSpace(RoutePrefix) || !RoutePrefix.StartsWith('/'))
            errors.Add("RoutePrefix must be an absolute application path.");

        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var document in Documents)
        {
            if (document is null) { errors.Add("Documents must not contain null entries."); continue; }
            if (string.IsNullOrWhiteSpace(document.Name)) errors.Add("Document.Name is required.");
            else if (!seen.Add(document.Name)) errors.Add($"Duplicate document name: '{document.Name}'.");
            if (string.IsNullOrWhiteSpace(document.Path)) errors.Add($"Document '{document.Name}' is missing a Path.");
            else if (!document.Path.StartsWith('/')) errors.Add($"Document '{document.Name}' Path must be absolute.");
            if (string.IsNullOrWhiteSpace(document.ContentType)) errors.Add($"Document '{document.Name}' ContentType is required.");
        }
        return errors;
    }
}

/// <summary>Configures a single named OpenAPI document.</summary>
public sealed class PlatformWebOpenApiDocumentOptions
{
    /// <summary>Gets or sets the document name. The platform uses this as the OpenAPI <c>info.title</c> token when no title is supplied.</summary>
    public string Name { get; set; } = PlatformWebOpenApiOptions.DefaultDocumentName;

    /// <summary>Gets or sets the document path (e.g. <c>/openapi/v1.json</c>).</summary>
    public string Path { get; set; } = string.Empty;

    /// <summary>Gets or sets the content type served at <see cref="Path"/>. Defaults to <c>application/json; charset=utf-8</c>.</summary>
    public string ContentType { get; set; } = PlatformWebOpenApiOptions.DefaultContentType;

    /// <summary>Gets or sets the human-friendly title used as the OpenAPI <c>info.title</c>.</summary>
    public string? Title { get; set; }

    /// <summary>Gets or sets the OpenAPI version declared in <c>openapi</c>.</summary>
    public string OpenApiVersion { get; set; } = "3.0.3";
}
