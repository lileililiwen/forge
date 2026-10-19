using Platform.Starter.DependencyInjection;
//#if (EnablePersistence)
using Microsoft.EntityFrameworkCore;
using Platform.Persistence.EfCore.DependencyInjection;
using Platform.Persistence.EfCore.Interceptors;
using StarterApp.Data;
//#endif
//#if (EnableSiteUsers)
using Microsoft.AspNetCore.Identity;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Hosting;
using Platform.Identity.AspNetCore;
using StarterApp.SiteUsers;
//#endif

//#if (EnableSiteUsers)
// bootstrap-owner is a Development-only sub-command. When the user invokes
// `dotnet run -- bootstrap-owner --email owner@example.com`, we run the
// command and exit before the web host starts. The platform application
// defaults to the Development environment when no ASPNETCORE_ENVIRONMENT
// or DOTNET_ENVIRONMENT is set, so the bootstrap command honors the same
// default rather than the host's "Production when nothing is set" behavior.
if (BootstrapOwnerCommand.IsBootstrapOwnerCommand(args))
{
    var bootstrapEnvironmentName = Environment.GetEnvironmentVariable("ASPNETCORE_ENVIRONMENT")
        ?? Environment.GetEnvironmentVariable("DOTNET_ENVIRONMENT")
        ?? "Development";
    var bootstrapBuilder = WebApplication.CreateBuilder(new WebApplicationOptions
    {
        Args = args,
        EnvironmentName = bootstrapEnvironmentName,
    });
    return await BootstrapOwnerCommand.RunAsync(args, bootstrapBuilder.Environment, bootstrapBuilder.Configuration);
}
//#endif

// The platform application defaults to the Development environment when no
// ASPNETCORE_ENVIRONMENT or DOTNET_ENVIRONMENT is set, so a fresh
// template checkout is runnable from `dotnet run` without exposing a
// hardened Production configuration that has not been wired up.
var environmentName = Environment.GetEnvironmentVariable("ASPNETCORE_ENVIRONMENT")
    ?? Environment.GetEnvironmentVariable("DOTNET_ENVIRONMENT")
    ?? "Development";
var builder = WebApplication.CreateBuilder(new WebApplicationOptions
{
    Args = args,
    EnvironmentName = environmentName,
});
builder.Services.AddPlatformApplication(options =>
{
//#if (EnableIdentity)
    options.EnableIdentity = true;
//#endif
    options.EnableAdmin = false;
});
//#if (EnablePersistence)
builder.Services.AddPlatformPersistenceEfCore();
builder.Services.AddDbContext<AppDbContext>((provider, options) =>
    options.UseSqlite(builder.Configuration.GetConnectionString("Default"))
        .AddInterceptors(provider.GetRequiredService<PlatformSaveChangesInterceptor>()));
//#endif
//#if (EnableSiteUsers)
builder.Services.AddPlatformIdentity();
builder.Services.AddSiteUsers(builder.Configuration);
builder.Services.AddRazorPages();
//#endif

var app = builder.Build();
app.UsePlatformApplication();
//#if (EnableSiteUsers)
app.UseSiteUsers();
app.MapRazorPages();
//#endif
app.MapGet("/", () => Results.Json(new { application = "StarterApp", status = "ok" }));
app.MapPlatformApplicationEndpoints();
app.Run();

return 0;

/// <summary>Entry point marker for integration hosting.</summary>
public partial class Program;
