using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Platform.EfCore.Sample;
using Platform.Persistence.EfCore.DependencyInjection;

namespace Platform.SampleMatrix.Tests;

/// <summary>Stage 2: the application-owned context migrates and round-trips through platform conventions.</summary>
public sealed class EfCoreTests : IDisposable
{
    private readonly string _database;

    /// <summary>Creates an isolated database path per test run.</summary>
    public EfCoreTests()
    {
        _database = Path.Combine(Path.GetTempPath(), $"matrix-efcore-{Guid.NewGuid():N}.db");
    }

    /// <inheritdoc />
    public void Dispose()
    {
        if (File.Exists(_database))
        {
            File.Delete(_database);
        }

        GC.SuppressFinalize(this);
    }

    /// <summary>Verifies the sample migration applies and data round-trips.</summary>
    [Fact]
    public async Task Migration_applies_and_items_round_trip()
    {
        var services = new ServiceCollection();
        services.AddPlatformPersistenceEfCore();
        services.AddDbContext<SampleDbContext>(options => options.UseSqlite($"Data Source={_database}"));
        await using var provider = services.BuildServiceProvider();
        await using var scope = provider.CreateAsyncScope();
        var context = scope.ServiceProvider.GetRequiredService<SampleDbContext>();

        var pending = await context.Database.GetPendingMigrationsAsync();
        Assert.Contains("202609110001_CreateSampleItems", pending);
        await context.Database.MigrateAsync();
        Assert.Empty(await context.Database.GetPendingMigrationsAsync());

        context.Items.Add(new SampleItem { Name = "matrix-item" });
        await context.SaveChangesAsync();

        var stored = await context.Items.SingleAsync(item => item.Name == "matrix-item");
        Assert.True(stored.Id > 0);
    }
}
