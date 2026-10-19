namespace Platform.Domain.Tests;

public class PlatformDomainAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformDomainAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.Domain", marker.Assembly.GetName().Name);
    }
}
