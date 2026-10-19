namespace Platform.ConsumerConformance;

public sealed class PlatformConsumerConformanceAssemblyMarkerTests
{
    [Fact]
    public void Assembly_marker_is_resolvable()
    {
        var marker = new PlatformConsumerConformanceAssemblyMarker();
        Assert.NotNull(marker);
        Assert.Equal(typeof(PlatformConsumerConformanceAssemblyMarker).Assembly.GetName().Name, "Platform.ConsumerConformance");
    }
}
