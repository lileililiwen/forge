using System.Net;
using Microsoft.AspNetCore.WebUtilities;

namespace Platform.Identity.OpenIdConnect.AspNetCore.Tests;

public class PlatformOpenIdConnectCallbackTests
{
    [Fact]
    public async Task A_valid_callback_signs_the_user_in()
    {
        await using var issuer = new LocalOidcTestIssuer();
        await using var app = await OidcTestApp.StartAsync(issuer);

        var challenge = await app.ChallengeAsync();
        Assert.Equal(HttpStatusCode.Found, challenge.StatusCode);

        var location = challenge.Headers.Location;
        Assert.NotNull(location);
        Assert.StartsWith(issuer.Issuer, location!.ToString(), StringComparison.Ordinal);

        var query = QueryHelpers.ParseQuery(location.Query);
        Assert.Equal("code", query["response_type"].ToString());
        Assert.Equal("S256", query["code_challenge_method"].ToString());
        Assert.NotEmpty(query["code_challenge"].ToString());
        Assert.NotEmpty(query["nonce"].ToString());
        Assert.NotEmpty(query["state"].ToString());

        var callback = await app.Client.GetAsync(issuer.CreateCallbackUrl(location));
        Assert.Equal(HttpStatusCode.Found, callback.StatusCode);

        var body = await app.WhoAmIAsync();
        using var document = System.Text.Json.JsonDocument.Parse(body);
        Assert.Equal(issuer.Subject, document.RootElement.GetProperty("subject").GetString());
        Assert.Equal(issuer.Issuer, document.RootElement.GetProperty("issuer").GetString());
        Assert.Equal(issuer.Subject, document.RootElement.GetProperty("externalSubject").GetString());
    }

    [Fact]
    public async Task A_token_signed_with_another_key_is_rejected()
    {
        await using var issuer = new LocalOidcTestIssuer { SignWithUnpublishedKey = true };
        await using var app = await OidcTestApp.StartAsync(issuer);

        var failure = await SignInAsync(app, issuer);

        Assert.Equal("InvalidResponse", failure.Kind);
    }

    [Fact]
    public async Task A_token_from_another_issuer_is_rejected()
    {
        await using var issuer = new LocalOidcTestIssuer { IssuerOverride = "https://other-issuer.test" };
        await using var app = await OidcTestApp.StartAsync(issuer);

        var failure = await SignInAsync(app, issuer);

        Assert.Equal("InvalidResponse", failure.Kind);
    }

    [Fact]
    public async Task A_token_for_another_audience_is_rejected()
    {
        await using var issuer = new LocalOidcTestIssuer { AudienceOverride = "another-client" };
        await using var app = await OidcTestApp.StartAsync(issuer);

        var failure = await SignInAsync(app, issuer);

        Assert.Equal("InvalidResponse", failure.Kind);
    }

    [Fact]
    public async Task A_token_with_another_nonce_is_rejected()
    {
        await using var issuer = new LocalOidcTestIssuer { NonceOverride = "another-nonce" };
        await using var app = await OidcTestApp.StartAsync(issuer);

        var failure = await SignInAsync(app, issuer);

        Assert.Equal("InvalidResponse", failure.Kind);
    }

    [Fact]
    public async Task An_expired_token_is_rejected()
    {
        await using var issuer = new LocalOidcTestIssuer { IssueExpiredToken = true };
        await using var app = await OidcTestApp.StartAsync(issuer);

        var failure = await SignInAsync(app, issuer);

        Assert.Equal("InvalidResponse", failure.Kind);
    }

    [Fact]
    public async Task A_replayed_callback_is_rejected()
    {
        await using var issuer = new LocalOidcTestIssuer();
        await using var app = await OidcTestApp.StartAsync(issuer);

        var challenge = await app.ChallengeAsync();
        var location = challenge.Headers.Location!;
        var callbackUrl = issuer.CreateCallbackUrl(location);

        var first = await app.Client.GetAsync(callbackUrl);
        Assert.Equal(HttpStatusCode.Found, first.StatusCode);

        var replay = await app.Client.GetAsync(callbackUrl);
        var body = await replay.Content.ReadAsStringAsync();

        Assert.Equal(HttpStatusCode.BadRequest, replay.StatusCode);
        Assert.StartsWith("oidc-failure:", body, StringComparison.Ordinal);
        Assert.Contains("InvalidResponse", body, StringComparison.Ordinal);

        var whoami = await app.WhoAmIAsync();
        using var document = System.Text.Json.JsonDocument.Parse(whoami);
        Assert.Equal(issuer.Subject, document.RootElement.GetProperty("subject").GetString());
    }

    [Fact]
    public async Task An_unreachable_provider_is_reported_as_unavailable()
    {
        await using var issuer = new LocalOidcTestIssuer();
        await using var app = await OidcTestApp.StartAsync(issuer, useFailoverHandler: true);

        var failure = await SignInAsync(app, issuer);

        Assert.Equal("ProviderUnavailable", failure.Kind);
    }

    [Fact]
    public async Task A_provider_server_error_is_reported_as_unavailable()
    {
        await using var issuer = new LocalOidcTestIssuer { TokenEndpointServerError = true };
        await using var app = await OidcTestApp.StartAsync(issuer);

        var failure = await SignInAsync(app, issuer);

        Assert.Equal("ProviderUnavailable", failure.Kind);
    }

    [Fact]
    public async Task A_provider_denial_is_reported_as_access_denied()
    {
        await using var issuer = new LocalOidcTestIssuer();
        await using var app = await OidcTestApp.StartAsync(issuer);

        var challenge = await app.ChallengeAsync();
        var callbackUrl = issuer.CreateErrorCallbackUrl(challenge.Headers.Location!, "access_denied");

        var response = await app.Client.GetAsync(callbackUrl);
        var body = await response.Content.ReadAsStringAsync();

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        Assert.Contains("AccessDenied", body, StringComparison.Ordinal);
    }

    [Fact]
    public async Task A_failure_never_leaks_the_code_or_the_token()
    {
        await using var issuer = new LocalOidcTestIssuer { SignWithUnpublishedKey = true };
        await using var app = await OidcTestApp.StartAsync(issuer);

        var challenge = await app.ChallengeAsync();
        var callbackUrl = issuer.CreateCallbackUrl(challenge.Headers.Location!);
        var response = await app.Client.GetAsync(callbackUrl);
        var body = await response.Content.ReadAsStringAsync();

        Assert.DoesNotContain("eyJ", body, StringComparison.Ordinal);
        Assert.DoesNotContain("code=", body, StringComparison.Ordinal);
        Assert.DoesNotContain(issuer.Subject, body, StringComparison.Ordinal);
    }

    private static async Task<(string Kind, string Body)> SignInAsync(OidcTestApp app, LocalOidcTestIssuer issuer)
    {
        var challenge = await app.ChallengeAsync();
        var callbackUrl = issuer.CreateCallbackUrl(challenge.Headers.Location!);
        var response = await app.Client.GetAsync(callbackUrl);
        var body = await response.Content.ReadAsStringAsync();

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        Assert.StartsWith("oidc-failure:", body, StringComparison.Ordinal);

        var parts = body.Split(':', StringSplitOptions.RemoveEmptyEntries);
        return (parts[1], body);
    }
}
