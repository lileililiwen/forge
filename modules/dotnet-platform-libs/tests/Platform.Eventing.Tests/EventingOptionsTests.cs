namespace Platform.Eventing.Tests;

public class EventingOptionsTests
{
    [Fact]
    public void Default_bounded_capacity_is_1024()
    {
        var options = new EventingOptions();

        Assert.Equal(1024, options.InProcessBoundedCapacity);
    }

    [Fact]
    public void Section_name_constant_is_stable()
    {
        Assert.Equal("Eventing", EventingOptions.SectionName);
    }
}
