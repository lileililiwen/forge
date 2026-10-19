#pragma warning disable CS1591

using Platform.Admin.Contracts;

namespace Platform.Admin.Testing;

/// <summary>In-memory administrative store for deterministic host and integration tests.</summary>
public sealed class InMemoryAdminStore : IAdminStore
{
    /// <summary>Users exposed by the store.</summary>
    public List<AdminUser> Users { get; } = [];
    /// <summary>Roles exposed by the store.</summary>
    public List<AdminRole> Roles { get; } = [];
    /// <summary>Sessions exposed by the store.</summary>
    public List<AdminSession> Sessions { get; } = [];
    /// <summary>Audit entries exposed by the store.</summary>
    public List<AdminAuditEntry> AuditEntries { get; } = [];
    /// <summary>Providers exposed by the store.</summary>
    public List<AdminProviderStatus> Providers { get; } = [];
    /// <summary>Subscription summaries exposed by the store.</summary>
    public List<AdminSubscriptionSummary> Subscriptions { get; } = [];
    /// <summary>Permissions exposed by the store.</summary>
    public List<AdminPermission> Permissions { get; } = [];

    public ValueTask<AdminPage<AdminUser>> GetUsersAsync(AdminQuery query, CancellationToken cancellationToken = default) => Page(Users, query, u => u.Id + " " + u.Email + " " + u.DisplayName, u => u.Id);
    public ValueTask<AdminPage<AdminRole>> GetRolesAsync(AdminQuery query, CancellationToken cancellationToken = default) => Page(Roles, query, r => r.Id + " " + r.Name, r => r.Id);
    public ValueTask<IReadOnlyList<AdminPermission>> GetPermissionsAsync(CancellationToken cancellationToken = default) => ValueTask.FromResult<IReadOnlyList<AdminPermission>>(Permissions.ToArray());
    public ValueTask<AdminPage<AdminSession>> GetSessionsAsync(AdminQuery query, CancellationToken cancellationToken = default) => Page(Sessions, query, s => s.Id + " " + s.SubjectId, s => s.Id);
    public ValueTask<AdminPage<AdminAuditEntry>> GetAuditAsync(AdminQuery query, CancellationToken cancellationToken = default) => Page(AuditEntries, query, a => a.Id + " " + a.Action + " " + a.ActorId + " " + a.SubjectId, a => a.OccurredAt.ToString("O"));
    public ValueTask<IReadOnlyList<AdminProviderStatus>> GetProviderStatusesAsync(CancellationToken cancellationToken = default) => ValueTask.FromResult<IReadOnlyList<AdminProviderStatus>>(Providers.ToArray());
    public ValueTask<AdminPage<AdminSubscriptionSummary>> GetSubscriptionSummariesAsync(AdminQuery query, CancellationToken cancellationToken = default) => Page(Subscriptions, query, s => s.SubjectId + " " + s.Status + " " + s.Product, s => s.SubjectId);
    public ValueTask<AdminMutationResult> SetUserEnabledAsync(string userId, bool enabled, string actorId, string? tenantId, CancellationToken cancellationToken = default)
    {
        var index = Users.FindIndex(u => u.Id == userId && u.TenantId == tenantId);
        if (index < 0) return ValueTask.FromResult(AdminMutationResult.Failure("not_found"));
        Users[index] = Users[index] with { Enabled = enabled };
        return ValueTask.FromResult(AdminMutationResult.Success());
    }
    public ValueTask<AdminMutationResult> RevokeSessionAsync(string sessionId, string actorId, string? tenantId, CancellationToken cancellationToken = default)
    {
        var index = Sessions.FindIndex(s => s.Id == sessionId && s.TenantId == tenantId);
        if (index < 0) return ValueTask.FromResult(AdminMutationResult.Failure("not_found"));
        Sessions[index] = Sessions[index] with { Revoked = true };
        return ValueTask.FromResult(AdminMutationResult.Success());
    }

    private static ValueTask<AdminPage<T>> Page<T>(IEnumerable<T> values, AdminQuery query, Func<T, string> searchable, Func<T, string> ordering)
    {
        var filtered = values.Where(value => query.TenantId is null || value switch
        {
            AdminUser user => user.TenantId == query.TenantId,
            AdminRole role => role.TenantId == query.TenantId,
            AdminSession session => session.TenantId == query.TenantId,
            AdminAuditEntry audit => audit.TenantId == query.TenantId,
            AdminSubscriptionSummary subscription => subscription.TenantId == query.TenantId,
            _ => true,
        });
        if (!string.IsNullOrWhiteSpace(query.Search)) filtered = filtered.Where(value => searchable(value).Contains(query.Search, StringComparison.OrdinalIgnoreCase));
        var ordered = query.Descending ? filtered.OrderByDescending(ordering) : filtered.OrderBy(ordering);
        var items = ordered.Skip((query.Page - 1) * query.PageSize).Take(query.PageSize).ToArray();
        return ValueTask.FromResult(new AdminPage<T>(items, query.Page, query.PageSize, filtered.LongCount()));
    }
}

/// <summary>Collects structured administration audit events.</summary>
public sealed class RecordingAdminAuditSink : IAdminAuditSink
{
    /// <summary>Recorded entries.</summary>
    public List<AdminAuditEntry> Entries { get; } = [];
    public ValueTask RecordAsync(AdminAuditEntry auditEntry, CancellationToken cancellationToken = default) { Entries.Add(auditEntry); return ValueTask.CompletedTask; }
}
