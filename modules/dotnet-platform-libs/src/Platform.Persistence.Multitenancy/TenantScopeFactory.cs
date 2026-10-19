using Platform.Core.Tenancy;

namespace Platform.Persistence.Multitenancy;

/// <summary>
/// Default <see cref="ITenantScopeFactory"/> implementation. Each call
/// to <see cref="BeginGlobalOperation"/> or <see cref="BeginTenant"/>
/// snapshots the current <see cref="AmbientTenantScope"/>, replaces it
/// with a fresh instance configured for the requested state, and
/// returns a disposable that restores the prior scope on disposal.
/// </summary>
public sealed class TenantScopeFactory : ITenantScopeFactory
{
    private readonly AmbientTenantScopeStore _store;
    private readonly MultitenancyOptions _options;

    /// <summary>Creates a new factory bound to the supplied store and options.</summary>
    public TenantScopeFactory(AmbientTenantScopeStore store, MultitenancyOptions options)
    {
        _store = store ?? throw new ArgumentNullException(nameof(store));
        _options = options ?? throw new ArgumentNullException(nameof(options));
    }

    /// <inheritdoc />
    public IDisposable BeginGlobalOperation(string reason)
    {
        if (string.IsNullOrWhiteSpace(reason))
            throw new ArgumentException("A short, stable reason is required.", nameof(reason));
        return Install(new AmbientTenantScope
        {
            Status = TenantResolutionStatus.GlobalOperation,
            Reason = reason,
        });
    }

    /// <inheritdoc />
    public IDisposable BeginTenant(ITenantInfo tenant)
    {
        ArgumentNullException.ThrowIfNull(tenant);
        if (string.IsNullOrWhiteSpace(tenant.Id))
            throw new ArgumentException("Tenant.Id is required.", nameof(tenant));
        if (tenant.Id.Length > _options.MaxTenantIdLength)
            throw new ArgumentException(
                $"Tenant.Id exceeds MaxTenantIdLength ({_options.MaxTenantIdLength}).",
                nameof(tenant));
        return Install(new AmbientTenantScope
        {
            Status = TenantResolutionStatus.Resolved,
            Tenant = tenant,
        });
    }

    private Restorer Install(AmbientTenantScope next)
    {
        var previous = _store.Current;
        _store.Replace(next);
        return new Restorer(_store, previous);
    }

    private sealed class Restorer : IDisposable
    {
        private readonly AmbientTenantScopeStore _store;
        private AmbientTenantScope? _previous;
        private bool _disposed;

        public Restorer(AmbientTenantScopeStore store, AmbientTenantScope previous)
        {
            _store = store;
            _previous = previous;
        }

        public void Dispose()
        {
            if (_disposed) return;
            _disposed = true;
            if (_previous is null)
            {
                _store.Current.Reset();
            }
            else
            {
                _store.Replace(_previous);
                _previous = null;
            }
        }
    }
}
