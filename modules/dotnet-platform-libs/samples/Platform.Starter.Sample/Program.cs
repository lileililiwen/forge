using Platform.Admin.Contracts;
using Platform.Admin.Testing;
using Platform.Starter.DependencyInjection;

var builder = WebApplication.CreateBuilder(args);
builder.Services.AddPlatformApplication(options =>
{
    options.EnableIdentity = true;
    options.EnableAdmin = true;
    options.EnableNotifications = true;
    options.NotificationProviderName = "fake";
});
builder.Services.AddSingleton<IAdminStore, InMemoryAdminStore>();

var app = builder.Build();
app.UsePlatformApplication();
app.MapGet("/", () => Results.Json(new { application = "Platform.Starter.Sample", status = "ok" }));
app.MapGet("/sample/login", () => Results.Json(new { state = "login-ready", provider = "fake" }));
app.MapGet("/sample/permission", () => Results.Json(new { state = "permission-ready", permission = "sample.read" }));
app.MapGet("/sample/providers", () => Results.Json(new { billing = "fake", ai = "fake", mail = "fake", sms = "fake" }));
app.MapGet("/sample/ui", () => Results.Json(new { state = "empty-loading-error-success" }));
app.MapPlatformApplicationEndpoints();
app.Run();

/// <summary>Entry point marker for integration hosting.</summary>
public partial class Program;
