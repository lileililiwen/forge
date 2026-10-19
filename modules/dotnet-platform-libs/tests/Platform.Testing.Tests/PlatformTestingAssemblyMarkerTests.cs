namespace Platform.Testing.Tests;

public class PlatformTestingAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformTestingAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.Testing", marker.Assembly.GetName().Name);
    }
}
