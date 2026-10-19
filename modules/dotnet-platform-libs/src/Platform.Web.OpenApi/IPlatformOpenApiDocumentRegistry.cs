namespace Platform.Web.OpenApi;

/// <summary>Aggregates registered document providers and resolves a document by name.</summary>
public interface IPlatformOpenApiDocumentRegistry
{
    /// <summary>Returns the available document names, ordered by registration.</summary>
    IReadOnlyList<string> AvailableDocuments { get; }

    /// <summary>Resolves a document. Returns <c>null</c> when no provider recognises the name.</summary>
    string? Resolve(string name);
}
