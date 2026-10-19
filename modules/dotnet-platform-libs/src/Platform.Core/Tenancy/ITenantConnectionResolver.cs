namespace Platform.Core.Tenancy;

/// <summary>
/// Selects the connection descriptor for the current tenant. The
/// implementation is application-owned; it may return a shared
/// connection for tenants that share a database or a dedicated
/// connection for tenants that need their own.
/// </summary>
public interface ITenantConnectionResolver
{
    /// <summary>
    /// Resolves the connection descriptor for the supplied
    /// <paramref name="tenant"/>. Must never return <c>null</c>; return
    /// a shared descriptor when no dedicated connection is available.
    /// </summary>
    /// <param name="tenant">The resolved tenant, or <c>null</c> when a global operation is in effect.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    ValueTask<TenantConnectionDescriptor> ResolveConnectionAsync(ITenantInfo? tenant, CancellationToken cancellationToken = default);
}
