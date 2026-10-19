using Microsoft.EntityFrameworkCore;

namespace StarterApp.Data;

/// <summary>
/// Application-owned EF Core context. Add entity sets, model configuration,
/// migrations, and seed data here; the platform never owns application state.
/// </summary>
public sealed class AppDbContext(DbContextOptions<AppDbContext> options) : DbContext(options)
{
}
