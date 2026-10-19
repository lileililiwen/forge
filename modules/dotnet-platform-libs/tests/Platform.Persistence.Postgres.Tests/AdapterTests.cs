using Microsoft.EntityFrameworkCore;
using Platform.Persistence.Postgres;

namespace Platform.Persistence.Postgres.Tests;

public sealed class AdapterTests
{
    [Fact]
    public void PostgreSql_options_use_the_postgres_provider()
    {
        var options = new DbContextOptionsBuilder()
            .UsePlatformPostgres("Host=localhost;Database=test;Username=test;Password=test")
            .Options;

        Assert.Contains(options.Extensions, extension => extension.GetType().Assembly.GetName().Name?.Contains("Npgsql") == true);
    }
}
