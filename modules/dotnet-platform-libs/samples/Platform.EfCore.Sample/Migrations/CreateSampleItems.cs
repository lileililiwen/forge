using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Infrastructure;
using Microsoft.EntityFrameworkCore.Migrations;

namespace Platform.EfCore.Sample;

/// <summary>
/// Application-owned hand-written migration fixture. Checked in beside the
/// context it evolves; generated migrations would live here too.
/// </summary>
[DbContext(typeof(SampleDbContext))]
[Migration("202609110001_CreateSampleItems")]
public sealed class CreateSampleItems : Migration
{
    /// <inheritdoc />
    protected override void Up(MigrationBuilder migrationBuilder)
    {
        migrationBuilder.CreateTable(
            name: "SampleItems",
            columns: table => new
            {
                Id = table.Column<int>(type: "INTEGER", nullable: false)
                    .Annotation("Sqlite:Autoincrement", true),
                Name = table.Column<string>(type: "TEXT", maxLength: 200, nullable: false),
            },
            constraints: table => table.PrimaryKey("PK_SampleItems", x => x.Id));
    }

    /// <inheritdoc />
    protected override void Down(MigrationBuilder migrationBuilder)
    {
        migrationBuilder.DropTable(name: "SampleItems");
    }
}
