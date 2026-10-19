using System.Security.Claims;
using System.Text.Json;
using Microsoft.AspNetCore.Authentication;
using Microsoft.Extensions.DependencyInjection;

namespace Platform.Identity.OpenIdConnect.AspNetCore.Tests;

public class PlatformOpenIdConnectOwnershipTests
{
    [Fact]
    public async Task A_provider_role_grants_no_local_permission_without_an_explicit_mapping()
    {
        await using var issuer = new LocalOidcTestIssuer();
        issuer.ExtraClaims[ClaimTypes.Role] = "admin";
        await using var app = await OidcTestApp.StartAsync(issuer);

        var whoami = await SignInAsync(app, issuer);

        using var document = JsonDocument.Parse(whoami);
        var roles = document.RootElement.GetProperty("roles");
        Assert.Equal(0, roles.GetArrayLength());
    }

    [Fact]
    public async Task An_explicit_role_mapping_lets_the_application_map_a_provider_role()
    {
        await using var issuer = new LocalOidcTestIssuer();
        issuer.ExtraClaims[ClaimTypes.Role] = "admin";
        await using var app = await OidcTestApp.StartAsync(
            issuer,
            options => options.RoleClaimType = ClaimTypes.Role);

        var whoami = await SignInAsync(app, issuer);

        using var document = JsonDocument.Parse(whoami);
        var roles = document.RootElement.GetProperty("roles");
        Assert.Equal(1, roles.GetArrayLength());
        Assert.Equal("admin", roles[0].GetString());
    }

    [Fact]
    public void An_application_without_the_adapter_has_no_oidc_scheme()
    {
        var services = new ServiceCollection();
        services.AddAuthentication();
        using var provider = services.BuildServiceProvider();

        var schemes = provider.GetRequiredService<IAuthenticationSchemeProvider>();
        var scheme = schemes.GetSchemeAsync(PlatformOpenIdConnectOptions.DefaultSchemeName).GetAwaiter().GetResult();

        Assert.Null(scheme);
    }

    [Fact]
    public void The_adapter_registers_its_scheme_only_when_the_application_opts_in()
    {
        var services = new ServiceCollection();
        services.AddAuthentication();
        services.AddPlatformOpenIdConnect(options =>
        {
            options.Issuer = "https://issuer.test";
            options.ClientId = "client";
            options.SignInScheme = "Cookies";
            options.Scopes = new[] { "openid" };
        });
        using var provider = services.BuildServiceProvider();

        var schemes = provider.GetRequiredService<IAuthenticationSchemeProvider>();
        var scheme = schemes.GetSchemeAsync(PlatformOpenIdConnectOptions.DefaultSchemeName).GetAwaiter().GetResult();

        Assert.NotNull(scheme);
    }

