namespace Platform.Core.Tests;

public class PlatformCoreAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformCoreAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.Core", marker.Assembly.GetName().Name);
    }
}
