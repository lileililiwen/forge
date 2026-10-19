using Hangfire;
using Platform.Core.Time;
using Platform.Jobs.Hangfire;

namespace Platform.Jobs.Hangfire.Tests;

public class HangfireRecurringJobRegistryTests : HangfireTest
{
    private static readonly DateTimeOffset Now = new(2026, 9, 9, 12, 0, 0, TimeSpan.Zero);

    [Fact]
    public void Register_attaches_the_platform_recurring_executor_to_hangfire()
    {
        var manager = new RecordingRecurringJobManager();
        var telemetry = new RecordingJobTelemetry();
        var registry = new HangfireRecurringJobRegistry(manager, new FixedClock(Now), telemetry);
        var descriptor = new RecurringJobDescriptor(
            "nightly-cleanup",
            "0 2 * * *",
            typeof(RecordingRecurringHandler),
            TimeZone: "UTC");

        registry.Register(descriptor);

        var registration = Assert.Single(manager.Added);
        Assert.Equal("nightly-cleanup", registration.Id);
        Assert.Equal("0 2 * * *", registration.Cron);
        Assert.Equal(typeof(HangfireJobExecutor), registration.Job.Type);
        Assert.Equal(nameof(HangfireJobExecutor.ExecuteRecurring), registration.Job.Method.Name);
        Assert.Equal("nightly-cleanup", registration.Job.Args[0]);
        Assert.Equal(TimeZoneInfo.Utc, registration.Options.TimeZone);

        var registeredTelemetry = Assert.Single(telemetry.Registered);
        Assert.Equal("nightly-cleanup", registeredTelemetry.Descriptor.Name);
        Assert.Equal(Now, registeredTelemetry.At);
    }

    [Fact]
    public void Registering_the_same_name_again_is_a_no_op()
    {
        var manager = new RecordingRecurringJobManager();
        var telemetry = new RecordingJobTelemetry();
        var registry = new HangfireRecurringJobRegistry(manager, new FixedClock(Now), telemetry);
        var first = new RecurringJobDescriptor("job", "* * * * *", typeof(RecordingRecurringHandler));
        var second = new RecurringJobDescriptor("job", "0 0 * * *", typeof(RecordingRecurringHandler));

        registry.Register(first);
        registry.Register(second);

        Assert.Single(manager.Added);
        Assert.Single(telemetry.Registered);
        var registered = Assert.Single(registry.Registered);
        Assert.Equal("* * * * *", registered.Cron);
    }

    [Fact]
    public void Register_rejects_null_and_malformed_descriptors()
    {
        var registry = new HangfireRecurringJobRegistry(new RecordingRecurringJobManager(), new FixedClock(Now));

        Assert.Throws<ArgumentNullException>(() => registry.Register(null!));
        Assert.Throws<ArgumentException>(() => registry.Register(new RecurringJobDescriptor(" ", "* * * * *", typeof(RecordingRecurringHandler))));
        Assert.Throws<ArgumentException>(() => registry.Register(new RecurringJobDescriptor("job", " ", typeof(RecordingRecurringHandler))));
    }

    [Fact]
    public void Register_with_unknown_time_zone_fails_and_does_not_keep_the_descriptor()
    {
        var registry = new HangfireRecurringJobRegistry(new RecordingRecurringJobManager(), new FixedClock(Now));

        Assert.ThrowsAny<TimeZoneNotFoundException>(
            () => registry.Register(new RecurringJobDescriptor("job", "* * * * *", typeof(RecordingRecurringHandler), TimeZone: "Not/AZone")));

        Assert.Empty(registry.Registered);
    }

    [Fact]
    public void Registered_reflects_registration_order()
    {
        var registry = new HangfireRecurringJobRegistry(new RecordingRecurringJobManager(), new FixedClock(Now));

        registry.Register(new RecurringJobDescriptor("a", "* * * * *", typeof(RecordingRecurringHandler)));
        registry.Register(new RecurringJobDescriptor("b", "* * * * *", typeof(RecordingRecurringHandler)));

        Assert.Equal(new[] { "a", "b" }, registry.Registered.Select(d => d.Name).ToArray());
    }
}
