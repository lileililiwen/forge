using System.Net;
using System.Text.Json;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Platform.Privacy.AspNetCore.Cookies;
using Platform.Privacy.AspNetCore.DependencyInjection;
using Platform.Privacy.AspNetCore.Enforcement;
using Platform.Privacy.AspNetCore.State;
using Platform.Privacy.Contracts;

namespace Platform.Privacy.AspNetCore.Tests;

public class ConsentEnforcementTests : IDisposable
{
    private readonly List<WebApplication> _apps = new();

    public void Dispose()
    {
        foreach (var app in _apps)
        {
            app.StopAsync().GetAwaiter().GetResult();
            app.DisposeAsync().GetAwaiter().GetResult();
        }
    }

    private sealed class FixedSubject(string? subject) : IConsentSubjectResolver
    {
        public string? Resolve(HttpContext context) => subject;
    }

    private WebApplication BuildApp(string? subject, Action<ConsentCookieOptions>? configure = null)
    {
        var builder = WebApplication.CreateBuilder();
        builder.Services.AddPlatformPrivacyAspNetCore(configure);
        builder.Services.AddSingleton<IConsentSubjectResolver>(new FixedSubject(subject));
        builder.WebHost.UseTestServer();
        var app = builder.Build();
        app.UseRouting();
        app.UseMiddleware<ConsentEnforcementMiddleware>();
        app.MapGet("/open", () => "open");
        app.MapGet("/analytics", () => "ok").WithMetadata(new RequireConsentAttribute(ConsentCategory.Analytics));
        _apps.Add(app);
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }

    [Fact]
    public async Task Unconsented_category_returns_403_naming_it()
    {
        var app = BuildApp("sub1");
        using var client = app.GetTestClient();
        var response = await client.GetAsync("/analytics");
        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("Analytics", body, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Consented_category_proceeds()
    {
        var app = BuildApp("sub1");
        var store = app.Services.GetRequiredService<IConsentStore>();
        await store.SetAsync(new ConsentRecord(
            "sub1",
            new HashSet<ConsentCategory> { ConsentCategory.Analytics },
            DateTimeOffset.UtcNow,
            1));
        using var client = app.GetTestClient();
        var response = await client.GetAsync("/analytics");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
    }

    [Fact]
    public void Tampered_cookie_reads_null()
    {
        var service = new ConsentCookieService(new ConsentCookieOptions());
        var context = new DefaultHttpContext();
        context.Request.Headers.Cookie = "platform_consent=not-json{{{";
        Assert.Null(service.Read(context.Request));
    }

    [Fact]
    public void Cookie_round_trip_preserves_grants()
    {
        var service = new ConsentCookieService(new ConsentCookieOptions());
        var record = new ConsentRecord("sub1", new HashSet<ConsentCategory> { ConsentCategory.Analytics }, DateTimeOffset.UtcNow, 1);
        var writeContext = new DefaultHttpContext();
        service.Write(writeContext.Response, record);
        var header = writeContext.Response.Headers.SetCookie.ToString();
        var readContext = new DefaultHttpContext();
        readContext.Request.Headers.Cookie = header.Split(';')[0];
        var back = service.Read(readContext.Request);
        Assert.NotNull(back);
        Assert.Equal("sub1", back.SubjectId);
        Assert.Contains(ConsentCategory.Analytics, back.Granted);
    }
}
