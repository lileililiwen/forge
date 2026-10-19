namespace Platform.Idempotency.Tests;

public class PlatformIdempotencyAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformIdempotencyAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.Idempotency", marker.Assembly.GetName().Name);
    }
}
