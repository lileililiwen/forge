using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Platform.Web.Composition;

namespace Platform.Web.Composition.Tests;

public class ModuleRegistrationTests
{
    [Fact]
    public void Explicit_registration_records_the_module_and_runs_ConfigureServices()
    {
        var services = new ServiceCollection();

        services.AddPlatformWebModule<OrdersModule>();

        using var provider = services.BuildServiceProvider();
        var registered = Assert.Single(provider.GetServices<IPlatformWebModule>());
        Assert.Equal("orders", registered.Name);
        Assert.True(OrdersModule.ServicesConfigured);
    }

    [Fact]
    public void Registration_orders_modules_by_order_then_name()
    {
        var services = new ServiceCollection();
        services.AddPlatformWebModule<ZuluModule>();
        services.AddPlatformWebModule<AlphaModule>();
        services.AddPlatformWebModule<MiddleModule>();

        using var provider = services.BuildServiceProvider();
        var registry = PlatformWebModuleRegistry.FromProvider(provider);

        Assert.Equal(
            new[] { "zulu", "alpha", "middle" },
            registry.Modules.Select(module => module.Name).ToArray());
    }

    [Fact]
    public void Duplicate_module_type_is_rejected_deterministically()
    {
        var services = new ServiceCollection();
        services.AddPlatformWebModule<OrdersModule>();

        var exception = Assert.Throws<InvalidOperationException>(
            () => services.AddPlatformWebModule<OrdersModule>());
        Assert.Contains(typeof(OrdersModule).FullName!, exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Duplicate_module_name_is_rejected_deterministically()
    {
        var services = new ServiceCollection();
        services.AddPlatformWebModule<OrdersModule>();

        var exception = Assert.Throws<InvalidOperationException>(
            () => services.AddPlatformWebModule<OrdersAliasModule>());
        Assert.Contains("orders", exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Module_with_empty_name_is_rejected()
    {
        var services = new ServiceCollection();

        var exception = Assert.Throws<InvalidOperationException>(
            () => services.AddPlatformWebModule<NamelessModule>());
        Assert.Contains(nameof(IPlatformWebModule.Name), exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Module_without_parameterless_constructor_is_rejected()
    {
        var services = new ServiceCollection();

        var exception = Assert.Throws<InvalidOperationException>(
            () => services.AddPlatformWebModule<ConstructorModule>());
        Assert.Contains(typeof(ConstructorModule).FullName!, exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Non_module_type_is_rejected()
    {
        var services = new ServiceCollection();

        Assert.Throws<ArgumentException>(() => services.AddPlatformWebModule(typeof(string)));
    }

    [Fact]
    public void Registration_rejects_null_arguments()
    {
        Assert.Throws<ArgumentNullException>(() => ServiceCollectionExtensions.AddNullModule());
        Assert.Throws<ArgumentNullException>(() => new ServiceCollection().AddPlatformWebModule(null!));
    }

    private static class ServiceCollectionExtensions
    {
        public static IServiceCollection AddNullModule()
        {
            IServiceCollection? services = null;
            return services!.AddPlatformWebModule<OrdersModule>();
        }
    }

    private sealed class OrdersModule : IPlatformWebModule
    {
        public static bool ServicesConfigured { get; private set; }

        public string Name => "orders";

        public int Order => 10;

        public void ConfigureServices(IServiceCollection services)
        {
            ServicesConfigured = true;
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
            _ = endpoints.MapGet("/orders/ping", () => Results.Ok("orders"));
        }
    }

    private sealed class OrdersAliasModule : IPlatformWebModule
    {
        public string Name => "orders";

        public int Order => 20;

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

    private sealed class NamelessModule : IPlatformWebModule
    {
        public string Name => string.Empty;

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

    private sealed class ConstructorModule : IPlatformWebModule
    {
        public ConstructorModule(string _)
        {
        }

        public string Name => "ctor";

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

    private sealed class AlphaModule : IPlatformWebModule
    {
        public string Name => "alpha";

        public int Order => 5;

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

    private sealed class MiddleModule : IPlatformWebModule
    {
        public string Name => "middle";

        public int Order => 5;

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

    private sealed class ZuluModule : IPlatformWebModule
    {
        public string Name => "zulu";

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
