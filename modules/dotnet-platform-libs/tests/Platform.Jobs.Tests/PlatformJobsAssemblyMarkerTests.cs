namespace Platform.Jobs.Tests;

public class PlatformJobsAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformJobsAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.Jobs", marker.Assembly.GetName().Name);
    }
}
