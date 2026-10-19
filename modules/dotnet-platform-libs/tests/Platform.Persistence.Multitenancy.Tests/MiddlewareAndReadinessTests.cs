using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Platform.Core.Tenancy;
using Platform.Persistence.Multitenancy;
using Platform.Persistence.Multitenancy.DependencyInjection;
using Platform.Persistence.Multitenancy.Readiness;

namespace Platform.Persistence.Multitenancy.Tests;

public sealed class MiddlewareAndReadinessTests
{
    [Fact]
    public async Task Middleware_installs_resolved_tenant_when_header_is_present()
    {
        var (host, _, _) = await CreateHostAsync();
        try
        {
            var server = host.GetTestServer();
            var request = new HttpRequestMessage(HttpMethod.Get, "/probe");
            request.Headers.Add("X-Tenant-Id", "tenant-a");
            var response = await server.CreateClient().SendAsync(request);
            response.EnsureSuccessStatusCode();
            var body = await response.Content.ReadAsStringAsync();
            Assert.Equal("tenant-a", body);
        }
        finally
        {
            await host.StopAsync();
        }
    }

    [Fact]
    public async Task Middleware_rejects_tenant_identifiers_above_the_bounded_length()
    {
        var (host, _, _) = await CreateHostAsync(maxTenantIdLength: 4);
        try
        {
            var server = host.GetTestServer();
            var request = new HttpRequestMessage(HttpMethod.Get, "/probe");
            request.Headers.Add("X-Tenant-Id", new string('a', 5));
            var response = await server.CreateClient().SendAsync(request);
            response.EnsureSuccessStatusCode();
            var body = await response.Content.ReadAsStringAsync();
            Assert.Equal("unresolved", body);
        }
        finally
        {
            await host.StopAsync();
        }
    }

    [Fact]
    public async Task Middleware_skips_installation_when_disabled()
    {
        var (host, _, _) = await CreateHostAsync(enableHttpScopeInstallation: false);
        try
        {
            var server = host.GetTestServer();
            var request = new HttpRequestMessage(HttpMethod.Get, "/probe");
            request.Headers.Add("X-Tenant-Id", "tenant-a");
            var response = await server.CreateClient().SendAsync(request);
            var body = await response.Content.ReadAsStringAsync();
            Assert.Equal("unresolved", body);
        }
        finally
        {
            await host.StopAsync();
        }
    }

    [Fact]
    public async Task Middleware_replaces_scope_per_request()
    {
        var (host, _, _) = await CreateHostAsync();
        try
        {
            var server = host.GetTestServer();
            var client = server.CreateClient();
            var first = new HttpRequestMessage(HttpMethod.Get, "/probe");
            first.Headers.Add("X-Tenant-Id", "tenant-a");
            var firstResponse = await client.SendAsync(first);
            Assert.Equal("tenant-a", await firstResponse.Content.ReadAsStringAsync());

            var second = new HttpRequestMessage(HttpMethod.Get, "/probe");
            second.Headers.Add("X-Tenant-Id", "tenant-b");
            var secondResponse = await client.SendAsync(second);
            Assert.Equal("tenant-b", await secondResponse.Content.ReadAsStringAsync());
        }
        finally
        {
            await host.StopAsync();
        }
    }

    [Fact]
    public async Task Readiness_check_reports_unhealthy_when_any_tenant_probe_fails()
    {
        var tenants = new ITenantInfo[] { new StaticTenant("tenant-a"), new StaticTenant("tenant-b") };
        var probe = new StaticProbe(("tenant-a", true, null), ("tenant-b", false, "connection refused"));
        var check = new TenantConnectionReadinessCheck(tenants, probe);

        var result = await check.CheckHealthAsync(new HealthCheckContext());

        Assert.Equal(HealthStatus.Unhealthy, result.Status);
        Assert.Contains("tenant-b", result.Description);
    }

    [Fact]
    public async Task Readiness_check_reports_healthy_when_all_probes_succeed()
    {
        var tenants = new ITenantInfo[] { new StaticTenant("tenant-a") };
        var probe = new StaticProbe(("tenant-a", true, null));
        var check = new TenantConnectionReadinessCheck(tenants, probe);

        var result = await check.CheckHealthAsync(new HealthCheckContext());

        Assert.Equal(HealthStatus.Healthy, result.Status);
    }

