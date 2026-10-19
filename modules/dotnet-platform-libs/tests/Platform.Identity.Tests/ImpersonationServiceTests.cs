using Platform.Identity.Contracts;
using Platform.Identity.Testing;

namespace Platform.Identity.Tests;

public sealed class ImpersonationServiceTests
{
    [Fact]
    public async Task Default_policy_denies_every_request_and_audits()
    {
        var audit = new RecordingIdentityAuditHook();
        var service = new FakeImpersonationService(audit: audit);
        var request = new ImpersonationAuthorizationRequest("alice", "bob", "support investigation", TimeSpan.FromMinutes(15));
        var result = await service.StartAsync(request);

        Assert.False(result.Succeeded);
        Assert.Equal(IdentityLifecycleOutcome.PolicyDenied, result.Outcome);
        Assert.Contains(audit.Events, e => e.Action == "identity.impersonation.denied");
    }

    [Fact]
    public async Task Allow_policy_grants_and_audits_started()
    {
        var audit = new RecordingIdentityAuditHook();
        var service = new FakeImpersonationService(new AllowImpersonationPolicy(), audit);
        var request = new ImpersonationAuthorizationRequest("alice", "bob", "support investigation", TimeSpan.FromMinutes(15));
        var result = await service.StartAsync(request);

        Assert.True(result.Succeeded);
        Assert.Equal("bob", result.Value!.TargetSubjectId);
        Assert.Contains(audit.Events, e => e.Action == "identity.impersonation.started" && e.Succeeded);
    }

    [Fact]
    public async Task Active_context_returns_target_for_active_grant()
    {
        var service = new FakeImpersonationService(new AllowImpersonationPolicy());
        var request = new ImpersonationAuthorizationRequest("alice", "bob", "support investigation", TimeSpan.FromMinutes(15));
        var grant = await service.StartAsync(request);
        var context = await service.GetActiveAsync("alice");
        Assert.True(context.Succeeded);
        Assert.Equal(grant.Value!.GrantId, context.Value!.GrantId);
        Assert.Equal("bob", context.Value.TargetSubjectId);
    }

    [Fact]
    public async Task End_invalidates_the_active_grant()
    {
        var service = new FakeImpersonationService(new AllowImpersonationPolicy());
        var request = new ImpersonationAuthorizationRequest("alice", "bob", "support investigation", TimeSpan.FromMinutes(15));
        var grant = await service.StartAsync(request);
        var end = await service.EndAsync(grant.Value!.GrantId);
        Assert.Equal(IdentityLifecycleOutcome.Succeeded, end);

        var context = await service.GetActiveAsync("alice");
        Assert.True(context.Succeeded);
        Assert.False(context.Value!.IsActive);
    }

    [Fact]
    public async Task End_with_unknown_grant_returns_InvalidHandle()
    {
        var service = new FakeImpersonationService(new AllowImpersonationPolicy());
        var end = await service.EndAsync("imp_missing");
        Assert.Equal(IdentityLifecycleOutcome.InvalidHandle, end);
    }

    [Fact]
    public async Task Invalid_request_shape_is_InvalidRequest()
    {
        var service = new FakeImpersonationService(new AllowImpersonationPolicy());
        var result = await service.StartAsync(new ImpersonationAuthorizationRequest("alice", "bob", " ", TimeSpan.Zero));
        Assert.Equal(IdentityLifecycleOutcome.InvalidRequest, result.Outcome);
    }
}
