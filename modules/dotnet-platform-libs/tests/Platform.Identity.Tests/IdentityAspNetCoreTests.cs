using System.Security.Claims;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Platform.Authorization;
using Platform.Identity.AspNetCore;
using Platform.Identity.Contracts;
using Platform.Identity.Testing;

namespace Platform.Identity.Tests;

public sealed class IdentityAspNetCoreTests
{
    [Fact]
    public void Current_user_accessor_projects_subject_tenant_role_and_permission_claims()
    {
        var context = new DefaultHttpContext
        {
            User = new ClaimsPrincipal(new ClaimsIdentity([
            new Claim(ClaimTypes.NameIdentifier, "subject-1"),
            new Claim(ClaimTypes.Email, "a@example.test"),
            new Claim("tenant_id", "tenant-1"),
            new Claim(ClaimTypes.Role, "operator"),
            new Claim("permission", "reports.read")], "test"))
        };
        var services = new ServiceCollection().AddOptions<PlatformIdentityOptions>().Services;
        services.AddSingleton<IHttpContextAccessor>(new HttpContextAccessor { HttpContext = context });
        services.AddSingleton<Platform.Identity.AspNetCore.HttpCurrentUserAccessor>();
        using var provider = services.BuildServiceProvider();
        var user = provider.GetRequiredService<HttpCurrentUserAccessor>().GetCurrentUser();
        Assert.True(user.IsAuthenticated);
        Assert.Equal("subject-1", user.SubjectId);
        Assert.Equal("tenant-1", user.TenantId);
        Assert.Contains("reports.read", user.PermissionSet);
    }

    [Fact]
    public async Task Permission_policy_allows_permission_and_records_decision()
    {
        var auditor = new RecordingAuthorizationDecisionAuditor();
        var current = new FakeCurrentUserAccessor(new CurrentUser("subject-1", Permissions: ["reports.read"]));
        var services = new ServiceCollection();
        services.AddSingleton<ICurrentUserAccessor>(current);
        services.AddSingleton<IAuthorizationDecisionAuditor>(auditor);
        services.AddLogging();
        services.AddPlatformIdentity().RequirePlatformPermission("reports.read");
        using var provider = services.BuildServiceProvider();
        var authorization = provider.GetRequiredService<Microsoft.AspNetCore.Authorization.IAuthorizationService>();
        var result = await authorization.AuthorizeAsync(new ClaimsPrincipal(new ClaimsIdentity("test")), null, "platform:permission:reports.read");
        Assert.True(result.Succeeded);
        Assert.True(auditor.Decisions.Single().Decision.Succeeded);
    }

    [Fact]
    public async Task Role_policy_uses_application_owned_role_claim()
    {
        var services = new ServiceCollection().AddLogging();
        services.AddPlatformIdentity().RequirePlatformRole("operator");
        using var provider = services.BuildServiceProvider();
        var authorization = provider.GetRequiredService<Microsoft.AspNetCore.Authorization.IAuthorizationService>();
        var principal = new ClaimsPrincipal(new ClaimsIdentity([new Claim(ClaimTypes.Role, "operator")], "test"));
        var result = await authorization.AuthorizeAsync(principal, null, "platform:role:operator");
        Assert.True(result.Succeeded);
    }
}
