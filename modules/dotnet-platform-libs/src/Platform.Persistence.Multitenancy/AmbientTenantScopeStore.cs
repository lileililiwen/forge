namespace Platform.Persistence.Multitenancy;

/// <summary>
/// Scoped store that exposes the current <see cref="AmbientTenantScope"/>
/// to tenant-aware collaborators. The store itself is a singleton that
/// holds an <see cref="AsyncLocal{T}"/> so HTTP middleware, background
/// handlers, and explicit <c>using</c> blocks cooperate without sharing
/// mutable state across concurrent operations.
/// </summary>
public sealed class AmbientTenantScopeStore
{
    private readonly AsyncLocal<AmbientTenantScope?> _current = new();
    private int _replacementCounter;

    /// <summary>
    /// Gets the current scope. When no scope has been installed the
    /// returned instance reports
    /// <see cref="Platform.Core.Tenancy.TenantResolutionStatus.Unresolved"/>.
    /// </summary>
    public AmbientTenantScope Current
    {
        get
        {
            var current = _current.Value;
            if (current is not null) return current;
            current = new AmbientTenantScope();
            _current.Value = current;
            return current;
        }
    }

    /// <summary>Replaces the current scope and tags it with a fresh generation counter.</summary>
    /// <param name="scope">The new ambient scope instance.</param>
    public void Replace(AmbientTenantScope scope)
    {
        ArgumentNullException.ThrowIfNull(scope);
        scope.Generation = Interlocked.Increment(ref _replacementCounter);
        _current.Value = scope;
    }
}
