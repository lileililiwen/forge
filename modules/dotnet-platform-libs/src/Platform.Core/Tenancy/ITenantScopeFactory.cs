namespace Platform.Core.Tenancy;

/// <summary>
/// Scoped factory that installs and restores ambient tenant scopes.
/// Background handlers call <c>BeginGlobalOperation</c> or
/// <c>BeginTenant</c> with a tenant identifier or an explicit global
/// reason before resolving tenant-scoped services; the returned
/// <see cref="IDisposable"/> must be disposed in a <c>using</c> block
/// to restore the prior scope.
/// </summary>
public interface ITenantScopeFactory
{
    /// <summary>
    /// Begins an explicit global operation scope. The supplied
    /// <paramref name="reason"/> is propagated to the ambient scope for
    /// diagnostics and must not contain sensitive data.
    /// </summary>
    /// <param name="reason">Short, stable reason code.</param>
    /// <returns>A disposable that restores the prior scope on dispose.</returns>
    IDisposable BeginGlobalOperation(string reason);

    /// <summary>
    /// Begins a scope for the supplied tenant. The ambient scope
    /// immediately reflects the resolved tenant for the duration of the
    /// returned disposable.
    /// </summary>
    /// <param name="tenant">Application-supplied tenant information.</param>
    /// <returns>A disposable that restores the prior scope on dispose.</returns>
    IDisposable BeginTenant(ITenantInfo tenant);
}
