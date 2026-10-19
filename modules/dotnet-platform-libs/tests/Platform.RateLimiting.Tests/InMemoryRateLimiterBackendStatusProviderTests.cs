namespace Platform.RateLimiting.Tests;

public class InMemoryRateLimiterBackendStatusProviderTests
{
    [Fact]
    public void GetStatus_reports_memory_provider_as_available()
    {
        var provider = new InMemoryRateLimiterBackendStatusProvider();

        var status = provider.GetStatus();

        Assert.Equal("memory", status.Provider);
        Assert.True(status.Available);
        Assert.NotNull(status.Detail);
    }
}
