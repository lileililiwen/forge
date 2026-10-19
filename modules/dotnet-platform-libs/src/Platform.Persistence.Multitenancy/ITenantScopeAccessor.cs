using Platform.Core.Tenancy;

namespace Platform.Persistence.Multitenancy;

/// <summary>
/// Resolves the current ambient tenant scope without coupling
/// consumers to the <see cref="AmbientTenantScopeStore"/> singleton.
/// </summary>
public interface ITenantScopeAccessor
{
    /// <summary>Gets the current ambient scope.</summary>
    IAmbientTenantScope Current { get; }
}

/// <summary>Default accessor that proxies <see cref="AmbientTenantScopeStore"/>.</summary>
public sealed class TenantScopeAccessor : ITenantScopeAccessor
{
    private readonly AmbientTenantScopeStore _store;

    /// <summary>Creates a new accessor bound to the supplied store.</summary>
    public TenantScopeAccessor(AmbientTenantScopeStore store)
    {
        _store = store ?? throw new ArgumentNullException(nameof(store));
    }

    /// <inheritdoc />
    public IAmbientTenantScope Current => _store.Current;
}
