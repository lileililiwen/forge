using System.Security.Claims;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Platform.Identity.Contracts;

namespace Platform.Identity.AspNetCore;

/// <summary>Projects configured claims into the provider-neutral current-user contract.</summary>
public sealed class HttpCurrentUserAccessor(IHttpContextAccessor httpContextAccessor, IOptions<PlatformIdentityOptions> options) : ICurrentUserAccessor
{
    /// <inheritdoc />
    public CurrentUser GetCurrentUser()
    {
        var principal = httpContextAccessor.HttpContext?.User;
        if (principal?.Identity?.IsAuthenticated != true)
            return CurrentUser.Anonymous;

        var subjectOptions = options.Value;
        var subject = principal.FindFirstValue(subjectOptions.SubjectClaimType)
            ?? principal.FindFirstValue("sub");
        var email = principal.FindFirstValue(subjectOptions.EmailClaimType)
            ?? principal.FindFirstValue("email");
        var tenant = principal.FindFirstValue(subjectOptions.TenantClaimType);
        var roles = principal.FindAll(ClaimTypes.Role).Select(c => c.Value).Distinct(StringComparer.OrdinalIgnoreCase).ToArray();
        var permissions = principal.FindAll(subjectOptions.PermissionClaimType).Select(c => c.Value).Distinct(StringComparer.OrdinalIgnoreCase).ToArray();
        return new CurrentUser(subject, email, tenant, roles, permissions);
    }
}
