using Platform.Core.Results;
using Platform.Core.Time;

namespace Platform.Jobs.Tests;

public class IJobTelemetrySurfaceTests
{
    [Fact]
    public void Recording_telemetry_captures_all_four_calls_with_clock_time()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var telemetry = new RecordingJobTelemetry();
        var descriptor = new RecurringJobDescriptor("a", "0 * * * *", typeof(PresenceEvictHandler));
        var payload = JobPayload.Create("billing-renew");
        var error = Error.Validation("name is required");

        telemetry.JobRegistered(descriptor, clock.UtcNow);
        telemetry.JobEnqueued(payload, clock.UtcNow);
        telemetry.JobExecuted("billing-renew", clock.UtcNow);
        telemetry.JobFailed("billing-renew", error, clock.UtcNow);

        var registered = Assert.Single(telemetry.Registered);
        Assert.Same(descriptor, registered.Descriptor);
        Assert.Equal(clock.UtcNow, registered.At);
        var enqueued = Assert.Single(telemetry.Enqueued);
        Assert.Same(payload, enqueued.Payload);
        var executed = Assert.Single(telemetry.Executed);
        Assert.Equal("billing-renew", executed.Name);
        var failed = Assert.Single(telemetry.Failed);
        Assert.Equal("billing-renew", failed.Name);
        Assert.Same(error, failed.Error);
    }

    [Fact]
    public void Registry_invokes_telemetry_with_clock_time()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var telemetry = new RecordingJobTelemetry();
        var registry = new InMemoryRecurringJobRegistry(clock, telemetry);

        registry.Register(RecurringJobAttribute.GetDescriptor(typeof(PresenceEvictHandler)));

        var recorded = Assert.Single(telemetry.Registered);
        Assert.Equal("0 * * * *", recorded.Descriptor.Cron);
        Assert.Equal(typeof(PresenceEvictHandler), recorded.Descriptor.HandlerType);
        Assert.Equal(clock.UtcNow, recorded.At);
    }

    [Fact]
    public void Registry_does_not_invoke_telemetry_when_none_registered()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var registry = new InMemoryRecurringJobRegistry(clock, telemetry: null);

        registry.Register(RecurringJobAttribute.GetDescriptor(typeof(PresenceEvictHandler)));

        Assert.Single(registry.Registered);
    }

    [Fact]
    public void Registry_keeps_first_descriptor_when_name_re_registered()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var registry = new InMemoryRecurringJobRegistry(clock);
        var first = new RecurringJobDescriptor("a", "0 * * * *", typeof(PresenceEvictHandler));
        var duplicate = new RecurringJobDescriptor("a", "*/5 * * * *", typeof(QueueDrainHandler));

        registry.Register(first);
        registry.Register(duplicate);

        var stored = Assert.Single(registry.Registered);
        Assert.Same(first, stored);
    }

    [Fact]
    public async Task Dispatcher_invokes_telemetry_with_clock_time()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var telemetry = new RecordingJobTelemetry();
        var dispatcher = new RecordingDispatcher(clock, telemetry);
        var payload = JobPayload.Create("billing-renew");

        await dispatcher.EnqueueAsync(payload);

        Assert.Same(payload, Assert.Single(dispatcher.Enqueued));
        var enqueued = Assert.Single(telemetry.Enqueued);
        Assert.Same(payload, enqueued.Payload);
        Assert.Equal(clock.UtcNow, enqueued.At);
    }

    [Fact]
    public async Task Dispatcher_rejects_null_payload()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var dispatcher = new RecordingDispatcher(clock);

        await Assert.ThrowsAsync<ArgumentNullException>(
            async () => await dispatcher.EnqueueAsync(null!));
    }
}
