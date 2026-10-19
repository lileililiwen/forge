using System.Net;
using System.Text.Json;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Time;
using Platform.Quota.AspNetCore.Contracts;
using Platform.Quota.AspNetCore.DependencyInjection;
using Platform.Quota.Contracts;
using Platform.Quota.DependencyInjection;
using Platform.Quota.Stores;

namespace Platform.Quota.AspNetCore.Tests;

public class QuotaEnforcementTests : IDisposable
{
    private readonly List<WebApplication> _apps = new();

    public void Dispose()
    {
        foreach (var app in _apps)
        {
            app.StopAsync().GetAwaiter().GetResult();
            app.DisposeAsync().GetAwaiter().GetResult();
        }
    }

    private WebApplication Start(WebApplication app)
    {
        _apps.Add(app);
        return app;
    }

    private static QuotaRequest AllowedRequest(long limit = 10, long amount = 1, bool reserve = false) =>
        new(new QuotaResource("api"), limit, amount, TestDoubles.UtcWindow(), reserve);

    [Fact]
    public async Task Allowed_request_proceeds_and_records_operation()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)), initialConsumed: 0);
        var app = Start(TestDoubles.BuildApp(
            store,
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { AllowedRequest() })));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Contains("allowed", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Exceeded_request_returns_429_with_safe_metadata_and_retry_after()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)), initialConsumed: 10);
        var app = Start(TestDoubles.BuildApp(
            store,
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { new QuotaRequest(new QuotaResource("api"), 10, 1, TestDoubles.UtcWindow()) })));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.TooManyRequests, response.StatusCode);
        Assert.Equal("application/problem+json", response.Content.Headers.ContentType?.MediaType);

        var retryAfter = response.Headers.RetryAfter;
        Assert.NotNull(retryAfter);
        Assert.True(retryAfter.Delta > TimeSpan.Zero);

        var body = JsonDocument.Parse(await response.Content.ReadAsStringAsync()).RootElement;
        Assert.Equal(429, body.GetProperty("status").GetInt32());
        Assert.Equal("api", body.GetProperty("resource").GetString());
        Assert.Equal(10, body.GetProperty("limit").GetInt64());
        Assert.Equal(10, body.GetProperty("usage").GetInt64());
        Assert.Equal(1, body.GetProperty("requested").GetInt64());
        Assert.True(body.TryGetProperty("traceId", out _));
        Assert.True(body.TryGetProperty("correlationId", out _));
        Assert.True(body.TryGetProperty("resetAtUtc", out _));
    }

    [Fact]
    public async Task Missing_subject_fails_closed_when_policy_is_fail_closed()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)));
        var app = Start(TestDoubles.BuildApp(
            store,
            new FixedSubjectResolver(null),
            new FixedResourceResolver(new[] { AllowedRequest() }),
            configure: o => o.MissingContextPolicy = MissingContextPolicy.FailClosed));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);
        var body = JsonDocument.Parse(await response.Content.ReadAsStringAsync()).RootElement;
        Assert.Equal("Quota Context Required", body.GetProperty("title").GetString());
        Assert.True(body.TryGetProperty("traceId", out _));
    }

    [Fact]
    public async Task Missing_subject_proceeds_when_policy_is_allow()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)));
        var app = Start(TestDoubles.BuildApp(
            store,
            new FixedSubjectResolver(null),
            new FixedResourceResolver(new[] { AllowedRequest() }),
            configure: o =>
            {
                o.MissingContextPolicy = MissingContextPolicy.Allow;
                o.AnonymousSubject = "anonymous";
            }));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
    }

    [Fact]
    public async Task Health_probe_is_exempt_and_bypasses_quota()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)), initialConsumed: 1_000_000);
        var app = Start(TestDoubles.BuildApp(
            store,
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { new QuotaRequest(new QuotaResource("api"), 10, 1, TestDoubles.UtcWindow()) })));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/health");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Contains("healthy", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Endpoint_metadata_exemption_bypasses_quota()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)), initialConsumed: 1_000_000);
        var app = Start(TestDoubles.BuildApp(
            store,
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { new QuotaRequest(new QuotaResource("api"), 10, 1, TestDoubles.UtcWindow()) })));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/exempt");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
    }

    [Fact]
    public async Task Configured_method_exemption_bypasses_quota()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)), initialConsumed: 1_000_000);
        var app = Start(TestDoubles.BuildApp(
            store,
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { new QuotaRequest(new QuotaResource("api"), 10, 1, TestDoubles.UtcWindow()) }),
            configure: o => o.ExemptMethods.Add("POST")));

        using var client = app.GetTestClient();
        var response = await client.PostAsync("/write", null);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
    }

    [Fact]
    public async Task Provider_unavailable_fails_closed_when_policy_is_fail_closed()
    {
        var app = Start(TestDoubles.BuildApp(
            new ThrowingQuotaStore(),
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { AllowedRequest() }),
            configure: o => o.ProviderUnavailablePolicy = QuotaUnavailablePolicy.FailClosed));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
        var body = JsonDocument.Parse(await response.Content.ReadAsStringAsync()).RootElement;
        Assert.Equal("Quota Unavailable", body.GetProperty("title").GetString());
    }

    [Fact]
    public async Task Provider_unavailable_proceeds_when_policy_is_allow()
    {
        var app = Start(TestDoubles.BuildApp(
            new ThrowingQuotaStore(),
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { AllowedRequest() }),
            configure: o => o.ProviderUnavailablePolicy = QuotaUnavailablePolicy.Allow));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
    }

    [Fact]
    public async Task Unconfigured_resource_resolver_fails_closed()
    {
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformQuota();
        builder.Services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)));
        builder.Services.AddPlatformQuotaAspNetCore();
        builder.Services.AddSingleton<IQuotaSubjectResolver>(new FixedSubjectResolver(new QuotaSubject("sub1")));
        var app = Start(builder.Build());
        app.UsePlatformQuota();
        app.MapGet("/resource", () => Results.Ok());
        await app.StartAsync();

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
    }

    [Fact]
    public async Task Reserve_mode_settles_quota_on_success()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)), initialConsumed: 0);
        var app = Start(TestDoubles.BuildApp(
            store,
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { AllowedRequest(reserve: true) })));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var snapshot = await store.GetSnapshotAsync(new QuotaSubject("sub1"), new QuotaResource("api"), TestDoubles.UtcWindow(), 10);
        Assert.Equal(1, snapshot.Consumed);
    }

    [Fact]
    public async Task Reserve_mode_releases_quota_when_downstream_fails()
    {
        var store = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)), initialConsumed: 0);
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformQuota();
        builder.Services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)));
        builder.Services.AddPlatformQuotaAspNetCore();
        builder.Services.AddSingleton<IQuotaSubjectResolver>(new FixedSubjectResolver(new QuotaSubject("sub1")));
        builder.Services.AddSingleton<IQuotaResourceResolver>(new FixedResourceResolver(new[] { AllowedRequest(reserve: true) }));
        var app = Start(builder.Build());
        app.UsePlatformQuota();
        app.MapGet("/boom", (HttpContext httpContext) => throw new InvalidOperationException("boom"));
        await app.StartAsync();

        using var client = app.GetTestClient();
        await Assert.ThrowsAnyAsync<Exception>(() => client.GetAsync("/boom"));

        var snapshot = await store.GetSnapshotAsync(new QuotaSubject("sub1"), new QuotaResource("api"), TestDoubles.UtcWindow(), 10);
        Assert.Equal(0, snapshot.Consumed);
    }

    [Fact]
    public async Task Cancellation_token_is_threaded_into_the_store()
    {
        var inner = new InMemoryQuotaStore(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)));
        var observing = new TokenObservingStore(inner);
        var app = Start(TestDoubles.BuildApp(
            observing,
            new FixedSubjectResolver(new QuotaSubject("sub1")),
            new FixedResourceResolver(new[] { AllowedRequest() })));

        using var client = app.GetTestClient();
        var response = await client.GetAsync("/resource");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.NotNull(observing.LastToken);
        Assert.False(observing.LastToken!.Value.IsCancellationRequested);
    }
}
