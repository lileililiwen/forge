using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Platform.Core.Time;
using Platform.Eventing.Contracts;

namespace Platform.Eventing.EfCore;

/// <summary>Hosted, transport-neutral dispatcher for an application-owned outbox.</summary>
public sealed class DurableOutboxDispatcher : IHostedService, IAsyncDisposable
{
    private readonly IOutboxStore _store;
    private readonly IDurableEventPublisher _publisher;
    private readonly IClock _clock;
    private readonly DurableEventingOptions _options;
    private readonly ILogger<DurableOutboxDispatcher> _logger;
    private readonly string _workerId;
    private readonly CancellationTokenSource _shutdown = new();
    private Task? _runTask;

    /// <summary>Creates a hosted outbox dispatcher.</summary>
    public DurableOutboxDispatcher(
        IOutboxStore store,
        IDurableEventPublisher publisher,
        IClock clock,
        DurableEventingOptions options,
        ILogger<DurableOutboxDispatcher> logger,
        string workerId)
    {
        ArgumentNullException.ThrowIfNull(store);
        ArgumentNullException.ThrowIfNull(publisher);
        ArgumentNullException.ThrowIfNull(clock);
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(logger);
        if (string.IsNullOrWhiteSpace(workerId)) throw new ArgumentException("Worker id is required.", nameof(workerId));
        options.Validate();
        _store = store;
        _publisher = publisher;
        _clock = clock;
        _options = options;
        _logger = logger;
        _workerId = workerId;
    }

    /// <inheritdoc />
    public Task StartAsync(CancellationToken cancellationToken)
    {
        if (_runTask is not null) return Task.CompletedTask;
        _runTask = RunAsync(_shutdown.Token);
        return Task.CompletedTask;
    }

    /// <inheritdoc />
    public async Task StopAsync(CancellationToken cancellationToken)
    {
        _shutdown.Cancel();
        if (_runTask is not null)
            await _runTask.WaitAsync(cancellationToken).ConfigureAwait(false);
    }

    /// <summary>Dispatches one bounded batch and returns the number of successful publications.</summary>
    public async Task<int> DispatchBatchAsync(CancellationToken cancellationToken = default)
    {
        var claimed = await _store.ClaimAsync(_clock.UtcNow, _workerId, _options.LeaseDuration, _options.BatchSize, cancellationToken).ConfigureAwait(false);
        var published = 0;
        foreach (var message in claimed)
        {
            try
            {
                await _publisher.PublishAsync(message.Envelope, cancellationToken).ConfigureAwait(false);
                await _store.MarkSucceededAsync(message.MessageId, _workerId, cancellationToken).ConfigureAwait(false);
                published++;
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception exception)
            {
                var failure = new DurableDispatchFailure("durable_event.publish_failed", "The event publisher failed.");
                await _store.MarkFailedAsync(message.MessageId, _workerId, failure, cancellationToken).ConfigureAwait(false);
#pragma warning disable CA1848 // Logger source-generator delegates are not available in the platform contract.
                _logger.LogWarning(exception, "Durable event {MessageId} failed publication.", message.MessageId);
#pragma warning restore CA1848
            }
        }
        return published;
    }

    /// <inheritdoc />
    public async ValueTask DisposeAsync()
    {
        _shutdown.Cancel();
        if (_runTask is not null)
        {
            try { await _runTask.ConfigureAwait(false); }
            catch (OperationCanceledException) when (_shutdown.IsCancellationRequested) { }
        }
        _shutdown.Dispose();
    }

    private async Task RunAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            await DispatchBatchAsync(cancellationToken).ConfigureAwait(false);
            await Task.Delay(_options.PollInterval, cancellationToken).ConfigureAwait(false);
        }
    }
}
