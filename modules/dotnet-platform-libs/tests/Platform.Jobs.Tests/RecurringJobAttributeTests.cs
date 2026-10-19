namespace Platform.Jobs.Tests;

public class RecurringJobAttributeTests
{
    [Fact]
    public void Ctor_rejects_empty_cron()
    {
        Assert.Throws<ArgumentException>(() => new RecurringJobAttribute(""));
        Assert.Throws<ArgumentException>(() => new RecurringJobAttribute("   "));
    }

    [Fact]
    public void GetDescriptor_returns_cron_and_default_name_when_name_not_set()
    {
        var descriptor = RecurringJobAttribute.GetDescriptor(typeof(PresenceEvictHandler));

        Assert.Equal(typeof(PresenceEvictHandler).FullName, descriptor.Name);
        Assert.Equal("0 * * * *", descriptor.Cron);
        Assert.Equal(typeof(PresenceEvictHandler), descriptor.HandlerType);
        Assert.Equal("UTC", descriptor.TimeZone);
        Assert.Null(descriptor.Options);
    }

    [Fact]
    public void GetDescriptor_returns_explicit_name_and_time_zone()
    {
        var descriptor = RecurringJobAttribute.GetDescriptor(typeof(QueueDrainHandler));

        Assert.Equal("queue-drain", descriptor.Name);
        Assert.Equal("*/5 * * * *", descriptor.Cron);
        Assert.Equal("Europe/Berlin", descriptor.TimeZone);
    }

    [Fact]
    public void GetDescriptor_throws_when_handler_is_undecorated()
    {
        var exception = Assert.Throws<InvalidOperationException>(
            () => RecurringJobAttribute.GetDescriptor(typeof(UndecoratedHandler)));

        Assert.Contains(typeof(UndecoratedHandler).FullName!, exception.Message);
    }

    [Fact]
    public void GetDescriptor_throws_when_handler_type_is_null()
    {
        Assert.Throws<ArgumentNullException>(
            () => RecurringJobAttribute.GetDescriptor(null!));
    }
}
