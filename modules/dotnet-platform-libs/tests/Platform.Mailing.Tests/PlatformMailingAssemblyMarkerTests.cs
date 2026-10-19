namespace Platform.Mailing.Tests;

public class PlatformMailingAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformMailingAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.Mailing", marker.Assembly.GetName().Name);
    }
}
