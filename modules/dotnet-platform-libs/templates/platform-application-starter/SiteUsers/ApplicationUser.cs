using Microsoft.AspNetCore.Identity;

namespace StarterApp.SiteUsers;

/// <summary>
/// Application-owned end-user record. Each generated site owns its own user table;
/// the platform never owns a user entity or migration.
/// </summary>
public sealed class ApplicationUser : IdentityUser
{
}
