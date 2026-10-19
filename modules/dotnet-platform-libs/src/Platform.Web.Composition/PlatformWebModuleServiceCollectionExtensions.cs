using Microsoft.Extensions.DependencyInjection;

namespace Platform.Web.Composition;

/// <summary>
/// Explicit module registration. Only types passed to these methods are
/// instantiated; the platform never scans assemblies and never registers
/// Mediator, validators, persistence, or product services implicitly.
/// Duplicate module types or names fail deterministically with
/// <see cref="InvalidOperationException"/>.
/// </summary>
public static class PlatformWebModuleServiceCollectionExtensions
{
    /// <summary>
    /// Registers a module explicitly: validates its name, rejects duplicate
    /// types or names, invokes <see cref="IPlatformWebModule.ConfigureServices"/>,
    /// and records the instance for the per-host registry snapshot.
    /// </summary>
    /// <typeparam name="TModule">The module type. Must declare a public parameterless constructor.</typeparam>
    /// <param name="services">The service collection.</param>
    /// <returns>The service collection.</returns>
    public static IServiceCollection AddPlatformWebModule<TModule>(this IServiceCollection services)
        where TModule : class, IPlatformWebModule
        => services.AddPlatformWebModule(typeof(TModule));

    /// <summary>
    /// Registers a module explicitly by type. Prefer the generic overload;
    /// this overload exists for consumers that enumerate a known,
    /// explicitly-listed set of module types.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="moduleType">The module type. Must implement <see cref="IPlatformWebModule"/> and declare a public parameterless constructor.</param>
    /// <returns>The service collection.</returns>
    public static IServiceCollection AddPlatformWebModule(this IServiceCollection services, Type moduleType)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(moduleType);
        if (!typeof(IPlatformWebModule).IsAssignableFrom(moduleType))
        {
            throw new ArgumentException(
                $"Module type {moduleType.FullName} must implement {nameof(IPlatformWebModule)}.",
                nameof(moduleType));
        }

        var module = CreateModule(moduleType);
        RegisterModule(services, module);
        return services;
    }

    private static IPlatformWebModule CreateModule(Type moduleType)
    {
        try
        {
            if (Activator.CreateInstance(moduleType) is not IPlatformWebModule module)
            {
                throw new InvalidOperationException($"Unable to create module {moduleType.FullName}.");
            }

            return module;
        }
        catch (MissingMethodException exception)
        {
            throw new InvalidOperationException(
                $"Unable to create module {moduleType.FullName}: declare a public parameterless constructor.",
                exception);
        }
    }

    private static void RegisterModule(IServiceCollection services, IPlatformWebModule module)
    {
        var name = module.Name;
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new InvalidOperationException(
                $"Module {module.GetType().FullName} must declare a non-empty {nameof(IPlatformWebModule.Name)}.");
        }

        foreach (var existing in EnumerateRegistered(services))
        {
            if (existing.GetType() == module.GetType())
            {
                throw new InvalidOperationException(
                    $"Module type {module.GetType().FullName} is already registered.");
            }

            if (string.Equals(existing.Name, name, StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"Module name '{name}' is already registered by {existing.GetType().FullName}.");
            }
        }

        module.ConfigureServices(services);
        services.AddSingleton<IPlatformWebModule>(module);
    }

    private static IEnumerable<IPlatformWebModule> EnumerateRegistered(IServiceCollection services)
    {
        foreach (var descriptor in services)
        {
            if (descriptor.ServiceType == typeof(IPlatformWebModule)
                && descriptor.ImplementationInstance is IPlatformWebModule registered)
            {
                yield return registered;
            }
        }
    }
}
