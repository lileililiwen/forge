using System.Text.Json;
using Hangfire;
using Hangfire.Common;
using Hangfire.States;
using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// Hangfire-backed <see cref="IJobDispatcher"/>. Serializes the platform
/// payload and enqueues it through the Hangfire client so the job executes
/// inside a scoped, context-restored <see cref="HangfireJobExecutor"/>
/// invocation. The dispatcher never owns retries; Hangfire's automatic
/// retry policy remains the retry owner.
/// </summary>
public sealed class HangfireJobDispatcher : IJobDispatcher
{
    private readonly IBackgroundJobClient _client;
    private readonly HangfireJobsOptions _options;
    private readonly IClock _clock;
    private readonly IJobTelemetry? _telemetry;

    /// <summary>
    /// Initializes a new instance of the <see cref="HangfireJobDispatcher"/> class.
    /// </summary>
    /// <param name="client">The Hangfire background-job client.</param>
    /// <param name="options">The validated adapter options.</param>
    /// <param name="clock">The platform clock.</param>
    /// <param name="telemetry">The optional platform job telemetry.</param>
    public HangfireJobDispatcher(
        IBackgroundJobClient client,
        IOptions<HangfireJobsOptions> options,
        IClock clock,
        IJobTelemetry? telemetry = null)
    {
        ArgumentNullException.ThrowIfNull(client);
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(clock);
        _client = client;
        _options = options.Value;
        _clock = clock;
        _telemetry = telemetry;
    }

    /// <inheritdoc />
    public Task EnqueueAsync(JobPayload payload, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(payload);
        if (string.IsNullOrWhiteSpace(payload.Name))
        {
            throw new ArgumentException("The job payload must carry a non-empty name.", nameof(payload));
        }

        cancellationToken.ThrowIfCancellationRequested();

        var payloadJson = JsonSerializer.Serialize(payload, HangfireJobExecutor.PayloadJsonOptions);
        _client.Create(HangfireJobExecutor.CreateExecuteJob(payloadJson), new EnqueuedState(_options.Queue));
        _telemetry?.JobEnqueued(payload, _clock.UtcNow);

        return Task.CompletedTask;
    }
}
