using Platform.Authorization;
using Platform.Identity.Contracts;
using Platform.Identity.Testing;

namespace Platform.Identity.Tests;

public sealed class IdentityContractTests
{
    [Fact]
    public void Anonymous_user_is_safe_and_provider_neutral()
    {
        var user = CurrentUser.Anonymous;
        Assert.False(user.IsAuthenticated);
        Assert.Empty(user.RoleSet);
        Assert.Empty(user.PermissionSet);
    }

    [Fact]
    public async Task Fake_verifier_classifies_invalid_credentials_without_provider_details()
    {
        var verifier = new FakeCredentialVerifier().Add("alice", "secret", new CurrentUser("u1"));
        var result = await verifier.VerifyAsync(new Credential("alice", "wrong"));
        Assert.False(result.Succeeded);
        Assert.Equal(IdentityFailureReason.InvalidCredentials, result.Failure);
        Assert.Null(result.Value);
    }

    [Fact]
    public async Task Fake_external_provider_classifies_rejection_without_provider_response()
    {
        var result = await new FakeExternalIdentityProvider().AuthenticateAsync("missing");
        Assert.False(result.Succeeded);
        Assert.Equal(IdentityFailureReason.ProviderRejected, result.Failure);
        Assert.Null(result.Value);
    }

    [Fact]
    public void Permission_catalog_is_module_owned_and_keyed_by_resource_action()
    {
        var catalog = new PermissionCatalog().Register(new PermissionDefinition("reports", "read"));
        Assert.Equal("reports.read", catalog.All.Single().Key);
        Assert.NotNull(catalog.Find("REPORTS.READ"));
    }
}
