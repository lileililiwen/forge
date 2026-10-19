namespace Platform.Core.Tenancy;

/// <summary>
/// Thrown by the multitenancy adapter when tenant-scoped access is
/// attempted without a resolved tenant or an explicit global
/// operation. The exception message is safe to surface to logs and
/// never carries tenant values or connection strings.
/// </summary>
public sealed class TenantScopeNotResolvedException : InvalidOperationException
{
    /// <summary>Initializes a new instance of the <see cref="TenantScopeNotResolvedException"/> class.</summary>
    public TenantScopeNotResolvedException()
        : base("A tenant scope must be resolved or an explicit global operation must be opened before tenant-scoped access is allowed.")
    {
    }

    /// <summary>Initializes a new instance with a safe reason code.</summary>
    /// <param name="reason">Short, stable reason code describing the cause.</param>
    public TenantScopeNotResolvedException(string reason)
        : base($"A tenant scope must be resolved or an explicit global operation must be opened before tenant-scoped access is allowed. Reason: {reason}")
    {
    }
}
