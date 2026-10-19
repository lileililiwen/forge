using System.Security.Claims;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Authorization;
using Platform.Identity.AspNetCore;
using Platform.Identity.Contracts;
using Platform.Identity.Testing;

namespace Platform.Identity.Tests;

public sealed class IdentityHostIntegrationTests
{
    [Fact]
    public void Current_user_accessor_uses_configurable_subject_and_email_claim_types()
    {
        var context = new DefaultHttpContext
        {
            User = new ClaimsPrincipal(new ClaimsIdentity([
            new Claim("custom_sub", "subject-9"),
            new Claim("custom_email", "b@example.test"),
            new Claim("tenant_id", "tenant-9"),
            new Claim(ClaimTypes.Role, "operator"),
            new Claim("permission", "reports.read")], "test"))
        };
        var services = new ServiceCollection().AddOptions<PlatformIdentityOptions>().Configure(o =>
        {
            o.SubjectClaimType = "custom_sub";
            o.EmailClaimType = "custom_email";
        }).Services;
        services.AddSingleton<IHttpContextAccessor>(new HttpContextAccessor { HttpContext = context });
        services.AddSingleton<HttpCurrentUserAccessor>();
        using var provider = services.BuildServiceProvider();
        var user = provider.GetRequiredService<HttpCurrentUserAccessor>().GetCurrentUser();
        Assert.Equal("subject-9", user.SubjectId);
        Assert.Equal("b@example.test", user.Email);
        Assert.Equal("tenant-9", user.TenantId);
        Assert.Contains("reports.read", user.PermissionSet);
    }

    [Fact]
    public void Current_user_accessor_returns_anonymous_for_unauthenticated_request()
    {
        var context = new DefaultHttpContext { User = new ClaimsPrincipal(new ClaimsIdentity()) };
        var services = new ServiceCollection().AddOptions<PlatformIdentityOptions>().Services;
        services.AddSingleton<IHttpContextAccessor>(new HttpContextAccessor { HttpContext = context });
        services.AddSingleton<HttpCurrentUserAccessor>();
        using var provider = services.BuildServiceProvider();
        var user = provider.GetRequiredService<HttpCurrentUserAccessor>().GetCurrentUser();
        Assert.False(user.IsAuthenticated);
        Assert.Equal(CurrentUser.Anonymous, user);
    }

    [Fact]
    public void Current_user_accessor_deduplicates_duplicate_role_and_permission_claims()
    {
        var context = new DefaultHttpContext
        {
            User = new ClaimsPrincipal(new ClaimsIdentity([
            new Claim(ClaimTypes.NameIdentifier, "subject-2"),
            new Claim(ClaimTypes.Role, "operator"),
            new Claim(ClaimTypes.Role, "operator"),
            new Claim("permission", "reports.read"),
            new Claim("permission", "reports.read")], "test"))
        };
        var services = new ServiceCollection().AddOptions<PlatformIdentityOptions>().Services;
        services.AddSingleton<IHttpContextAccessor>(new HttpContextAccessor { HttpContext = context });
        services.AddSingleton<HttpCurrentUserAccessor>();
        using var provider = services.BuildServiceProvider();
        var user = provider.GetRequiredService<HttpCurrentUserAccessor>().GetCurrentUser();
        Assert.Single(user.RoleSet);
        Assert.Single(user.PermissionSet);
    }

    [Fact]
    public async Task Permission_denied_records_identity_audit_event_without_token_contents()
    {
        var hook = new RecordingIdentityAuditHook();
        var current = new FakeCurrentUserAccessor(new CurrentUser("subject-1"));
        var services = new ServiceCollection();
        services.AddSingleton<ICurrentUserAccessor>(current);
        services.AddSingleton<IIdentityAuditHook>(hook);
        services.AddLogging();
        services.AddPlatformIdentity().RequirePlatformPermission("reports.read");
        using var provider = services.BuildServiceProvider();
        var authorization = provider.GetRequiredService<Microsoft.AspNetCore.Authorization.IAuthorizationService>();
        var result = await authorization.AuthorizeAsync(new ClaimsPrincipal(new ClaimsIdentity("test")), null, "platform:permission:reports.read");
        Assert.False(result.Succeeded);
        var recorded = Assert.Single(hook.Events);
        Assert.Equal("authorization.denied", recorded.Action);
        Assert.Equal("subject-1", recorded.SubjectId);
        Assert.False(recorded.Succeeded);
    }

