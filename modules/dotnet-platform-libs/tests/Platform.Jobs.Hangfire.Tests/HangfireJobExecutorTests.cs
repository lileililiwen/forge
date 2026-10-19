using System.Text.Json;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Time;
using Platform.Jobs.Hangfire;

namespace Platform.Jobs.Hangfire.Tests;

public class HangfireJobExecutorTests : HangfireTest
{
    private static readonly DateTimeOffset Now = new(2026, 9, 9, 12, 0, 0, TimeSpan.Zero);

    private sealed class StubRegistry(IReadOnlyList<RecurringJobDescriptor> descriptors) : IRecurringJobRegistry
    {
        public IReadOnlyList<RecurringJobDescriptor> Registered { get; } = descriptors;

        public void Register(RecurringJobDescriptor descriptor) => throw new NotSupportedException();
    }

    private static HangfireJobExecutor CreateExecutor(
        Action<ServiceCollection>? configure = null,
        IRecurringJobRegistry? registry = null,
        RecordingJobTelemetry? telemetry = null)
    {
        var services = new ServiceCollection();
        configure?.Invoke(services);
        var provider = services.BuildServiceProvider();
        return new HangfireJobExecutor(
            provider,
            registry ?? new StubRegistry(Array.Empty<RecurringJobDescriptor>()),
            new FixedClock(Now),
            telemetry);
    }

    [Fact]
    public async Task Execute_invokes_the_payload_handler_with_normalized_arguments()
    {
        var handler = new RecordingPayloadHandler();
        var executor = CreateExecutor(services => services.AddSingleton<IJobPayloadHandler>(handler));
        var payloadJson = JsonSerializer.Serialize(
            JobPayload.Create("cleanup", new Dictionary<string, object?> { ["name"] = "t1", ["count"] = 3, ["enabled"] = true }),
            HangfireJobExecutor.PayloadJsonOptions);

        await executor.Execute(payloadJson, new FakeJobCancellationToken());

        var handled = Assert.Single(handler.Handled);
        Assert.Equal("cleanup", handled.Name);
        Assert.Equal("t1", handled.Arguments!["name"]);
        Assert.Equal(3L, handled.Arguments!["count"]);
        Assert.Equal(true, handled.Arguments!["enabled"]);
    }

    [Fact]
    public async Task Execute_records_successful_telemetry_with_clock_time()
    {
        var telemetry = new RecordingJobTelemetry();
        var executor = CreateExecutor(
            services => services.AddSingleton<IJobPayloadHandler>(new RecordingPayloadHandler()),
            telemetry: telemetry);

        await executor.Execute(
            JsonSerializer.Serialize(JobPayload.Create("cleanup"), HangfireJobExecutor.PayloadJsonOptions),
            new FakeJobCancellationToken());

        var executed = Assert.Single(telemetry.Executed);
        Assert.Equal("cleanup", executed.Name);
        Assert.Equal(Now, executed.At);
        Assert.Empty(telemetry.Failed);
    }

    [Fact]
    public async Task Execute_records_a_redacted_failure_and_rethrows()
    {
        var telemetry = new RecordingJobTelemetry();
        var handler = new RecordingPayloadHandler { Failure = new InvalidOperationException("secret connection string failed") };
        var executor = CreateExecutor(
            services => services.AddSingleton<IJobPayloadHandler>(handler),
            telemetry: telemetry);

        await Assert.ThrowsAsync<InvalidOperationException>(
            () => executor.Execute(
                JsonSerializer.Serialize(JobPayload.Create("cleanup"), HangfireJobExecutor.PayloadJsonOptions),
                new FakeJobCancellationToken()));

        var failure = Assert.Single(telemetry.Failed);
        Assert.Equal("cleanup", failure.Name);
        Assert.Equal(HangfireJobExecutor.ExecutionFailedCode, failure.Error.Code);
        Assert.Equal("A background job failed.", failure.Error.Message);
        Assert.Equal("InvalidOperationException", failure.Error.Metadata!["exceptionType"]);
        Assert.DoesNotContain("secret", failure.Error.Message, StringComparison.OrdinalIgnoreCase);
        Assert.Empty(telemetry.Executed);
    }

    [Fact]
    public async Task Execute_does_not_record_cancellation_as_a_failure()
    {
        var telemetry = new RecordingJobTelemetry();
        var handler = new RecordingPayloadHandler { Failure = new OperationCanceledException() };
        var executor = CreateExecutor(
            services => services.AddSingleton<IJobPayloadHandler>(handler),
            telemetry: telemetry);

        await Assert.ThrowsAsync<OperationCanceledException>(
            () => executor.Execute(
                JsonSerializer.Serialize(JobPayload.Create("cleanup"), HangfireJobExecutor.PayloadJsonOptions),
                new FakeJobCancellationToken()));

        Assert.Empty(telemetry.Failed);
        Assert.Empty(telemetry.Executed);
    }

