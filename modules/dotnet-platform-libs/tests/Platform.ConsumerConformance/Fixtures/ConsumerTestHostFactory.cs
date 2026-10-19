using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;

namespace Platform.ConsumerConformance.Fixtures;

internal static class ConsumerTestHostFactory
{
    public static IHost Build(Action<IServiceCollection> configureServices, Action<IEndpointRouteBuilder>? configureRoutes = null)
    {
        var builder = new HostBuilder()
            .ConfigureAppConfiguration(configuration =>
            {
                configuration.AddInMemoryCollection(new Dictionary<string, string?>
                {
                    ["Environment"] = "Testing",
                });
            })
            .ConfigureWebHost(web =>
            {
                web.UseTestServer();
                web.ConfigureServices(services =>
                {
                    services.AddRouting();
                    configureServices(services);
                });
                web.Configure(app =>
                {
                    app.UseRouting();
                    if (configureRoutes is not null)
                    {
                        app.UseEndpoints(configureRoutes);
                    }
                });
            });

        var host = builder.Start();
        return host;
    }

    public static WebApplication BuildWebApplication(
        Action<IServiceCollection> configureServices,
        Action<WebApplication> configureApp)
    {
        var options = new WebApplicationOptions
        {
            EnvironmentName = "Testing",
        };
        var builder = WebApplication.CreateBuilder(options);
        builder.WebHost.UseTestServer();
        configureServices(builder.Services);
        var app = builder.Build();
        configureApp(app);
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
