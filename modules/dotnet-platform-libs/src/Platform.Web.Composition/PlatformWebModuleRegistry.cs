namespace Platform.Web.Composition;

/// <summary>
/// Immutable ordered snapshot of the modules registered on one service
/// collection. Resolved from the application service provider, so registry
/// state is scoped to the host and never shared across hosts in the same
/// process. Modules are ordered by <see cref="IPlatformWebModule.Order"/>
/// and then by <see cref="IPlatformWebModule.Name"/> (ordinal).
/// </summary>
public sealed class PlatformWebModuleRegistry
{
    private readonly IReadOnlyList<IPlatformWebModule> _modules;

    /// <summary>
    /// Initializes a new instance of the <see cref="PlatformWebModuleRegistry"/> class.
    /// </summary>
    /// <param name="modules">The registered modules. The collection is copied; later service-collection changes do not affect this snapshot.</param>
    public PlatformWebModuleRegistry(IEnumerable<IPlatformWebModule> modules)
    {
        ArgumentNullException.ThrowIfNull(modules);
        _modules = modules
            .Where(module => module is not null)
            .OrderBy(module => module.Order)
            .ThenBy(module => module.Name, StringComparer.Ordinal)
            .ToArray();
    }

    /// <summary>
    /// Gets the registered modules in invocation order.
    /// </summary>
    public IReadOnlyList<IPlatformWebModule> Modules => _modules;

    /// <summary>
    /// Builds the ordered snapshot from the provider service set.
    /// </summary>
    /// <param name="provider">The application service provider.</param>
    /// <returns>The ordered registry snapshot.</returns>
    public static PlatformWebModuleRegistry FromProvider(IServiceProvider provider)
    {
        ArgumentNullException.ThrowIfNull(provider);
        var modules = provider.GetService(typeof(IEnumerable<IPlatformWebModule>)) as IEnumerable<IPlatformWebModule>
            ?? [];
        return new PlatformWebModuleRegistry(modules);
    }
}
