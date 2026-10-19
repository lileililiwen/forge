using System.Net;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Web.Composition;

namespace Platform.Web.Composition.Tests;

public sealed class ModulePipelineTests
{
    [Fact]
    public async Task Middleware_runs_in_module_order()
    {
        await using var app = BuildApp(
            services =>
            {
                services.AddPlatformWebModule<SecondMiddlewareModule>();
                services.AddPlatformWebModule<FirstMiddlewareModule>();
            },
            appBuilder =>
            {
                appBuilder.UsePlatformWebModules();
                appBuilder.Run(context => context.Response.WriteAsync("ok"));
            });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/");
        var values = response.Headers.GetValues("X-Module-Order").ToArray();

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Equal(new[] { "first", "second" }, values);
    }

    [Fact]
    public async Task Endpoints_are_mapped_in_module_order_and_each_hook_runs_once()
    {
        PingModule.MapCount = 0;
        PongModule.MapCount = 0;
        await using var app = BuildApp(
            services =>
            {
                services.AddPlatformWebModule<PongModule>();
                services.AddPlatformWebModule<PingModule>();
            },
            appBuilder => appBuilder.MapPlatformWebModules());
        var client = app.GetTestClient();

        var ping = await client.GetAsync("/ping");
        var pong = await client.GetAsync("/pong");

        Assert.Equal(HttpStatusCode.OK, ping.StatusCode);
        Assert.Equal(HttpStatusCode.OK, pong.StatusCode);
        Assert.Equal("ping", await ping.Content.ReadAsStringAsync());
        Assert.Equal("pong", await pong.Content.ReadAsStringAsync());
        Assert.Equal(1, PingModule.MapCount);
        Assert.Equal(1, PongModule.MapCount);
    }

    [Fact]
    public async Task Registered_modules_contribute_nothing_without_opt_in_calls()
    {
        await using var app = BuildApp(
            services => services.AddPlatformWebModule<PingModule>(),
            _ =>
            {
            });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/ping");

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Unregistered_module_types_are_never_instantiated_or_mapped()
    {
        GhostModule.Created = false;
        await using var app = BuildApp(
            services => services.AddPlatformWebModule<PingModule>(),
            appBuilder => appBuilder.MapPlatformWebModules());
        var client = app.GetTestClient();

        var response = await client.GetAsync("/ghost");

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
        Assert.False(GhostModule.Created);
    }

    [Fact]
    public async Task Two_hosts_in_one_process_keep_independent_module_sets()
    {
        await using var first = BuildApp(
            services => services.AddPlatformWebModule<FirstHostModule>(),
            appBuilder => appBuilder.MapPlatformWebModules());
        await using var second = BuildApp(
            services => services.AddPlatformWebModule<SecondHostModule>(),
            appBuilder => appBuilder.MapPlatformWebModules());

        var firstOwn = await first.GetTestClient().GetAsync("/first");
        var firstForeign = await first.GetTestClient().GetAsync("/second");
        var secondOwn = await second.GetTestClient().GetAsync("/second");
        var secondForeign = await second.GetTestClient().GetAsync("/first");

        Assert.Equal(HttpStatusCode.OK, firstOwn.StatusCode);
        Assert.Equal(HttpStatusCode.NotFound, firstForeign.StatusCode);
        Assert.Equal(HttpStatusCode.OK, secondOwn.StatusCode);
        Assert.Equal(HttpStatusCode.NotFound, secondForeign.StatusCode);
    }

    [Fact]
    public void Pipeline_extensions_reject_null_arguments()
    {
        IApplicationBuilder? app = null;
        IEndpointRouteBuilder? endpoints = null;

        Assert.Throws<ArgumentNullException>(() => app!.UsePlatformWebModules());
        Assert.Throws<ArgumentNullException>(() => endpoints!.MapPlatformWebModules());
    }

    private static WebApplication BuildApp(
        Action<IServiceCollection> services,
        Action<WebApplication> pipeline)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        services(builder.Services);
        var app = builder.Build();
        pipeline(app);
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }

    private sealed class FirstMiddlewareModule : IPlatformWebModule
    {
        public string Name => "middleware-first";

        public int Order => 1;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
            app.Use(async (context, next) =>
            {
                context.Response.Headers.Append("X-Module-Order", "first");
                await next(context);
            });
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
        }
    }

    private sealed class SecondMiddlewareModule : IPlatformWebModule
    {
        public string Name => "middleware-second";

        public int Order => 2;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
            app.Use(async (context, next) =>
            {
                context.Response.Headers.Append("X-Module-Order", "second");
                await next(context);
            });
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
        }
    }

    private sealed class PingModule : IPlatformWebModule
    {
        public static int MapCount;

        public string Name => "ping";

        public int Order => 1;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
            MapCount++;
            _ = endpoints.MapGet("/ping", () => Results.Text("ping"));
        }
    }

    private sealed class PongModule : IPlatformWebModule
    {
        public static int MapCount;

        public string Name => "pong";

        public int Order => 2;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
            MapCount++;
            _ = endpoints.MapGet("/pong", () => Results.Text("pong"));
        }
    }

    private sealed class GhostModule : IPlatformWebModule
    {
        public static bool Created;

        public GhostModule()
        {
            Created = true;
        }

        public string Name => "ghost";

        public int Order => 1;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
            _ = endpoints.MapGet("/ghost", () => Results.Text("ghost"));
        }
    }

    private sealed class FirstHostModule : IPlatformWebModule
    {
        public string Name => "first-host";

        public int Order => 1;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
            _ = endpoints.MapGet("/first", () => Results.Text("first"));
        }
    }

    private sealed class SecondHostModule : IPlatformWebModule
    {
        public string Name => "second-host";

        public int Order => 1;

        public void ConfigureServices(IServiceCollection services)
        {
        }

        public void ConfigureMiddleware(IApplicationBuilder app)
        {
        }

        public void MapEndpoints(IEndpointRouteBuilder endpoints)
        {
            _ = endpoints.MapGet("/second", () => Results.Text("second"));
        }
    }
}
