using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Infrastructure;
using Microsoft.EntityFrameworkCore.Migrations;
using Platform.Persistence.EfCore.Migrator;

namespace Platform.Persistence.EfCore.Migrator.Tests;

/// <summary>Application-owned sample model. Lives in the test assembly the way an application-owned model lives in the application.</summary>
public sealed class Note
{
    public int Id { get; set; }

    public string Title { get; set; } = string.Empty;
}

/// <summary>Application-owned context under migration.</summary>
public sealed class MigratorSampleDbContext(DbContextOptions<MigratorSampleDbContext> options) : DbContext(options)
{
    public DbSet<Note> Notes => Set<Note>();
}

/// <summary>Application-owned hand-written migration. No design-time tooling involved.</summary>
[DbContext(typeof(MigratorSampleDbContext))]
[Migration("20260101000000_CreateNotes")]
public sealed class CreateNotes : Migration
{
    protected override void Up(MigrationBuilder migrationBuilder)
    {
        migrationBuilder.CreateTable(
            name: "Notes",
            columns: table => new
            {
                Id = table.Column<int>(type: "INTEGER", nullable: false)
                    .Annotation("Sqlite:Autoincrement", true),
                Title = table.Column<string>(type: "TEXT", nullable: false),
            },
            constraints: table => table.PrimaryKey("PK_Notes", columns: x => x.Id));
    }

    protected override void Down(MigrationBuilder migrationBuilder)
    {
        migrationBuilder.DropTable(name: "Notes");
    }
}
