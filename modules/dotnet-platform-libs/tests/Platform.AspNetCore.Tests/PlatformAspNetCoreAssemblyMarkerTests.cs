namespace Platform.AspNetCore.Tests;

public class PlatformAspNetCoreAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformAspNetCoreAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.AspNetCore", marker.Assembly.GetName().Name);
    }
}
