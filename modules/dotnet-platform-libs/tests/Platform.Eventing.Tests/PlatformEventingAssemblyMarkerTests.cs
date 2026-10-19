namespace Platform.Eventing.Tests;

public class PlatformEventingAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformEventingAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.Eventing", marker.Assembly.GetName().Name);
    }
}