    [Fact]
    public async Task A_linked_identity_signs_in_only_its_own_application_user()
    {
        await using var issuer = new LocalOidcTestIssuer();
        var store = new InMemoryIdentityStore();
        store.Link(issuer.Issuer, issuer.Subject, "local-user-7");

        await using var app = await OidcTestApp.StartAsync(
            issuer,
            options => options.IdentityResolution = PlatformOpenIdConnectIdentityResolutionMode.Required,
            configureServices: services => services.AddSingleton<IPlatformOpenIdConnectIdentityResolver>(store));

        var whoami = await SignInAsync(app, issuer);

        Assert.Equal("local-user-7", store.LastResolvedLocalSubject);
        Assert.Contains(issuer.Subject, whoami, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Enrollment_that_is_disabled_denies_the_sign_in_without_creating_a_user()
    {
        await using var issuer = new LocalOidcTestIssuer();
        var store = new InMemoryIdentityStore();

        await using var app = await OidcTestApp.StartAsync(
            issuer,
            options => options.IdentityResolution = PlatformOpenIdConnectIdentityResolutionMode.Required,
            configureServices: services => services.AddSingleton<IPlatformOpenIdConnectIdentityResolver>(store));

        var challenge = await app.ChallengeAsync();
        var response = await app.Client.GetAsync(issuer.CreateCallbackUrl(challenge.Headers.Location!));
        var body = await response.Content.ReadAsStringAsync();

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        Assert.Contains("ApplicationMapping", body, StringComparison.Ordinal);
        Assert.Equal(0, store.LinkCount);
        Assert.Equal(0, store.CreatedUserCount);
    }

    [Fact]
    public async Task A_duplicate_link_conflict_is_rejected_without_merging_accounts()
    {
        await using var issuer = new LocalOidcTestIssuer();
        var store = new InMemoryIdentityStore { ConflictOnResolve = true };

        await using var app = await OidcTestApp.StartAsync(
            issuer,
            options => options.IdentityResolution = PlatformOpenIdConnectIdentityResolutionMode.Required,
            configureServices: services => services.AddSingleton<IPlatformOpenIdConnectIdentityResolver>(store));

        var challenge = await app.ChallengeAsync();
        var response = await app.Client.GetAsync(issuer.CreateCallbackUrl(challenge.Headers.Location!));
        var body = await response.Content.ReadAsStringAsync();

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        Assert.Contains("duplicate_link", body, StringComparison.Ordinal);
        Assert.Equal(0, store.CreatedUserCount);
    }

    [Fact]
    public async Task Two_applications_resolve_the_same_subject_from_their_own_stores()
    {
        await using var issuer = new LocalOidcTestIssuer();
        var firstStore = new InMemoryIdentityStore();
        var secondStore = new InMemoryIdentityStore();
        firstStore.Link(issuer.Issuer, issuer.Subject, "first-local-user");
        secondStore.Link(issuer.Issuer, issuer.Subject, "second-local-user");

        await using var first = await OidcTestApp.StartAsync(
            issuer,
            options => options.IdentityResolution = PlatformOpenIdConnectIdentityResolutionMode.Required,
            configureServices: services => services.AddSingleton<IPlatformOpenIdConnectIdentityResolver>(firstStore));
        await using var second = await OidcTestApp.StartAsync(
            issuer,
            options => options.IdentityResolution = PlatformOpenIdConnectIdentityResolutionMode.Required,
            configureServices: services => services.AddSingleton<IPlatformOpenIdConnectIdentityResolver>(secondStore));

        await SignInAsync(first, issuer);
        await SignInAsync(second, issuer);

        Assert.Equal("first-local-user", firstStore.LastResolvedLocalSubject);
        Assert.Equal("second-local-user", secondStore.LastResolvedLocalSubject);
        Assert.Equal(1, firstStore.LinkCount);
        Assert.Equal(1, secondStore.LinkCount);
        Assert.NotSame(firstStore, secondStore);
    }

    [Fact]
    public async Task The_adapter_creates_no_local_identity_of_its_own()
    {
        await using var issuer = new LocalOidcTestIssuer();
        await using var app = await OidcTestApp.StartAsync(issuer);

        await SignInAsync(app, issuer);

        var userTypes = typeof(PlatformOpenIdConnectOptions).Assembly
            .GetExportedTypes()
            .Where(type => type.Name.Contains("User", StringComparison.Ordinal)
                || type.Name.Contains("Enroll", StringComparison.Ordinal)
                || type.Name.Contains("Store", StringComparison.Ordinal))
            .Select(type => type.Name)
            .ToArray();

        Assert.Empty(userTypes);
    }

    private static async Task<string> SignInAsync(OidcTestApp app, LocalOidcTestIssuer issuer)
    {
        var challenge = await app.ChallengeAsync();
        var response = await app.Client.GetAsync(issuer.CreateCallbackUrl(challenge.Headers.Location!));
        Assert.Equal(System.Net.HttpStatusCode.Found, response.StatusCode);
        return await app.WhoAmIAsync();
    }
}

/// <summary>An application-owned identity store used by the ownership tests.</summary>
internal sealed class InMemoryIdentityStore : IPlatformOpenIdConnectIdentityResolver
{
    private readonly Dictionary<string, string> _links = new(StringComparer.Ordinal);

    public bool ConflictOnResolve { get; set; }

    public int LinkCount => _links.Count;

    public int CreatedUserCount { get; private set; }

    public string? LastResolvedLocalSubject { get; private set; }

    public void Link(string issuer, string subject, string localSubject) =>
        _links[Key(issuer, subject)] = localSubject;

    public Task<PlatformOpenIdConnectIdentityResolution> ResolveAsync(
        PlatformOpenIdConnectExternalIdentity identity,
        ClaimsPrincipal principal,
        CancellationToken cancellationToken)
    {
        if (ConflictOnResolve)
        {
            return Task.FromResult(PlatformOpenIdConnectIdentityResolution.Conflict("duplicate_link"));
        }

        if (!_links.TryGetValue(Key(identity.Issuer, identity.Subject), out var localSubject))
        {
            return Task.FromResult(PlatformOpenIdConnectIdentityResolution.Denied("enrollment_disabled"));
        }

        LastResolvedLocalSubject = localSubject;
        return Task.FromResult(PlatformOpenIdConnectIdentityResolution.Authenticated(localSubject));
    }

    private static string Key(string issuer, string subject) => $"{issuer}|{subject}";
}
