using Microsoft.AspNetCore.Http;

namespace Platform.Auditing.AspNetCore.Capture;

/// <summary>The resolved subject and tenant for an audit event.</summary>
/// <param name="SubjectId">The optional actor identifier.</param>
/// <param name="TenantId">The optional tenant identifier.</param>
public readonly record struct AuditSubjectResolution(string? SubjectId, string? TenantId);

/// <summary>Resolves the actor and tenant for an inbound request. Applications supply a resolver; the default reads configured headers.</summary>
public interface IAuditSubjectResolver
{
    /// <summary>Resolves the subject and tenant from the request.</summary>
    /// <param name="httpContext">The current HTTP context.</param>
    /// <returns>The resolved subject and tenant.</returns>
    AuditSubjectResolution Resolve(HttpContext httpContext);
}
