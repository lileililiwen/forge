using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Platform.EfCore.Sample;
using Platform.Persistence.EfCore.DependencyInjection;

// Stage 2: EF Core persistence. The application owns the context, the
// provider selection (SQLite here), the connection string, and the
// migrations. The platform contributes only opt-in conventions.
var database = args.FirstOrDefault() ?? Path.Combine(Path.GetTempPath(), "platform-efcore-sample.db");
var services = new ServiceCollection();
services.AddPlatformPersistenceEfCore();
services.AddDbContext<SampleDbContext>(options => options.UseSqlite($"Data Source={database}"));
await using var provider = services.BuildServiceProvider();
await using var scope = provider.CreateAsyncScope();
var context = scope.ServiceProvider.GetRequiredService<SampleDbContext>();
await context.Database.MigrateAsync();
if (!await context.Items.AnyAsync())
{
    context.Items.Add(new SampleItem { Name = "matrix-first" });
    await context.SaveChangesAsync();
}

var count = await context.Items.CountAsync();
Console.WriteLine($"database={database} items={count}");
