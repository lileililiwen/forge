using System.Text.Json;

namespace Platform.Web.OpenApi;

/// <summary>Supplies the JSON document for a named OpenAPI definition.</summary>
/// <remarks>The platform does not own a specific OpenAPI implementation. Implementations are expected to return a stable JSON document that already includes the <c>openapi</c>, <c>info</c>, and <c>paths</c> sections and the application-owned authorization metadata.</remarks>
public interface IPlatformOpenApiDocumentProvider
{
    /// <summary>The document names this provider can supply. The platform rejects duplicates and unknown requests.</summary>
    IReadOnlyCollection<string> SupportedDocuments { get; }

    /// <summary>Returns the JSON document for the supplied name, or <c>null</c> if the provider does not know it.</summary>
    string? GetDocument(string name);
}
