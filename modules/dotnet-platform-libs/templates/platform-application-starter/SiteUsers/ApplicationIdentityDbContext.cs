using Microsoft.AspNetCore.Identity.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore;

namespace StarterApp.SiteUsers;

/// <summary>
/// Application-owned EF Core context for the site-user identity schema. Each
/// generated site stores its own users, roles, and claim rows; the platform
/// never owns application identity migrations.
/// </summary>
public sealed class ApplicationIdentityDbContext(DbContextOptions<ApplicationIdentityDbContext> options) : IdentityDbContext<ApplicationUser>(options)
{
}