    [Fact]
    public async Task Session_store_seam_uses_application_store_and_preserves_outcomes()
    {
        var services = new ServiceCollection().AddLogging();
        services.AddPlatformIdentity().AddPlatformIdentitySessionStore<FakeSessionStore>();
        using var provider = services.BuildServiceProvider();
        var sessionService = provider.GetRequiredService<IIdentitySessionService>();
        var created = await sessionService.CreateAsync("subject-1", DateTimeOffset.UtcNow.AddHours(1));
        Assert.True(created.Succeeded);
        Assert.Equal("session-1", created.Value!.SessionId);
        Assert.Equal("subject-1", created.Value.SubjectId);
    }

    [Fact]
    public async Task Session_service_reports_unavailable_when_no_store_registered()
    {
        var services = new ServiceCollection().AddLogging();
        services.AddPlatformIdentity();
        using var provider = services.BuildServiceProvider();
        var sessionService = provider.GetRequiredService<IIdentitySessionService>();
        var created = await sessionService.CreateAsync("subject-1", DateTimeOffset.UtcNow);
        Assert.False(created.Succeeded);
        Assert.Equal(IdentityFailureReason.ProviderUnavailable, created.Failure);
    }

    [Fact]
    public async Task Jwt_validation_fails_at_startup_when_signing_configuration_missing()
    {
        var builder = WebApplication.CreateBuilder();
        builder.Services.AddPlatformIdentityJwt(o =>
        {
            o.Enabled = true;
            o.Issuer = "";
            o.Audience = "";
            o.SigningKey = "";
        });
        var host = builder.Build();
        var exception = await Assert.ThrowsAsync<OptionsValidationException>(() => host.StartAsync());
        Assert.Contains("signing key is required", exception.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Jwt_validation_message_does_not_echo_secret_when_other_fields_missing()
    {
        var builder = WebApplication.CreateBuilder();
        builder.Services.AddPlatformIdentityJwt(o =>
        {
            o.Enabled = true;
            o.Issuer = "";
            o.Audience = "";
            o.SigningKey = "super-secret-value";
        });
        var host = builder.Build();
        var exception = await Assert.ThrowsAsync<OptionsValidationException>(() => host.StartAsync());
        Assert.Contains("issuer is required", exception.Message, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("super-secret-value", exception.Message);
        await host.StopAsync();
    }

    [Fact]
    public void Jwt_options_diagnostic_name_redacts_signing_key()
    {
        var options = new PlatformIdentityJwtOptions
        {
            Enabled = true,
            Issuer = "issuer",
            Audience = "audience",
            SigningKey = "super-secret-value",
        };
        var diagnostic = options.GetDiagnosticName();
        Assert.Contains("signingKey=set", diagnostic);
        Assert.DoesNotContain("super-secret-value", diagnostic);
    }

    private sealed class FakeSessionStore : ISessionStore
    {
        public ValueTask<IdentityProviderResult<IdentitySession>> CreateAsync(string subjectId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityProviderResults.Success(new IdentitySession("session-1", subjectId, expiresAt)));
        public ValueTask<IdentityProviderResult<bool>> RevokeAsync(string sessionId, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityProviderResults.Success(true));
    }

    private sealed class RecordingIdentityAuditHook : IIdentityAuditHook
    {
        public List<IdentityAuditEvent> Events { get; } = [];
        public ValueTask RecordAsync(IdentityAuditEvent auditEvent, CancellationToken cancellationToken = default)
        {
            Events.Add(auditEvent);
            return ValueTask.CompletedTask;
        }
    }
}
