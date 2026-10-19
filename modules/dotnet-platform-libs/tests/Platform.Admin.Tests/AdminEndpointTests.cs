using System.Security.Claims;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Admin.AspNetCore;
using Platform.Admin.Contracts;
using Platform.Admin.Testing;
using Platform.Identity.Contracts;

namespace Platform.Admin.Tests;

public sealed class AdminEndpointTests
{
    [Fact]
    public async Task Missing_permission_returns_forbidden_without_calling_store()
    {
        await using var app = BuildApp(hasPermissions: false);
        var response = await app.GetTestClient().GetAsync("/admin/users");

        Assert.Equal(StatusCodes.Status403Forbidden, (int)response.StatusCode);
        Assert.Equal(0, app.Services.GetRequiredService<FakeAdminStore>().UserQueries);
    }

    [Fact]
    public async Task Oversized_query_returns_bad_request_without_calling_store()
    {
        await using var app = BuildApp(hasPermissions: true);
        var response = await app.GetTestClient().GetAsync("/admin/users?pageSize=101");

        Assert.Equal(StatusCodes.Status400BadRequest, (int)response.StatusCode);
        Assert.Equal(0, app.Services.GetRequiredService<FakeAdminStore>().UserQueries);
    }

    [Fact]
    public async Task Authorized_query_is_bounded_and_returns_projection()
    {
        await using var app = BuildApp(hasPermissions: true);
        var response = await app.GetTestClient().GetAsync("/admin/users?pageSize=2&search=alice");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        Assert.Contains("alice@example.test", await response.Content.ReadAsStringAsync());
        Assert.Equal(1, app.Services.GetRequiredService<FakeAdminStore>().UserQueries);
    }

    [Fact]
    public async Task Successful_user_mutation_emits_a_structured_audit_entry()
    {
        await using var app = BuildApp(hasPermissions: true);
        var response = await app.GetTestClient().PostAsJsonAsync("/admin/users/u1/enabled", new { enabled = false });

        Assert.Equal(StatusCodes.Status204NoContent, (int)response.StatusCode);
        var audit = Assert.Single(app.Services.GetRequiredService<RecordingAdminAuditSink>().Entries);
        Assert.Equal("user.disabled", audit.Action);
        Assert.Equal("operator", audit.ActorId);
        Assert.Equal("u1", audit.SubjectId);
        Assert.Equal("tenant-1", audit.TenantId);
        Assert.False(audit.Reason is not null && audit.Reason.Contains("secret", StringComparison.OrdinalIgnoreCase));
    }

    private static WebApplication BuildApp(bool hasPermissions)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        builder.Services.AddAuthentication("test").AddScheme<AuthenticationSchemeOptions, TestAuthHandler>("test", _ => { });
        builder.Services.AddSingleton(new FakeAdminStore());
        builder.Services.AddSingleton<IAdminStore>(sp => sp.GetRequiredService<FakeAdminStore>());
        builder.Services.AddSingleton<RecordingAdminAuditSink>();
        builder.Services.AddSingleton<IAdminAuditSink>(sp => sp.GetRequiredService<RecordingAdminAuditSink>());
        builder.Services.AddSingleton<ICurrentUserAccessor>(new FixedCurrentUserAccessor(hasPermissions));
        builder.Services.AddPlatformAdmin(o => o.MaximumPageSize = 100);
        var app = builder.Build();
        app.UseAuthentication();
        app.UseAuthorization();
        app.MapPlatformAdminEndpoints();
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }

    private sealed class TestAuthHandler(Microsoft.Extensions.Options.IOptionsMonitor<AuthenticationSchemeOptions> options, Microsoft.Extensions.Logging.ILoggerFactory logger, System.Text.Encodings.Web.UrlEncoder encoder)
        : AuthenticationHandler<AuthenticationSchemeOptions>(options, logger, encoder)
    {
        protected override Task<AuthenticateResult> HandleAuthenticateAsync() => Task.FromResult(AuthenticateResult.Success(new AuthenticationTicket(new ClaimsPrincipal(new ClaimsIdentity("test")), "test")));
    }

    private sealed class FixedCurrentUserAccessor(bool hasPermissions) : ICurrentUserAccessor
    {
        public CurrentUser GetCurrentUser() => new("operator", TenantId: "tenant-1", Permissions: hasPermissions ? [AdminPermissions.UsersRead, AdminPermissions.UsersManage] : []);
    }

    private sealed class FakeAdminStore : IAdminStore
    {
        public int UserQueries { get; private set; }
        public ValueTask<AdminPage<AdminUser>> GetUsersAsync(AdminQuery query, CancellationToken cancellationToken = default)
        {
            UserQueries++;
            return ValueTask.FromResult(new AdminPage<AdminUser>([new("u1", "alice@example.test", "Alice", "tenant-1", true, [])], query.Page, query.PageSize, 1));
        }
        public ValueTask<AdminPage<AdminRole>> GetRolesAsync(AdminQuery query, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public ValueTask<IReadOnlyList<AdminPermission>> GetPermissionsAsync(CancellationToken cancellationToken = default) => ValueTask.FromResult<IReadOnlyList<AdminPermission>>([]);
        public ValueTask<AdminPage<AdminSession>> GetSessionsAsync(AdminQuery query, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public ValueTask<AdminPage<AdminAuditEntry>> GetAuditAsync(AdminQuery query, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public ValueTask<IReadOnlyList<AdminProviderStatus>> GetProviderStatusesAsync(CancellationToken cancellationToken = default) => ValueTask.FromResult<IReadOnlyList<AdminProviderStatus>>([]);
        public ValueTask<AdminPage<AdminSubscriptionSummary>> GetSubscriptionSummariesAsync(AdminQuery query, CancellationToken cancellationToken = default) => throw new NotImplementedException();
        public ValueTask<AdminMutationResult> SetUserEnabledAsync(string userId, bool enabled, string actorId, string? tenantId, CancellationToken cancellationToken = default) => ValueTask.FromResult(AdminMutationResult.Success());
        public ValueTask<AdminMutationResult> RevokeSessionAsync(string sessionId, string actorId, string? tenantId, CancellationToken cancellationToken = default) => ValueTask.FromResult(AdminMutationResult.Success());
    }
}
