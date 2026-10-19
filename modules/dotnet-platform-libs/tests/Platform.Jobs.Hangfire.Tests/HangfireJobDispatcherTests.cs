using System.Text.Json;
using Hangfire;
using Hangfire.States;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Jobs.Hangfire;

namespace Platform.Jobs.Hangfire.Tests;

public class HangfireJobDispatcherTests : HangfireTest
{
    private static readonly DateTimeOffset Now = new(2026, 9, 9, 12, 0, 0, TimeSpan.Zero);

    [Fact]
    public void Enqueue_creates_a_hangfire_job_with_the_serialized_payload()
    {
        var client = new RecordingBackgroundJobClient();
        var telemetry = new RecordingJobTelemetry();
        var dispatcher = new HangfireJobDispatcher(
            client,
            Options.Create(new HangfireJobsOptions { Queue = "email" }),
            new FixedClock(Now),
            telemetry);

        dispatcher.EnqueueAsync(JobPayload.Create("cleanup", new Dictionary<string, object?> { ["count"] = 3 }));

        var (job, state) = Assert.Single(client.Created);
        Assert.Equal(typeof(HangfireJobExecutor), job.Type);
        Assert.Equal(nameof(HangfireJobExecutor.Execute), job.Method.Name);
        var enqueued = Assert.IsType<EnqueuedState>(state);
        Assert.Equal("email", enqueued.Queue);

        var payloadJson = (string)job.Args[0]!;
        var payload = JsonSerializer.Deserialize<JobPayload>(payloadJson, HangfireJobExecutor.PayloadJsonOptions);
        Assert.Equal("cleanup", payload!.Name);
        Assert.NotNull(payload.Arguments);

        var enqueuedTelemetry = Assert.Single(telemetry.Enqueued);
        Assert.Equal("cleanup", enqueuedTelemetry.Payload.Name);
        Assert.Equal(Now, enqueuedTelemetry.At);
    }

    [Fact]
    public void Enqueue_without_telemetry_does_not_throw()
    {
        var dispatcher = new HangfireJobDispatcher(
            new RecordingBackgroundJobClient(),
            Options.Create(new HangfireJobsOptions()),
            new FixedClock(Now));

        dispatcher.EnqueueAsync(JobPayload.Create("cleanup"));
    }

    [Fact]
    public async Task Enqueue_rejects_null_payload()
    {
        var dispatcher = new HangfireJobDispatcher(
            new RecordingBackgroundJobClient(),
            Options.Create(new HangfireJobsOptions()),
            new FixedClock(Now));

        await Assert.ThrowsAsync<ArgumentNullException>(() => dispatcher.EnqueueAsync(null!));
    }

    [Fact]
    public async Task Enqueue_rejects_payload_without_name()
    {
        var dispatcher = new HangfireJobDispatcher(
            new RecordingBackgroundJobClient(),
            Options.Create(new HangfireJobsOptions()),
            new FixedClock(Now));

        await Assert.ThrowsAsync<ArgumentException>(() => dispatcher.EnqueueAsync(new JobPayload(" ")));
    }

    [Fact]
    public async Task Enqueue_honours_cancellation()
    {
        var client = new RecordingBackgroundJobClient();
        var dispatcher = new HangfireJobDispatcher(
            client,
            Options.Create(new HangfireJobsOptions()),
            new FixedClock(Now));

        await Assert.ThrowsAsync<OperationCanceledException>(
            () => dispatcher.EnqueueAsync(JobPayload.Create("cleanup"), new CancellationToken(canceled: true)));
        Assert.Empty(client.Created);
    }
}
