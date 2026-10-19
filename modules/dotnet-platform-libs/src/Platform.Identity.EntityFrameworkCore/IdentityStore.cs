using Microsoft.EntityFrameworkCore;
using Platform.Identity.Contracts;

namespace Platform.Identity.EntityFrameworkCore;

/// <summary>Application-owned EF entity mapping seam; no platform user entity is supplied.</summary>
public interface IIdentityStore
{
    /// <summary>Looks up a current user by subject identifier.</summary>
    ValueTask<CurrentUser?> FindAsync(string subjectId, CancellationToken cancellationToken = default);
    /// <summary>Persists a normalized external identity association.</summary>
    ValueTask<IdentityProviderResult<CurrentUser>> LinkExternalIdentityAsync(ExternalIdentity identity, CancellationToken cancellationToken = default);
}

/// <summary>Optional adapter base that lets an application provide its own entity model.</summary>
public abstract class IdentityDbContextAdapter(DbContextOptions options) : DbContext(options)
{
    /// <summary>Configures only the identity adapter's application-owned mappings.</summary>
    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        base.OnModelCreating(modelBuilder);
        ConfigureIdentityModel(modelBuilder);
    }

    /// <summary>Allows the application to configure its own identity entities.</summary>
    protected abstract void ConfigureIdentityModel(ModelBuilder modelBuilder);
}

/// <summary>Registration options for an application-provided identity store.</summary>
public sealed class IdentityPersistenceOptions
{
    /// <summary>Gets or sets whether store services are enabled.</summary>
    public bool Enabled { get; set; }
}
