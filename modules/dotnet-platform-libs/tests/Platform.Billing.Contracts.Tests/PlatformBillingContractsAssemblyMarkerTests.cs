namespace Platform.Billing.Contracts.Tests;

public class PlatformBillingContractsAssemblyMarkerTests
{
    [Fact]
    public void MarkerType_IsAvailableInAssembly()
    {
        var marker = typeof(PlatformBillingContractsAssemblyMarker);
        Assert.NotNull(marker);
        Assert.Equal("Platform.Billing.Contracts", marker.Assembly.GetName().Name);
    }
}
