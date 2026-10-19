using Platform.Identity.Contracts;
using Platform.Identity.Testing;

namespace Platform.Identity.Tests;

public sealed class RefreshTokenRotationTests
{
    [Fact]
    public async Task Rotation_returns_a_new_handle_for_a_valid_token()
    {
        var store = new InMemoryRefreshTokenStore();
        var issued = await store.IssueAsync("alice", "session-1", DateTimeOffset.UtcNow.AddMinutes(10));
        Assert.True(issued.Succeeded);

        var rotated = await store.ConsumeAsync(issued.Value!.Handle);
        Assert.True(rotated.Succeeded);
        Assert.NotEqual(issued.Value.Handle, rotated.Value!.Handle);
    }

    [Fact]
    public async Task Unknown_handle_returns_InvalidHandle()
    {
        var store = new InMemoryRefreshTokenStore();
        var rotated = await store.ConsumeAsync("rt_missing");
        Assert.False(rotated.Succeeded);
        Assert.Equal(IdentityLifecycleOutcome.InvalidHandle, rotated.Outcome);
    }

    [Fact]
    public async Task Replay_of_a_consumed_handle_is_reported_and_revokes_the_family()
    {
        var store = new InMemoryRefreshTokenStore();
        var issued = await store.IssueAsync("alice", "session-1", DateTimeOffset.UtcNow.AddMinutes(10));
        var first = await store.ConsumeAsync(issued.Value!.Handle);
        var replay = await store.ConsumeAsync(issued.Value.Handle);

        Assert.True(first.Succeeded);
        Assert.False(replay.Succeeded);
        Assert.Equal(IdentityLifecycleOutcome.Replayed, replay.Outcome);

        var successor = await store.ConsumeAsync(first.Value!.Handle);
        Assert.False(successor.Succeeded);
        Assert.Equal(IdentityLifecycleOutcome.Revoked, successor.Outcome);
    }

    [Fact]
    public async Task Expired_handle_is_rejected_without_revoke()
    {
        var store = new InMemoryRefreshTokenStore();
        var issued = await store.IssueAsync("alice", "session-1", DateTimeOffset.UtcNow.AddMilliseconds(50));
        await Task.Delay(75);
        var rotated = await store.ConsumeAsync(issued.Value!.Handle);

        Assert.False(rotated.Succeeded);
        Assert.Equal(IdentityLifecycleOutcome.Expired, rotated.Outcome);
    }

    [Fact]
    public async Task Revoke_invalidates_the_handle_and_its_family()
    {
        var store = new InMemoryRefreshTokenStore();
        var issued = await store.IssueAsync("alice", "session-1", DateTimeOffset.UtcNow.AddMinutes(10));
        var first = await store.ConsumeAsync(issued.Value!.Handle);
        var revoke = await store.RevokeAsync(issued.Value.Handle);
        var successor = await store.ConsumeAsync(first.Value!.Handle);

        Assert.Equal(IdentityLifecycleOutcome.Succeeded, revoke);
        Assert.False(successor.Succeeded);
        Assert.Equal(IdentityLifecycleOutcome.Revoked, successor.Outcome);
    }

    [Fact]
    public async Task Concurrent_consumers_observe_a_single_success()
    {
        var store = new InMemoryRefreshTokenStore();
        var issued = await store.IssueAsync("alice", "session-1", DateTimeOffset.UtcNow.AddMinutes(10));
        var handle = issued.Value!.Handle;

        var barrier = new TaskCompletionSource();
        var workers = Enumerable.Range(0, 32)
            .Select(_ => Task.Run(async () =>
            {
                await barrier.Task;
                return await store.ConsumeAsync(handle);
            }))
            .ToArray();
        barrier.SetResult();
        var results = await Task.WhenAll(workers);

        var successes = results.Count(r => r.Succeeded);
        var refusals = results.Count(r => !r.Succeeded);
        Assert.Equal(1, successes);
        Assert.Equal(31, refusals);
        Assert.All(results.Where(r => !r.Succeeded), r => Assert.Contains(r.Outcome, new[] { IdentityLifecycleOutcome.Replayed, IdentityLifecycleOutcome.Revoked }));
    }

    [Fact]
    public async Task Default_service_emits_audit_events()
    {
        var audit = new RecordingIdentityAuditHook();
        var store = new InMemoryRefreshTokenStore();
        var service = new DefaultRefreshTokenService(store, audit);
        var issued = await service.IssueAsync("alice", "session-1", DateTimeOffset.UtcNow.AddMinutes(10));
        await service.RotateAsync(issued.Value!.Handle);
        await service.RevokeAsync(issued.Value.Handle);

        var actions = audit.Events.Select(e => e.Action).ToArray();
        Assert.Contains("identity.refresh.issued", actions);
        Assert.Contains("identity.refresh.rotated", actions);
        Assert.Contains("identity.refresh.revoked", actions);
    }
}
