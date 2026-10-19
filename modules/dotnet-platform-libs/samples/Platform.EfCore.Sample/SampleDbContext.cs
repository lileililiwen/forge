using Microsoft.EntityFrameworkCore;

namespace Platform.EfCore.Sample;

/// <summary>Application-owned entity. The platform never defines product tables.</summary>
public sealed class SampleItem
{
    /// <summary>Gets or sets the surrogate key.</summary>
    public int Id { get; set; }

    /// <summary>Gets or sets the display name.</summary>
    public string Name { get; set; } = string.Empty;
}

/// <summary>
/// Application-owned context. The schema, table names, and migrations below
/// belong to the application; no platform package owns them.
/// </summary>
public sealed class SampleDbContext : DbContext
{
    /// <summary>Creates the context with application-supplied options.</summary>
    /// <param name="options">The application-owned provider configuration.</param>
    public SampleDbContext(DbContextOptions<SampleDbContext> options)
        : base(options)
    {
    }

    /// <summary>Gets or sets the sample set.</summary>
    public DbSet<SampleItem> Items => Set<SampleItem>();

    /// <inheritdoc />
    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder.Entity<SampleItem>(entity =>
        {
            entity.ToTable("SampleItems");
            entity.HasKey(item => item.Id);
            entity.Property(item => item.Name).HasMaxLength(200).IsRequired();
        });
    }
}