    [Fact]
    public void Readiness_check_registers_under_ready_tag_via_extension()
    {
        var tenants = new ITenantInfo[] { new StaticTenant("tenant-a") };
        var probe = new StaticProbe(("tenant-a", true, null));
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddHealthChecks().AddPlatformTenantConnectionReadinessCheck(tenants, probe);
        using var provider = services.BuildServiceProvider();
        var registrations = provider.GetRequiredService<Microsoft.Extensions.Options.IOptions<HealthCheckServiceOptions>>().Value.Registrations;

        var registration = Assert.Single(registrations);
        Assert.Contains("ready", registration.Tags);
    }

    private static async Task<(IHost Host, AmbientTenantScopeStore Store, IServiceProvider Services)> CreateHostAsync(
        int maxTenantIdLength = 128,
        bool enableHttpScopeInstallation = true)
    {
        var resolver = new HeaderTenantResolver();
        var host = await new HostBuilder()
            .ConfigureWebHost(web =>
            {
                web.UseTestServer();
                web.ConfigureServices(services =>
                {
                    services.AddPlatformPersistenceMultitenancy(options =>
                    {
                        options.MaxTenantIdLength = maxTenantIdLength;
                        options.EnableHttpScopeInstallation = enableHttpScopeInstallation;
                    });
                    services.AddSingleton<ITenantResolver>(resolver);
                });
                web.Configure(app =>
                {
                    app.UsePlatformMultitenancy();
                    app.Run(async context =>
                    {
                        var accessor = context.RequestServices.GetRequiredService<ITenantScopeAccessor>();
                        var resolved = accessor.Current.Status switch
                        {
                            TenantResolutionStatus.Resolved => accessor.Current.Tenant!.Id,
                            TenantResolutionStatus.GlobalOperation => "global",
                            TenantResolutionStatus.Unresolved when accessor.Current.Reason == "tenant_id_invalid" => "unresolved",
                            _ => "unresolved",
                        };
                        await context.Response.WriteAsync(resolved);
                    });
                });
            })
            .StartAsync();

        var store = host.Services.GetRequiredService<AmbientTenantScopeStore>();
        return (host, store, host.Services);
    }

    private sealed record StaticTenant(string Id) : ITenantInfo
    {
        public string? Name => Id;
    }

    private sealed class StaticProbe : ITenantConnectionReadinessProbe
    {
        private readonly Dictionary<string, (bool IsReady, string? Error)> _outcomes;

        public StaticProbe(params (string TenantId, bool IsReady, string? Error)[] entries)
        {
            _outcomes = entries.ToDictionary(e => e.TenantId, e => (e.IsReady, e.Error));
        }

        public ValueTask<TenantConnectionReadinessResult> ProbeAsync(ITenantInfo? tenant, CancellationToken cancellationToken = default)
        {
            if (tenant is null)
            {
                return ValueTask.FromResult(TenantConnectionReadinessResult.Failed("shared", "no shared probe"));
            }
            if (!_outcomes.TryGetValue(tenant.Id, out var outcome))
            {
                return ValueTask.FromResult(TenantConnectionReadinessResult.Failed(tenant.Id, "no probe configured"));
            }
            return ValueTask.FromResult(outcome.IsReady
                ? TenantConnectionReadinessResult.Ready(tenant.Id)
                : TenantConnectionReadinessResult.Failed(tenant.Id, outcome.Error ?? "unknown"));
        }
    }

    private sealed class HeaderTenantResolver : ITenantResolver
    {
        public ValueTask<TenantResolutionResult> ResolveAsync(object? context, CancellationToken cancellationToken = default)
        {
            if (context is HttpContext http
                && http.Request.Headers.TryGetValue("X-Tenant-Id", out var values)
                && !string.IsNullOrWhiteSpace(values))
            {
                return ValueTask.FromResult(TenantResolutionResult.Resolved(new StaticTenant(values.ToString())));
            }
            return ValueTask.FromResult(TenantResolutionResult.Unresolved("missing_tenant_header"));
        }
    }
}
