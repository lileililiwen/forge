using System.Net;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Auditing.AspNetCore.DependencyInjection;
using Platform.Auditing.Contracts;

namespace Platform.Auditing.Tests.AspNetCore;

public class AuditMiddlewareTests : IDisposable
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

    private WebApplication Build(
        out CapturingAuditSink sink,
        Action<Platform.Auditing.AspNetCore.Common.AuditAspNetCoreOptions>? configure = null,
        IAuditSink? overrideSink = null)
    {
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformAuditingAspNetCore(configure ?? (_ => { }));
        sink = new CapturingAuditSink();
        builder.Services.AddSingleton<IAuditSink>(overrideSink ?? sink);

        var app = builder.Build();
        app.UseExceptionHandler(errorApp =>
        {
            errorApp.Run(_ => Task.CompletedTask);
        });
        app.UsePlatformAuditing();
        app.MapGet("/resource", () => Results.Ok("ok"));
        app.MapGet("/boom", () => Task.FromException<IResult>(new InvalidOperationException("secret-internal-detail")));
        app.MapGet("/unauthorized", () => Results.Unauthorized());
        app.MapGet("/health", () => Results.Ok("healthy"));

        app.StartAsync().GetAwaiter().GetResult();
        _apps.Add(app);
        return app;
    }

    [Fact]
    public async Task Successful_request_records_http_event_with_bounded_metadata()
    {
        var app = Build(out var sink);
        using var client = app.GetTestClient();

        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var recorded = Assert.Single(sink.Recorded);
        Assert.Equal("http.request", recorded.Action);
        Assert.Equal("http", recorded.Category);
        Assert.Equal(AuditOutcome.Success, recorded.Outcome);
        Assert.Equal("GET", recorded.Metadata["http.method"]);
        Assert.Equal("/resource", recorded.Metadata["http.path"]);
        Assert.True(recorded.Metadata.ContainsKey("http.duration_ms"));
    }

    [Fact]
    public async Task Server_error_response_is_classified_as_error_severity()
    {
        var app = Build(out var sink);
        using var client = app.GetTestClient();

        var response = await client.GetAsync("/boom");

        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        var httpEvent = sink.Recorded.Single(e => e.Category == "http");
        Assert.Equal(AuditOutcome.Error, httpEvent.Outcome);
        Assert.Equal(AuditSeverity.Error, httpEvent.Severity);
    }

    [Fact]
    public async Task Denied_status_records_security_event()
    {
        var app = Build(out var sink);
        using var client = app.GetTestClient();

        var response = await client.GetAsync("/unauthorized");

        Assert.Equal(HttpStatusCode.Unauthorized, response.StatusCode);
        var security = sink.Recorded.Single(e => e.Category == "security");
        Assert.Equal("authorization.denied", security.Action);
        Assert.Equal(AuditOutcome.Denied, security.Outcome);
        Assert.True(security.Metadata.ContainsKey("http.path"));
    }

    [Fact]
    public async Task Exempt_path_is_not_audited()
    {
        var app = Build(out var sink);
        using var client = app.GetTestClient();

        var response = await client.GetAsync("/health");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Empty(sink.Recorded);
    }

    [Fact]
    public async Task Unhandled_exception_records_normalized_exception_event_and_rethrows()
    {
        var app = Build(out var sink);
        using var client = app.GetTestClient();

        var response = await client.GetAsync("/boom");

        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        var exceptionEvent = sink.Recorded.Single(e => e.Category == "exception");
        Assert.Equal("http.exception", exceptionEvent.Action);
        Assert.Equal("server_error", exceptionEvent.Metadata["exception.kind"]);
        Assert.DoesNotContain("secret-internal-detail", string.Join(",", exceptionEvent.Metadata.Values), StringComparison.Ordinal);
    }

    [Fact]
    public async Task Subject_and_tenant_headers_propagate_to_event()
    {
        var app = Build(out var sink);
        using var client = app.GetTestClient();
        using var request = new HttpRequestMessage(HttpMethod.Get, "/resource");
        request.Headers.Add("X-Audit-Subject", "subject-7");
        request.Headers.Add("X-Audit-Tenant", "tenant-3");

        await client.SendAsync(request);

        var recorded = Assert.Single(sink.Recorded);
        Assert.Equal("subject-7", recorded.SubjectId);
        Assert.Equal("tenant-3", recorded.TenantId);
    }

    [Fact]
    public async Task Capture_is_fail_open_when_sink_throws()
    {
        var throwing = new ThrowingAuditSink();
        var app = Build(out _, overrideSink: throwing);
        using var client = app.GetTestClient();

        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Empty(throwing.Recorded);
    }
}
