namespace Platform.RateLimiting.Tests;

public class PlatformRateLimitingAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformRateLimitingAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.RateLimiting", marker.Assembly.GetName().Name);
    }
}
