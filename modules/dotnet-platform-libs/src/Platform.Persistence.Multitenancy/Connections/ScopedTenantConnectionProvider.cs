using Platform.Core.Tenancy;

namespace Platform.Persistence.Multitenancy.Connections;

/// <summary>
/// Exposes the connection descriptor that the application-owned
/// resolver selected for the current tenant. The provider resolves
/// the descriptor lazily and caches it for the lifetime of a single
/// ambient scope so participating <c>DbContext</c> instances share
/// the same connection and can enroll in a shared transaction. The
/// cache is invalidated automatically when the ambient scope changes.
/// </summary>
public sealed class ScopedTenantConnectionProvider : IDisposable
{
    private readonly AmbientTenantScopeStore _store;
    private readonly ITenantConnectionResolver _resolver;
    private readonly MultitenancyOptions _options;
    private readonly SemaphoreSlim _gate = new(1, 1);
    private TenantConnectionDescriptor? _cached;
    private int _cachedScopeGeneration;
    private bool _disposed;

    /// <summary>Creates a new provider bound to the supplied scope and resolver.</summary>
    public ScopedTenantConnectionProvider(
        AmbientTenantScopeStore store,
        ITenantConnectionResolver resolver,
        MultitenancyOptions options)
    {
        _store = store ?? throw new ArgumentNullException(nameof(store));
        _resolver = resolver ?? throw new ArgumentNullException(nameof(resolver));
        _options = options ?? throw new ArgumentNullException(nameof(options));
    }

    /// <summary>
    /// Gets the connection descriptor for the current ambient scope.
    /// Throws <see cref="TenantScopeNotResolvedException"/> when no
    /// tenant is resolved and fail-closed is enabled.
    /// </summary>
    public async ValueTask<TenantConnectionDescriptor> GetConnectionAsync(CancellationToken cancellationToken = default)
    {
        ThrowIfDisposed();
        var scope = _store.Current;
        var generation = scope.Generation;
        if (_cached is not null && _cachedScopeGeneration == generation) return _cached;
        await _gate.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (_cached is not null && _cachedScopeGeneration == generation) return _cached;
            if (scope.Status == TenantResolutionStatus.Unresolved)
            {
                if (_options.FailClosedOnMissingScope)
                    throw new TenantScopeNotResolvedException("ambient_scope_unresolved");
                _cached = await _resolver.ResolveConnectionAsync(null, cancellationToken);
                _cachedScopeGeneration = generation;
                return _cached;
            }
            _cached = await _resolver.ResolveConnectionAsync(scope.Tenant, cancellationToken);
            _cachedScopeGeneration = generation;
            return _cached;
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <inheritdoc />
    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _gate.Dispose();
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }
}
