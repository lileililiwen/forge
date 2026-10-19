using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Authentication;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Authorization;
using Platform.Identity.Contracts;

namespace Platform.Identity.AspNetCore;

internal sealed class PlatformPermissionRequirement(string permission) : IAuthorizationRequirement
{
    public string Permission { get; } = permission;
}

internal sealed class PlatformPermissionHandler(
    ICurrentUserAccessor currentUser,
    IAuthorizationDecisionAuditor? auditor = null,
    IIdentityAuditHook? auditHook = null)
    : AuthorizationHandler<PlatformPermissionRequirement>
{
    protected override async Task HandleRequirementAsync(AuthorizationHandlerContext context, PlatformPermissionRequirement requirement)
    {
        var user = currentUser.GetCurrentUser();
        var granted = user.IsAuthenticated && user.PermissionSet.Contains(requirement.Permission, StringComparer.OrdinalIgnoreCase);
        if (granted)
            context.Succeed(requirement);

        if (auditor is not null)
            await auditor.RecordAsync(granted
                ? AuthorizationDecision.Granted(requirement.Permission)
                : AuthorizationDecision.Denied(requirement.Permission), user.SubjectId).ConfigureAwait(false);

        if (!granted && auditHook is not null)
        {
            // Identity audit records a denied decision without any token contents.
            await auditHook.RecordAsync(
                new IdentityAuditEvent("authorization.denied", user.SubjectId, false, DateTimeOffset.UtcNow)).ConfigureAwait(false);
        }
    }
}

/// <summary>Registration and policy helpers for platform identity.</summary>
public static class IdentityServiceCollectionExtensions
{
    /// <summary>Registers current-user projection and ASP.NET authorization helpers.</summary>
    public static IServiceCollection AddPlatformIdentity(this IServiceCollection services) => services.AddPlatformIdentity(_ => { });

    /// <summary>Registers platform identity services with explicit options.</summary>
    public static IServiceCollection AddPlatformIdentity(this IServiceCollection services, Action<PlatformIdentityOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddOptions<PlatformIdentityOptions>().Configure(configure).Validate(o => o.IsValid(), "Platform identity options are invalid.");
        services.AddHttpContextAccessor();
        services.TryAddScoped<ICurrentUserAccessor, HttpCurrentUserAccessor>();
        services.TryAddScoped<IIdentitySessionService, IdentitySessionService>();
        services.AddAuthorization();
        services.TryAddEnumerable(ServiceDescriptor.Transient<IAuthorizationHandler, PlatformPermissionHandler>());
        return services;
    }

    /// <summary>Registers the platform-owned authentication scheme without replacing consumer schemes.</summary>
    public static AuthenticationBuilder AddPlatformIdentityAuthentication(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);
        var options = new PlatformIdentityOptions();
        return services.AddAuthentication(authentication =>
        {
            authentication.DefaultAuthenticateScheme = options.AuthenticationScheme;
            authentication.DefaultChallengeScheme = options.AuthenticationScheme;
        });
    }

    /// <summary>Registers an application-owned credential verifier without a platform user entity.</summary>
    public static IServiceCollection AddPlatformIdentityCredentialVerifier<TVerifier>(this IServiceCollection services)
        where TVerifier : class, ICredentialVerifier
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddScoped<ICredentialVerifier, TVerifier>();
        return services;
    }

    /// <summary>Registers an application-owned external identity provider without a platform user entity.</summary>
    public static IServiceCollection AddPlatformIdentityExternalProvider<TProvider>(this IServiceCollection services)
        where TProvider : class, IExternalIdentityProvider
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddScoped<IExternalIdentityProvider, TProvider>();
        return services;
    }

    /// <summary>Registers an application-owned verification provider without a platform channel.</summary>
    public static IServiceCollection AddPlatformIdentityVerificationProvider<TProvider>(this IServiceCollection services)
        where TProvider : class, IVerificationProvider
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddScoped<IVerificationProvider, TProvider>();
        return services;
    }

    /// <summary>Registers an application-owned session store used by the identity session service.</summary>
    public static IServiceCollection AddPlatformIdentitySessionStore<TStore>(this IServiceCollection services)
        where TStore : class, ISessionStore
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddScoped<ISessionStore, TStore>();
        return services;
    }

    /// <summary>Registers an application-owned identity audit hook.</summary>
    public static IServiceCollection AddPlatformIdentityAuditHook<THook>(this IServiceCollection services)
        where THook : class, IIdentityAuditHook
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddScoped<IIdentityAuditHook, THook>();
        return services;
    }

    /// <summary>Adds a named policy requiring the given module-owned permission.</summary>
    public static IServiceCollection RequirePlatformPermission(this IServiceCollection services, string permissionKey)
    {
        ArgumentNullException.ThrowIfNull(services);
        if (string.IsNullOrWhiteSpace(permissionKey)) throw new ArgumentException("Permission is required.", nameof(permissionKey));
        services.AddAuthorizationBuilder().AddPolicy(PlatformPolicyNames.ForPermission(permissionKey), policy =>
            policy.RequireAuthenticatedUser().AddRequirements(new PlatformPermissionRequirement(permissionKey)));
        return services;
    }

    /// <summary>Adds a named policy requiring an application-owned role.</summary>
    public static IServiceCollection RequirePlatformRole(this IServiceCollection services, string role)
    {
        ArgumentNullException.ThrowIfNull(services);
        if (string.IsNullOrWhiteSpace(role)) throw new ArgumentException("Role is required.", nameof(role));
        services.AddAuthorizationBuilder().AddPolicy(PlatformPolicyNames.ForRole(role), policy =>
            policy.RequireAuthenticatedUser().RequireRole(role));
        return services;
    }

    /// <summary>Returns a policy name for use with endpoint or controller authorization.</summary>
    public static string PlatformPermissionPolicy(string permissionKey) => PlatformPolicyNames.ForPermission(permissionKey);

    /// <summary>Returns a policy name for use with endpoint or controller authorization.</summary>
    public static string PlatformRolePolicy(string role) => PlatformPolicyNames.ForRole(role);
}
