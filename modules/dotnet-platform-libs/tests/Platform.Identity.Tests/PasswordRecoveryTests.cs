using Platform.Identity.Contracts;
using Platform.Identity.Testing;

namespace Platform.Identity.Tests;

public sealed class PasswordRecoveryTests
{
    [Fact]
    public async Task Initiate_returns_Succeeded_for_unknown_and_known_subjects_to_prevent_enumeration()
    {
        var service = new FakePasswordRecoveryService();
        var known = await service.InitiateAsync("alice");
        var unknown = await service.InitiateAsync("bob");
        Assert.True(known.Succeeded);
        Assert.True(unknown.Succeeded);
        Assert.NotEqual(known.Value!.ChallengeId, unknown.Value!.ChallengeId);
    }

    [Fact]
    public async Task Complete_accepts_the_configured_code()
    {
        var service = new FakePasswordRecoveryService { AcceptedCode = "123456" };
        var initiation = await service.InitiateAsync("alice");
        var completion = await service.CompleteAsync(initiation.Value!.ChallengeId, "123456", "new-secret");
        Assert.True(completion.Succeeded);
    }

    [Fact]
    public async Task Complete_with_wrong_code_is_PolicyDenied()
    {
        var service = new FakePasswordRecoveryService { AcceptedCode = "123456" };
        var initiation = await service.InitiateAsync("alice");
        var completion = await service.CompleteAsync(initiation.Value!.ChallengeId, "999999", "new-secret");
        Assert.False(completion.Succeeded);
        Assert.Equal(IdentityLifecycleOutcome.PolicyDenied, completion.Outcome);
    }

    [Fact]
    public async Task Replay_of_a_completed_challenge_is_rejected()
    {
        var service = new FakePasswordRecoveryService();
        var initiation = await service.InitiateAsync("alice");
        await service.CompleteAsync(initiation.Value!.ChallengeId, "000000", "new-secret");
        var replay = await service.CompleteAsync(initiation.Value.ChallengeId, "000000", "new-secret");
        Assert.Equal(IdentityLifecycleOutcome.Replayed, replay.Outcome);
    }

    [Fact]
    public async Task Unknown_challenge_returns_InvalidHandle()
    {
        var service = new FakePasswordRecoveryService();
        var result = await service.CompleteAsync("prc_missing", "000000", "new-secret");
        Assert.Equal(IdentityLifecycleOutcome.InvalidHandle, result.Outcome);
    }

    [Fact]
    public async Task Initiate_with_empty_subject_returns_InvalidRequest()
    {
        var service = new FakePasswordRecoveryService();
        var result = await service.InitiateAsync(" ");
        Assert.Equal(IdentityLifecycleOutcome.InvalidRequest, result.Outcome);
    }
}