    [Fact]
    public async Task Execute_without_a_registered_handler_fails_with_a_clear_error()
    {
        var executor = CreateExecutor();

        var exception = await Assert.ThrowsAsync<InvalidOperationException>(
            () => executor.Execute(
                JsonSerializer.Serialize(JobPayload.Create("cleanup"), HangfireJobExecutor.PayloadJsonOptions),
                new FakeJobCancellationToken()));

        Assert.Contains("IJobPayloadHandler", exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Execute_rejects_malformed_payloads()
    {
        var executor = CreateExecutor(services => services.AddSingleton<IJobPayloadHandler>(new RecordingPayloadHandler()));

        await Assert.ThrowsAsync<InvalidOperationException>(() => executor.Execute("not-json", new FakeJobCancellationToken()));
        await Assert.ThrowsAsync<InvalidOperationException>(() => executor.Execute(string.Empty, new FakeJobCancellationToken()));
        await Assert.ThrowsAsync<InvalidOperationException>(
            () => executor.Execute(
                JsonSerializer.Serialize(new JobPayload(" "), HangfireJobExecutor.PayloadJsonOptions),
                new FakeJobCancellationToken()));
    }

    [Fact]
    public async Task ExecuteRecurring_resolves_the_registered_handler()
    {
        var handler = new RecordingRecurringHandler();
        var registry = new StubRegistry(new[]
        {
            new RecurringJobDescriptor("nightly-cleanup", "0 2 * * *", typeof(RecordingRecurringHandler)),
        });
        var telemetry = new RecordingJobTelemetry();
        var executor = CreateExecutor(
            services => services.AddSingleton<IRecurringJobHandler>(handler),
            registry,
            telemetry);

        await executor.ExecuteRecurring("nightly-cleanup", new FakeJobCancellationToken());

        Assert.Equal(1, handler.Executions);
        var executed = Assert.Single(telemetry.Executed);
        Assert.Equal("nightly-cleanup", executed.Name);
    }

    [Fact]
    public async Task ExecuteRecurring_with_an_unknown_name_fails_with_a_clear_error()
    {
        var executor = CreateExecutor(services => services.AddSingleton<IRecurringJobHandler>(new RecordingRecurringHandler()));

        var exception = await Assert.ThrowsAsync<InvalidOperationException>(
            () => executor.ExecuteRecurring("missing", new FakeJobCancellationToken()));

        Assert.Contains("missing", exception.Message, StringComparison.Ordinal);
        Assert.Contains("IRecurringJobRegistry", exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task ExecuteRecurring_records_a_redacted_failure_and_rethrows()
    {
        var telemetry = new RecordingJobTelemetry();
        var registry = new StubRegistry(new[]
        {
            new RecurringJobDescriptor("nightly-cleanup", "0 2 * * *", typeof(RecordingRecurringHandler)),
        });
        var executor = CreateExecutor(
            services => services.AddSingleton<IRecurringJobHandler>(new RecordingRecurringHandler { Failure = new TimeoutException("secret host name") }),
            registry,
            telemetry);

        await Assert.ThrowsAsync<TimeoutException>(() => executor.ExecuteRecurring("nightly-cleanup", new FakeJobCancellationToken()));

        var failure = Assert.Single(telemetry.Failed);
        Assert.Equal(HangfireJobExecutor.ExecutionFailedCode, failure.Error.Code);
        Assert.Equal("TimeoutException", failure.Error.Metadata!["exceptionType"]);
        Assert.DoesNotContain("secret", failure.Error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task ExecuteRecurring_resolves_handlers_registered_under_the_interface()
    {
        var registry = new StubRegistry(new[]
        {
            new RecurringJobDescriptor("nightly-cleanup", "0 2 * * *", typeof(RecordingRecurringHandler)),
        });
        var executor = CreateExecutor(
            services => services.AddSingleton<IRecurringJobHandler>(new RecordingRecurringHandler()),
            registry);

        await executor.ExecuteRecurring("nightly-cleanup", new FakeJobCancellationToken());
    }

    [Fact]
    public async Task ExecuteRecurring_rejects_handlers_that_are_not_registered_or_not_recurring_handlers()
    {
        var registry = new StubRegistry(new[]
        {
            new RecurringJobDescriptor("unregistered-handler", "* * * * *", typeof(RecordingRecurringHandler)),
            new RecurringJobDescriptor("wrong-type", "* * * * *", typeof(RecordingPayloadHandler)),
        });
        var executor = CreateExecutor(registry: registry);

        await Assert.ThrowsAsync<InvalidOperationException>(() => executor.ExecuteRecurring("unregistered-handler", new FakeJobCancellationToken()));
        await Assert.ThrowsAsync<InvalidOperationException>(() => executor.ExecuteRecurring("wrong-type", new FakeJobCancellationToken()));
    }
}
