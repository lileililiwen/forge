using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Platform.Web.Composition;

namespace Platform.Web.Composition.Tests;

public class ModuleRegistryTests
{
    [Fact]
    public void Empty_provider_produces_an_empty_snapshot()
    {
        using var provider = new ServiceCollection().BuildServiceProvider();

        var registry = PlatformWebModuleRegistry.FromProvider(provider);

        Assert.Empty(registry.Modules);
    }

    [Fact]
    public void Snapshot_is_ordered_and_insensitive_to_registration_order()
    {
        var services = new ServiceCollection();
        services.AddPlatformWebModule<BravoModule>();
        services.AddPlatformWebModule<AlphaModule>();
        using var provider = services.BuildServiceProvider();

        var registry = PlatformWebModuleRegistry.FromProvider(provider);

        Assert.Equal(
            new[] { "alpha", "bravo" },
            registry.Modules.Select(module => module.Name).ToArray());
    }

    [Fact]
    public void Registry_rejects_null_arguments()
    {
        Assert.Throws<ArgumentNullException>(() => new PlatformWebModuleRegistry(null!));
        Assert.Throws<ArgumentNullException>(() => PlatformWebModuleRegistry.FromProvider(null!));
    }

    private sealed class AlphaModule : IPlatformWebModule
    {
        public string Name => "alpha";

        public int Order => 1;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
        }
    }

    private sealed class BravoModule : IPlatformWebModule
    {
        public string Name => "bravo";

        public int Order => 1;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
        }
    }
}
