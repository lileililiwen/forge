using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Tenancy;
using Platform.Persistence.Multitenancy;
using Platform.Persistence.Multitenancy.DependencyInjection;

namespace Platform.Persistence.Multitenancy.Tests;

public sealed class OptionsTests
{
    [Fact]
    public void Defaults_are_safe_and_bounded()
    {
        var options = new MultitenancyOptions();

        Assert.False(options.EnableDefaultTenantIsolation);
        Assert.True(options.EnableHttpScopeInstallation);
        Assert.True(options.FailClosedOnMissingScope);
        Assert.Equal(128, options.MaxTenantIdLength);
        Assert.Equal("X-Tenant-Id", options.TenantHeader);
        Assert.Equal("tenant_id", options.TenantClaim);
    }

    [Fact]
    public void Validation_reports_invalid_max_length()
    {
        var options = new MultitenancyOptions { MaxTenantIdLength = 0 };
        var errors = options.Validate();

        Assert.Contains("MaxTenantIdLength must be between 1 and 1024.", errors);
    }

    [Fact]
    public void Validation_reports_missing_header()
    {
        var options = new MultitenancyOptions { TenantHeader = " " };
        var errors = options.Validate();

        Assert.Contains("TenantHeader is required.", errors);
    }

    [Fact]
    public void Validation_reports_missing_claim()
    {
        var options = new MultitenancyOptions { TenantClaim = "" };
        var errors = options.Validate();

        Assert.Contains("TenantClaim is required.", errors);
    }

    [Fact]
    public void Registration_replaces_default_factory()
    {
        var services = new ServiceCollection();
        services.AddPlatformPersistenceMultitenancy();
        services.AddSingleton<ITenantScopeFactory, CustomFactory>();
        using var provider = services.BuildServiceProvider();

        Assert.IsType<CustomFactory>(provider.GetRequiredService<ITenantScopeFactory>());
    }

    [Fact]
    public void Registration_exposes_accessor_and_store()
    {
        var services = new ServiceCollection();
        services.AddPlatformPersistenceMultitenancy();
        using var provider = services.BuildServiceProvider();

        var store = provider.GetRequiredService<AmbientTenantScopeStore>();
        var accessor = provider.GetRequiredService<ITenantScopeAccessor>();
        Assert.Same(store.Current, accessor.Current);
    }

    private sealed class CustomFactory(AmbientTenantScopeStore store) : ITenantScopeFactory
    {
        public IDisposable BeginGlobalOperation(string reason)
        {
            var previous = store.Current;
            store.Replace(new AmbientTenantScope
            {
                Status = TenantResolutionStatus.GlobalOperation,
                Reason = reason,
            });
            return new Restorer(store, previous);
        }

        public IDisposable BeginTenant(ITenantInfo tenant)
        {
            var previous = store.Current;
            store.Replace(new AmbientTenantScope
            {
                Status = TenantResolutionStatus.Resolved,
                Tenant = tenant,
            });
            return new Restorer(store, previous);
        }

        private sealed class Restorer(AmbientTenantScopeStore store, AmbientTenantScope previous) : IDisposable
        {
            private bool _disposed;
            public void Dispose()
            {
                if (_disposed) return;
                _disposed = true;
                store.Replace(previous);
            }
        }
    }
}
