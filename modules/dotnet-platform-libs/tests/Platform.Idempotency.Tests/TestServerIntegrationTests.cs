using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Idempotency.DependencyInjection;

namespace Platform.Idempotency.Tests;

public class TestServerIntegrationTests
{
    [Fact]
    public async Task Host_with_AddPlatformIdempotency_resolves_default_options()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"retentionSeconds\":86400", body);
        Assert.Contains("\"maxKeyLength\":256", body);
    }

    [Fact]
    public async Task Host_with_configured_section_overrides_defaults()
    {
        await using var app = BuildApp(extraConfiguration: new Dictionary<string, string?>
        {
            ["Idempotency:RetentionSeconds"] = "120",
            ["Idempotency:MaxKeyLength"] = "64",
        });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"retentionSeconds\":120", body);
        Assert.Contains("\"maxKeyLength\":64", body);
    }

    [Fact]
    public async Task Host_save_and_try_get_round_trip()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var saveResponse = await client.PostAsync("/save?key=key-1", content: null);
        Assert.Equal(StatusCodes.Status200OK, (int)saveResponse.StatusCode);

        var getResponse = await client.GetAsync("/try-get?key=key-1");
        Assert.Equal(StatusCodes.Status200OK, (int)getResponse.StatusCode);
        var body = await getResponse.Content.ReadAsStringAsync();
        Assert.Contains("\"key\":\"key-1\"", body);
    }

    [Fact]
    public async Task Host_eviction_sweep_removes_expired_record()
    {
        await using var app = BuildApp(extraConfiguration: new Dictionary<string, string?>
        {
            ["Idempotency:RetentionSeconds"] = "60",
        });
        var client = app.GetTestClient();

        var saveResponse = await client.PostAsync("/save?key=stale", content: null);
        Assert.Equal(StatusCodes.Status200OK, (int)saveResponse.StatusCode);

        var advanceResponse = await client.PostAsync("/advance-seconds?seconds=61", content: null);
        Assert.Equal(StatusCodes.Status200OK, (int)advanceResponse.StatusCode);

        var evictResponse = await client.PostAsync("/evict", content: null);
        Assert.Equal(StatusCodes.Status200OK, (int)evictResponse.StatusCode);
        var evictBody = await evictResponse.Content.ReadAsStringAsync();
        Assert.Contains("\"removed\":1", evictBody);

        var getResponse = await client.GetAsync("/try-get?key=stale");
        Assert.Equal(StatusCodes.Status200OK, (int)getResponse.StatusCode);
        var getBody = await getResponse.Content.ReadAsStringAsync();
        Assert.Contains("\"hit\":false", getBody);
    }

    [Fact]
    public async Task Host_disabled_registration_uses_no_op_store()
    {
        await using var app = BuildApp(extraConfiguration: new Dictionary<string, string?>
        {
            ["Idempotency:Enabled"] = "false",
        });
        var client = app.GetTestClient();

        var saveResponse = await client.PostAsync("/save?key=key-1", content: null);
        Assert.Equal(StatusCodes.Status200OK, (int)saveResponse.StatusCode);

        var getResponse = await client.GetAsync("/try-get?key=key-1");
        Assert.Equal(StatusCodes.Status200OK, (int)getResponse.StatusCode);
        var body = await getResponse.Content.ReadAsStringAsync();
        Assert.Contains("\"hit\":false", body);
    }

    private static WebApplication BuildApp(
        IReadOnlyDictionary<string, string?>? extraConfiguration = null)
    {
        var options = new WebApplicationOptions
        {
            EnvironmentName = "Testing",
        };
        var builder = WebApplication.CreateBuilder(options);
        builder.WebHost.UseTestServer();
        if (extraConfiguration is not null)
        {
            builder.Configuration.AddInMemoryCollection(extraConfiguration);
        }

        builder.Services.AddPlatformIdempotency(idempotencyOptions =>
            builder.Configuration
                .GetSection(IdempotencyOptions.SectionName)
                .Bind(idempotencyOptions));
        var clock = new MutableClock();
        builder.Services.AddSingleton<MutableClock>(clock);
        builder.Services.AddSingleton<Platform.Core.Time.IClock>(clock);

        var app = builder.Build();
        app.MapGet("/probe", (IOptions<IdempotencyOptions> options) => new
        {
            retentionSeconds = options.Value.RetentionSeconds,
            maxKeyLength = options.Value.MaxKeyLength,
        });
        app.MapPost("/save", async (
            string key,
            [FromServices] IIdempotencyStore store,
            [FromServices] MutableClock clock) =>
        {
            var record = new IdempotencyRecord(
                Key: key,
                Fingerprint: "fp-1",
                StatusCode: 200,
                ContentType: "application/json",
                ResponseBody: "{}",
                CreatedAt: clock.UtcNow);
            await store.SaveAsync(record);
            return Results.Ok(new { saved = true });
        });
        app.MapGet("/try-get", async (
            string key,
            [FromServices] IIdempotencyStore store) =>
        {
            var record = await store.TryGetAsync(key);
            return Results.Ok(new
            {
                hit = record is not null,
                key = record?.Key,
            });
        });
        app.MapPost("/advance-seconds", (int seconds, [FromServices] MutableClock mutableClock) =>
        {
            mutableClock.Advance(TimeSpan.FromSeconds(seconds));
            return Results.Ok(new { advanced = seconds });
        });
        app.MapPost("/evict", async ([FromServices] IIdempotencyStore store) =>
        {
            var removed = await store.EvictExpiredAsync();
            return Results.Ok(new { removed });
        });

        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
