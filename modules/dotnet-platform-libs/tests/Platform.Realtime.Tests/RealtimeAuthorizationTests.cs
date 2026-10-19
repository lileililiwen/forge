using Platform.Realtime.AspNetCore.SignalR;
using Platform.Realtime.Authorization;
using Platform.Realtime.Tenant;

namespace Platform.Realtime.Tests;

public class RealtimeAuthorizationTests
{
    [Fact]
    public void Result_factory_sets_allowed_and_reason()
    {
        Assert.True(RealtimeAuthorizationResult.Allow().Allowed);
        var denied = RealtimeAuthorizationResult.Deny("nope");
        Assert.False(denied.Allowed);
        Assert.Equal("nope", denied.RejectionReason);
    }

    [Fact]
    public async Task DenyAll_authorizer_rejects_every_connection()
    {
        var authorizer = new DenyAllRealtimeAuthorizer();
        var result = await authorizer.AuthorizeAsync(new RealtimeConnectionRequest(), CancellationToken.None);
        Assert.False(result.Allowed);
    }

    [Fact]
    public async Task Policy_throws_when_authorizer_denies()
    {
        var authorizer = new DenyAllRealtimeAuthorizer();
        await Assert.ThrowsAsync<InvalidOperationException>(() =>
            RealtimeHubAuthorization.AuthorizeOrThrowAsync(
                new RealtimeConnectionRequest(),
                authorizer).AsTask());
    }

    [Fact]
    public async Task Policy_does_not_throw_when_authorizer_allows()
    {
        var authorizer = new AllowAlwaysAuthorizer();
        await RealtimeHubAuthorization.AuthorizeOrThrowAsync(
            new RealtimeConnectionRequest(),
            authorizer);
    }

    private sealed class AllowAlwaysAuthorizer : IRealtimeConnectionAuthorizer
    {
        public Task<RealtimeAuthorizationResult> AuthorizeAsync(RealtimeConnectionRequest request, CancellationToken cancellationToken = default) =>
            Task.FromResult(RealtimeAuthorizationResult.Allow());
    }
}

public class RealtimeTenantRoutingTests
{
    [Fact]
    public async Task DenyCrossTenant_allows_equal_tenants_and_broadcast_when_unscoped()
    {
        var router = new DenyCrossTenantRouter();

        Assert.True(await router.IsRouteAllowedAsync(new RealtimeTenantRoute { TargetTenantId = "t1", Caller = new Platform.Core.Context.CallerContext(SubjectId: "s", TenantId: "t1") }));
        Assert.True(await router.IsRouteAllowedAsync(new RealtimeTenantRoute { TargetTenantId = null, Caller = Platform.Core.Context.CallerContext.Anonymous }));
        Assert.False(await router.IsRouteAllowedAsync(new RealtimeTenantRoute { TargetTenantId = "t2", Caller = new Platform.Core.Context.CallerContext(SubjectId: "s", TenantId: "t1") }));
    }

    [Fact]
    public async Task Routing_policy_delegates_to_router()
    {
        var router = new DenyCrossTenantRouter();
        var route = new RealtimeTenantRoute { TargetTenantId = "t2", Caller = new Platform.Core.Context.CallerContext(SubjectId: "s", TenantId: "t1") };

        Assert.False(await RealtimeHubRouting.IsRouteAllowedAsync(route, router));
    }
}
