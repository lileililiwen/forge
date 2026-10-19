using Platform.Starter.DependencyInjection;

var builder = WebApplication.CreateBuilder(args);

// Stage 1: the smallest platform adoption. The web runtime only; identity,
// admin, billing, AI, notifications, and SMS stay disabled until the
// application explicitly opts in.
builder.Services.AddPlatformApplication(_ => { });

var app = builder.Build();
app.UsePlatformApplication();
app.MapGet("/", () => Results.Json(new { application = "Platform.MinimalWeb.Sample", status = "ok" }));
app.MapPlatformApplicationEndpoints();
app.Run();

/// <summary>Entry point marker for matrix hosting.</summary>
public partial class Program;
