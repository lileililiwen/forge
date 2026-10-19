namespace Platform.Web.OpenApi;

/// <summary>Default registry that aggregates registered <see cref="IPlatformOpenApiDocumentProvider"/> instances.</summary>
public sealed class PlatformOpenApiDocumentRegistry : IPlatformOpenApiDocumentRegistry
{
    private readonly IReadOnlyList<IPlatformOpenApiDocumentProvider> _providers;

    /// <summary>Initializes a new instance of the <see cref="PlatformOpenApiDocumentRegistry"/> class.</summary>
    public PlatformOpenApiDocumentRegistry(IEnumerable<IPlatformOpenApiDocumentProvider> providers)
    {
        ArgumentNullException.ThrowIfNull(providers);
        _providers = providers.ToArray();
    }

    /// <inheritdoc />
    public IReadOnlyList<string> AvailableDocuments
    {
        get
        {
            var names = new List<string>();
            foreach (var provider in _providers)
                foreach (var name in provider.SupportedDocuments)
                    if (!names.Contains(name, StringComparer.OrdinalIgnoreCase))
                        names.Add(name);
            return names;
        }
    }

    /// <inheritdoc />
    public string? Resolve(string name)
    {
        if (string.IsNullOrWhiteSpace(name)) return null;
        foreach (var provider in _providers)
        {
            var document = provider.GetDocument(name);
            if (document is not null) return document;
        }
        return null;
    }
}
