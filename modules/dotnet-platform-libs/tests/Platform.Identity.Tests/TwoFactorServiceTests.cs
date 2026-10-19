using Platform.Identity.Contracts;
using Platform.Identity.Testing;

namespace Platform.Identity.Tests;

public sealed class TwoFactorServiceTests
{
    [Fact]
    public async Task Issue_returns_an_opaque_challenge_with_expiry()
    {
        var service = new FakeTwoFactorService();
        var result = await service.IssueAsync("alice");
        Assert.True(result.Succeeded);
        Assert.True(result.Value!.ExpiresAt > DateTimeOffset.UtcNow);
        Assert.NotEmpty(result.Value.Channels);
    }

    [Fact]
    public async Task Verify_accepts_the_configured_code()
    {
        var service = new FakeTwoFactorService { AcceptedCode = "abcdef" };
        var challenge = await service.IssueAsync("alice");
        var verification = await service.VerifyAsync(challenge.Value!.ChallengeId, "abcdef");
        Assert.True(verification.Succeeded);
    }

    [Fact]
    public async Task Verify_with_wrong_code_is_PolicyDenied()
    {
        var service = new FakeTwoFactorService { AcceptedCode = "abcdef" };
        var challenge = await service.IssueAsync("alice");
        var verification = await service.VerifyAsync(challenge.Value!.ChallengeId, "000000");
        Assert.Equal(IdentityLifecycleOutcome.PolicyDenied, verification.Outcome);
    }

    [Fact]
    public async Task Verify_with_unknown_challenge_is_InvalidHandle()
    {
        var service = new FakeTwoFactorService();
        var verification = await service.VerifyAsync("tfc_missing", "000000");
        Assert.Equal(IdentityLifecycleOutcome.InvalidHandle, verification.Outcome);
    }

    [Fact]
    public async Task Issue_with_empty_subject_is_InvalidRequest()
    {
        var service = new FakeTwoFactorService();
        var result = await service.IssueAsync(" ");
        Assert.Equal(IdentityLifecycleOutcome.InvalidRequest, result.Outcome);
    }
}
