using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Core.Time;
using Platform.Testing.TestApplication;

var builder = WebApplication.CreateBuilder(args);

builder.Services.AddSingleton<IClock, SystemClock>();
builder.Services.AddSingleton<IGreetingSender, ApplicationGreetingSender>();
builder.Services.AddSingleton<IGreetingSender, DuplicateGreetingSender>();
builder.Services.AddSingleton<IAuditedOperation, ApplicationAuditedOperation>();
builder.Services.AddSingleton<TimeProvider>(TimeProvider.System);
builder.Services.AddHostedService<StopProbeHostedService>();

var app = builder.Build();

// Static file proving the content root resolves to this application's output.
app.UseStaticFiles();

app.MapGet("/", () => Results.Json(new
{
    application = "Platform.Testing.TestApplication",
    status = "ok",
}));

app.MapGet("/live", () => Results.Ok("live"));

app.MapGet("/environment", (IWebHostEnvironment environment) =>
    Results.Text(environment.EnvironmentName));

app.MapGet("/configuration/{key}", (string key, IConfiguration configuration) =>
    Results.Text(configuration[key] ?? "<missing>"));

app.MapGet("/clock", (IClock clock) => Results.Text(clock.UtcNow.ToString("O")));

app.MapGet("/greeting", (IGreetingSender sender) => Results.Text(sender.LastGreeting));

app.MapGet("/audited", (IAuditedOperation operation) => Results.Text(operation.Describe()));

app.Run();

// No `public partial class Program` is declared on purpose: the compiler-generated
// entry point stays internal to this assembly, which is the real shape of a
// top-level-statements app and the reason the public marker type
// PlatformTestApplicationEntryPoint exists.
