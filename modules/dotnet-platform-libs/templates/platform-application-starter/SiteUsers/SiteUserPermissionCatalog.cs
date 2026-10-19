using Platform.Authorization;

namespace StarterApp.SiteUsers;

/// <summary>
/// Application-owned permission catalog. The platform never owns an
/// application's permission keys; the generated site registers its
/// permissions here and through <see cref="Microsoft.Extensions.DependencyInjection.AuthorizationBuilderExtensions.AddPolicy"/>
/// with names produced by <see cref="PlatformPolicyNames.ForPermission"/>.
/// Unknown permission keys MUST resolve to <see cref="AuthorizationDecision.Denied"/>.
/// </summary>
public static class SiteUserPermissionCatalog
{
    /// <summary>Permission key: read the signed-in user's own profile.</summary>
    public const string ProfileRead = "site.profile.read";

    /// <summary>Permission key: update the signed-in user's own profile.</summary>
    public const string ProfileUpdate = "site.profile.update";

    /// <summary>Registers the documented sample permissions into a catalog.</summary>
    public static PermissionCatalog Register(PermissionCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(catalog);
        catalog.Register(new PermissionDefinition("site", "profile.read"));
        catalog.Register(new PermissionDefinition("site", "profile.update"));
        return catalog;
    }
}
