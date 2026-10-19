using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.Options;
using Microsoft.Extensions.DependencyInjection;
using Platform.Admin.Contracts;
using Platform.Identity.Contracts;

namespace Platform.Admin.AspNetCore;

/// <summary>Maps the opt-in administration endpoint surface.</summary>
public static class AdminEndpointRouteBuilderExtensions
{
    /// <summary>Maps bounded, permission-protected administration endpoints.</summary>
    public static IEndpointRouteBuilder MapPlatformAdminEndpoints(this IEndpointRouteBuilder endpoints)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        var options = endpoints.ServiceProvider.GetRequiredService<IOptions<AdminOptions>>().Value;
        var group = endpoints.MapGroup(options.RoutePrefix);
        group.MapGet("/users", async ([AsParameters] AdminQuery query, IAdminStore store, IOptions<AdminOptions> o, ICurrentUserAccessor current, IAdminTenantScope scope, CancellationToken ct) => await ScopedQuery(query, store.GetUsersAsync, o.Value, new HashSet<string>(["id", "email", "displayName"]), current, scope, ct)).RequireAuthorization("platform:permission:" + AdminPermissions.UsersRead);
        group.MapGet("/roles", async ([AsParameters] AdminQuery query, IAdminStore store, IOptions<AdminOptions> o, CancellationToken ct) => await Query(query, store.GetRolesAsync, o.Value, new HashSet<string>(["id", "name"]), ct)).RequireAuthorization("platform:permission:" + AdminPermissions.RolesRead);
        group.MapGet("/permissions", async (IAdminStore store, CancellationToken ct) => Results.Ok(await store.GetPermissionsAsync(ct))).RequireAuthorization("platform:permission:" + AdminPermissions.PermissionsRead);
        group.MapGet("/sessions", async ([AsParameters] AdminQuery query, IAdminStore store, IOptions<AdminOptions> o, CancellationToken ct) => await Query(query, store.GetSessionsAsync, o.Value, new HashSet<string>(["id", "createdAt", "expiresAt"]), ct)).RequireAuthorization("platform:permission:" + AdminPermissions.SessionsRead);
        group.MapGet("/audit", async ([AsParameters] AdminQuery query, IAdminStore store, IOptions<AdminOptions> o, CancellationToken ct) => await Query(query, store.GetAuditAsync, o.Value, new HashSet<string>(["id", "occurredAt"]), ct)).RequireAuthorization("platform:permission:" + AdminPermissions.AuditRead);
        group.MapGet("/providers", async (IAdminStore store, CancellationToken ct) => Results.Ok(await store.GetProviderStatusesAsync(ct))).RequireAuthorization("platform:permission:" + AdminPermissions.ProvidersRead);
        group.MapGet("/subscriptions", async ([AsParameters] AdminQuery query, IAdminStore store, IOptions<AdminOptions> o, CancellationToken ct) => await Query(query, store.GetSubscriptionSummariesAsync, o.Value, new HashSet<string>(["subjectId", "endsAt"]), ct)).RequireAuthorization("platform:permission:" + AdminPermissions.SubscriptionsRead);
        group.MapPost("/users/{id}/enabled", async (string id, SetEnabledRequest request, IAdminStore store, IAdminAuditSink audit, ICurrentUserAccessor current, IAdminTenantScope scope, HttpContext http, CancellationToken ct) =>
        {
            var user = current.GetCurrentUser();
            if (!scope.CanAccess(user.TenantId, request.TenantId)) return Results.Forbid();
            var result = await store.SetUserEnabledAsync(id, request.Enabled, user.SubjectId ?? "unknown", user.TenantId, ct);
            if (result.Succeeded)
                await audit.RecordAsync(new AdminAuditEntry(Guid.NewGuid().ToString("N"), request.Enabled ? "user.enabled" : "user.disabled", user.SubjectId ?? "unknown", id, request.TenantId ?? user.TenantId, DateTimeOffset.UtcNow, http.TraceIdentifier), ct);
            return result.Succeeded ? Results.NoContent() : Results.BadRequest(new { error = result.Code ?? "mutation_failed" });
        }).RequireAuthorization("platform:permission:" + AdminPermissions.UsersManage);
        group.MapPost("/sessions/{id}/revoke", async (string id, IAdminStore store, IAdminAuditSink audit, ICurrentUserAccessor current, HttpContext http, CancellationToken ct) =>
        {
            var user = current.GetCurrentUser();
            var result = await store.RevokeSessionAsync(id, user.SubjectId ?? "unknown", user.TenantId, ct);
            if (result.Succeeded)
                await audit.RecordAsync(new AdminAuditEntry(Guid.NewGuid().ToString("N"), "session.revoked", user.SubjectId ?? "unknown", id, user.TenantId, DateTimeOffset.UtcNow, http.TraceIdentifier), ct);
            return result.Succeeded ? Results.NoContent() : Results.BadRequest(new { error = result.Code ?? "mutation_failed" });
        }).RequireAuthorization("platform:permission:" + AdminPermissions.SessionsManage);
        if (options.EnableImpersonation)
        {
            group.MapPost("/impersonation", async (ImpersonationRequest request, IAdminImpersonationService service, IAdminAuditSink audit, ICurrentUserAccessor current, IAdminTenantScope scope, HttpContext http, IOptions<AdminOptions> o, CancellationToken ct) =>
            {
                var user = current.GetCurrentUser();
                if (string.IsNullOrWhiteSpace(request.TargetUserId) || string.IsNullOrWhiteSpace(request.Reason) || request.Reason.Length > 500 || request.Lifetime <= TimeSpan.Zero || request.Lifetime > o.Value.MaximumImpersonationLifetime || !scope.CanAccess(user.TenantId, request.TenantId))
                    return Results.BadRequest(new { error = "invalid_impersonation_request" });
                var result = await service.StartAsync(user.SubjectId ?? "unknown", request.TargetUserId, request.Reason, request.Lifetime, request.TenantId ?? user.TenantId, ct);
                if (result.Succeeded)
                    await audit.RecordAsync(new AdminAuditEntry(Guid.NewGuid().ToString("N"), "impersonation.started", user.SubjectId ?? "unknown", request.TargetUserId, request.TenantId ?? user.TenantId, DateTimeOffset.UtcNow, http.TraceIdentifier, request.Reason, result.ExpiresAt), ct);
                return result.Succeeded ? Results.Ok(result) : Results.BadRequest(new { error = result.Code ?? "impersonation_failed" });
            }).RequireAuthorization("platform:permission:" + AdminPermissions.Impersonation);
            group.MapPost("/impersonation/{id}/end", async (string id, IAdminImpersonationService service, IAdminAuditSink audit, ICurrentUserAccessor current, HttpContext http, CancellationToken ct) =>
            {
                var user = current.GetCurrentUser();
                var result = await service.EndAsync(user.SubjectId ?? "unknown", id, user.TenantId, ct);
                if (result.Succeeded)
                    await audit.RecordAsync(new AdminAuditEntry(Guid.NewGuid().ToString("N"), "impersonation.ended", user.SubjectId ?? "unknown", id, user.TenantId, DateTimeOffset.UtcNow, http.TraceIdentifier), ct);
                return result.Succeeded ? Results.NoContent() : Results.BadRequest(new { error = result.Code ?? "impersonation_end_failed" });
            }).RequireAuthorization("platform:permission:" + AdminPermissions.Impersonation);
        }
        return endpoints;
    }

    private static async Task<IResult> Query<T>(AdminQuery query, Func<AdminQuery, CancellationToken, ValueTask<AdminPage<T>>> operation, AdminOptions options, IReadOnlySet<string> sorts, CancellationToken ct)
    {
        try { return Results.Ok(await operation(query.Normalize(options.MaximumPageSize, sorts), ct)); }
        catch (ArgumentException) { return Results.BadRequest(new { error = "invalid_admin_query" }); }
    }

    private static async Task<IResult> ScopedQuery<T>(AdminQuery query, Func<AdminQuery, CancellationToken, ValueTask<AdminPage<T>>> operation, AdminOptions options, IReadOnlySet<string> sorts, ICurrentUserAccessor current, IAdminTenantScope scope, CancellationToken ct)
    {
        var user = current.GetCurrentUser();
        if (!scope.CanAccess(user.TenantId, query.TenantId)) return Results.Forbid();
        return await Query(query with { TenantId = query.TenantId ?? user.TenantId }, operation, options, sorts, ct);
    }

    private static async Task<IResult> Mutate(ValueTask<AdminMutationResult> operation, IAdminTenantScope? scope = null, string? operatorTenant = null, string? targetTenant = null)
    {
        if (scope is not null && !scope.CanAccess(operatorTenant, targetTenant)) return Results.Forbid();
        var result = await operation;
        return result.Succeeded ? Results.NoContent() : Results.BadRequest(new { error = result.Code ?? "mutation_failed" });
    }

    private sealed record SetEnabledRequest(bool Enabled, string? TenantId = null);
    private sealed record ImpersonationRequest(string TargetUserId, string Reason, TimeSpan Lifetime, string? TenantId = null);
}
