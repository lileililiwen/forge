#pragma warning disable CS1591
#pragma warning disable CA1711

namespace Platform.Admin.Contracts;

/// <summary>Bounded administrative query input.</summary>
public sealed record AdminQuery(int Page = 1, int PageSize = 25, string? Search = null, string? SortBy = null, bool Descending = false, string? TenantId = null)
{
    /// <summary>Gets a normalized copy after the host-configured bound is applied.</summary>
    public AdminQuery Normalize(int maximumPageSize, IReadOnlySet<string> allowedSorts)
    {
        if (Page < 1) throw new ArgumentOutOfRangeException(nameof(maximumPageSize), "Page must be positive.");
        if (PageSize < 1 || PageSize > maximumPageSize) throw new ArgumentOutOfRangeException(nameof(maximumPageSize), "PageSize is outside the configured bound.");
        if (Search?.Length > 200) throw new ArgumentException("Search must not exceed 200 characters.", nameof(maximumPageSize));
        if (SortBy is not null && !allowedSorts.Contains(SortBy)) throw new ArgumentException("The requested sort is not supported.", nameof(allowedSorts));
        return this with { Search = string.IsNullOrWhiteSpace(Search) ? null : Search.Trim() };
    }
}

/// <summary>One page of bounded administrative data.</summary>
public sealed record AdminPage<T>(IReadOnlyList<T> Items, int Page, int PageSize, long TotalCount);

/// <summary>Safe user projection for operator views.</summary>
public sealed record AdminUser(string Id, string? Email, string? DisplayName, string? TenantId, bool Enabled, IReadOnlyCollection<string> Roles);
/// <summary>Safe role projection.</summary>
public sealed record AdminRole(string Id, string Name, string? TenantId, IReadOnlyCollection<string> Permissions);
/// <summary>Safe permission projection.</summary>
public sealed record AdminPermission(string Key, string Resource, string Action);
/// <summary>Safe session projection.</summary>
public sealed record AdminSession(string Id, string SubjectId, string? TenantId, DateTimeOffset CreatedAt, DateTimeOffset ExpiresAt, bool Revoked);
/// <summary>Safe audit projection. It contains no credential or secret fields.</summary>
public sealed record AdminAuditEntry(string Id, string Action, string ActorId, string? SubjectId, string? TenantId, DateTimeOffset OccurredAt, string? CorrelationId, string? Reason = null, DateTimeOffset? ExpiresAt = null);
/// <summary>Safe external provider status projection.</summary>
public sealed record AdminProviderStatus(string Name, bool Available, string? Detail = null);
/// <summary>Safe subscription projection owned by the consuming application.</summary>
public sealed record AdminSubscriptionSummary(string SubjectId, string? TenantId, string Status, string? Product, DateTimeOffset? EndsAt);

/// <summary>Safe result from an administrative mutation.</summary>
public sealed record AdminMutationResult(bool Succeeded, string? Code = null)
{
    /// <summary>Creates a successful mutation result.</summary>
    public static AdminMutationResult Success() => new(true);
    /// <summary>Creates a safe failure result.</summary>
    public static AdminMutationResult Failure(string code) => new(false, code);
}

/// <summary>Names of the permissions required by the standard administrative endpoints.</summary>
public static class AdminPermissions
{
    public const string UsersRead = "admin.users.read";
    public const string UsersManage = "admin.users.manage";
    public const string RolesRead = "admin.roles.read";
    public const string RolesManage = "admin.roles.manage";
    public const string PermissionsRead = "admin.permissions.read";
    public const string SessionsRead = "admin.sessions.read";
    public const string SessionsManage = "admin.sessions.manage";
    public const string AuditRead = "admin.audit.read";
    public const string ProvidersRead = "admin.providers.read";
    public const string SubscriptionsRead = "admin.subscriptions.read";
    public const string Impersonation = "admin.impersonation";
}

/// <summary>Describes one endpoint and the permission required to use it.</summary>
public sealed record AdminEndpointDescriptor(string Route, string Method, string Permission);

/// <summary>Stable metadata for clients that build an administration navigation surface.</summary>
public static class AdminEndpointCatalog
{
    /// <summary>Gets the standard administration endpoint metadata.</summary>
    public static IReadOnlyList<AdminEndpointDescriptor> All { get; } =
    [
        new("/users", "GET", AdminPermissions.UsersRead),
        new("/roles", "GET", AdminPermissions.RolesRead),
        new("/permissions", "GET", AdminPermissions.PermissionsRead),
        new("/sessions", "GET", AdminPermissions.SessionsRead),
        new("/audit", "GET", AdminPermissions.AuditRead),
        new("/providers", "GET", AdminPermissions.ProvidersRead),
        new("/subscriptions", "GET", AdminPermissions.SubscriptionsRead),
        new("/users/{id}/enabled", "POST", AdminPermissions.UsersManage),
        new("/sessions/{id}/revoke", "POST", AdminPermissions.SessionsManage),
        new("/impersonation", "POST", AdminPermissions.Impersonation),
        new("/impersonation/{id}/end", "POST", AdminPermissions.Impersonation),
    ];
}

/// <summary>Application-owned administration persistence and projection boundary.</summary>
public interface IAdminStore
{
    ValueTask<AdminPage<AdminUser>> GetUsersAsync(AdminQuery query, CancellationToken cancellationToken = default);
    ValueTask<AdminPage<AdminRole>> GetRolesAsync(AdminQuery query, CancellationToken cancellationToken = default);
    ValueTask<IReadOnlyList<AdminPermission>> GetPermissionsAsync(CancellationToken cancellationToken = default);
    ValueTask<AdminPage<AdminSession>> GetSessionsAsync(AdminQuery query, CancellationToken cancellationToken = default);
    ValueTask<AdminPage<AdminAuditEntry>> GetAuditAsync(AdminQuery query, CancellationToken cancellationToken = default);
    ValueTask<IReadOnlyList<AdminProviderStatus>> GetProviderStatusesAsync(CancellationToken cancellationToken = default);
    ValueTask<AdminPage<AdminSubscriptionSummary>> GetSubscriptionSummariesAsync(AdminQuery query, CancellationToken cancellationToken = default);
    ValueTask<AdminMutationResult> SetUserEnabledAsync(string userId, bool enabled, string actorId, string? tenantId, CancellationToken cancellationToken = default);
    ValueTask<AdminMutationResult> RevokeSessionAsync(string sessionId, string actorId, string? tenantId, CancellationToken cancellationToken = default);
}

/// <summary>Receives structured administration audit events.</summary>
public interface IAdminAuditSink
{
    ValueTask RecordAsync(AdminAuditEntry auditEntry, CancellationToken cancellationToken = default);
}

/// <summary>Resolves whether an operator may act within a target tenant.</summary>
public interface IAdminTenantScope
{
    bool CanAccess(string? operatorTenantId, string? targetTenantId);
}

/// <summary>Optional application-owned impersonation boundary.</summary>
public interface IAdminImpersonationService
{
    ValueTask<AdminImpersonationResult> StartAsync(string actorId, string targetUserId, string reason, TimeSpan lifetime, string? tenantId, CancellationToken cancellationToken = default);
    ValueTask<AdminMutationResult> EndAsync(string actorId, string impersonationId, string? tenantId, CancellationToken cancellationToken = default);
}

/// <summary>Safe impersonation result; tokens and credentials are never returned by this contract.</summary>
public sealed record AdminImpersonationResult(bool Succeeded, string? ImpersonationId = null, DateTimeOffset? ExpiresAt = null, string? Code = null);
