using System.Net.Http.Json;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Identity.AspNetCore;
using Platform.Identity.Contracts;
using Platform.Identity.Testing;

namespace Platform.Identity.Tests;

public sealed class IdentityLifecycleEndpointTests
{
    [Fact]
    public async Task Refresh_endpoint_rotates_a_valid_handle()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddSingleton<IRefreshTokenStore, InMemoryRefreshTokenStore>();
            services.AddPlatformIdentityLifecycle();
        });
        var client = app.GetTestClient();
        var store = app.Services.GetRequiredService<IRefreshTokenStore>();
        var issued = await store.IssueAsync("alice", "session-1", DateTimeOffset.UtcNow.AddMinutes(10));

        using var request = new HttpRequestMessage(HttpMethod.Post, "/platform/identity/refresh");
        request.Headers.Add("X-Refresh-Token", issued.Value!.Handle);
        var response = await client.SendAsync(request);

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
    }

    [Fact]
    public async Task Refresh_endpoint_rejects_replay_with_401()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddSingleton<IRefreshTokenStore, InMemoryRefreshTokenStore>();
            services.AddPlatformIdentityLifecycle();
        });
        var client = app.GetTestClient();
        var store = app.Services.GetRequiredService<IRefreshTokenStore>();
        var issued = await store.IssueAsync("alice", "session-1", DateTimeOffset.UtcNow.AddMinutes(10));
        await store.ConsumeAsync(issued.Value!.Handle);

        using var request = new HttpRequestMessage(HttpMethod.Post, "/platform/identity/refresh");
        request.Headers.Add("X-Refresh-Token", issued.Value.Handle);
        var response = await client.SendAsync(request);

        Assert.Equal(StatusCodes.Status401Unauthorized, (int)response.StatusCode);
    }

    [Fact]
    public async Task Password_recovery_initiation_responds_202_for_known_and_unknown_subjects()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddSingleton<IPasswordRecoveryService, FakePasswordRecoveryService>();
            services.AddPlatformIdentityLifecycle();
        });
        var client = app.GetTestClient();
        var known = await client.PostAsJsonAsync("/platform/identity/password-recovery", new { subjectIdentifier = "alice" });
        var unknown = await client.PostAsJsonAsync("/platform/identity/password-recovery", new { subjectIdentifier = "bob" });
        Assert.Equal(StatusCodes.Status202Accepted, (int)known.StatusCode);
        Assert.Equal(StatusCodes.Status202Accepted, (int)unknown.StatusCode);
    }

    [Fact]
    public async Task Two_factor_verify_endpoint_returns_204_for_known_code()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddSingleton<ITwoFactorService, FakeTwoFactorService>();
            services.AddPlatformIdentityLifecycle();
        });
        var client = app.GetTestClient();
        var issue = await client.PostAsJsonAsync("/platform/identity/two-factor", new { subjectId = "alice" });
        Assert.Equal(StatusCodes.Status202Accepted, (int)issue.StatusCode);
        var body = await issue.Content.ReadFromJsonAsync<Dictionary<string, string>>();
        Assert.NotNull(body);
        var verify = await client.PostAsJsonAsync("/platform/identity/two-factor/verify", new { challengeId = body!["challengeId"], code = "000000" });
        Assert.Equal(StatusCodes.Status204NoContent, (int)verify.StatusCode);
    }

    [Fact]
    public async Task Impersonation_endpoint_fails_closed_without_a_policy()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddSingleton<ICurrentUserAccessor>(new FakeCurrentUserAccessor(new CurrentUser("alice")));
            services.AddSingleton<IImpersonationService>(new FakeImpersonationService(new DenyAllImpersonationPolicy()));
            services.AddPlatformIdentityLifecycle();
        });
        var client = app.GetTestClient();
        var response = await client.PostAsJsonAsync("/platform/identity/impersonation", new { targetSubjectId = "bob", reason = "support", durationMinutes = 15 });
        Assert.Equal(StatusCodes.Status409Conflict, (int)response.StatusCode);
    }

    private static WebApplication BuildApp(Action<IServiceCollection>? services = null)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        services?.Invoke(builder.Services);
        var app = builder.Build();
        app.MapPlatformRefreshTokenRotation();
        app.MapPlatformRefreshTokenRevocation();
        app.MapPlatformPasswordRecoveryInitiation();
        app.MapPlatformPasswordRecoveryCompletion();
        app.MapPlatformTwoFactorChallenge();
        app.MapPlatformTwoFactorVerification();
        app.MapPlatformImpersonationStart();
        app.MapPlatformImpersonationEnd();
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
