namespace Platform.Jobs.Tests;

public class RecurringJobDescriptorTests
{
    [Fact]
    public void Defaults_apply_time_zone_and_no_options()
    {
        var descriptor = new RecurringJobDescriptor(
            Name: "billing-renew",
            Cron: "0 0 * * *",
            HandlerType: typeof(PresenceEvictHandler));

        Assert.Equal("billing-renew", descriptor.Name);
        Assert.Equal("0 0 * * *", descriptor.Cron);
        Assert.Equal(typeof(PresenceEvictHandler), descriptor.HandlerType);
        Assert.Equal("UTC", descriptor.TimeZone);
        Assert.Null(descriptor.Options);
    }

    [Fact]
    public void WithName_overrides_name()
    {
        var descriptor = new RecurringJobDescriptor("a", "0 * * * *", typeof(PresenceEvictHandler));

        var renamed = descriptor.WithName("b");

        Assert.Equal("b", renamed.Name);
        Assert.Equal(descriptor.Cron, renamed.Cron);
    }

    [Fact]
    public void WithCron_overrides_cron()
    {
        var descriptor = new RecurringJobDescriptor("a", "0 * * * *", typeof(PresenceEvictHandler));

        var changed = descriptor.WithCron("*/15 * * * *");

        Assert.Equal("*/15 * * * *", changed.Cron);
        Assert.Equal(descriptor.Name, changed.Name);
    }

    [Fact]
    public void WithOptions_overrides_options()
    {
        var descriptor = new RecurringJobDescriptor("a", "0 * * * *", typeof(PresenceEvictHandler));
        var options = new Dictionary<string, object?> { ["queue"] = "primary" };

        var changed = descriptor.WithOptions(options);

        Assert.Same(options, changed.Options);
    }
}
