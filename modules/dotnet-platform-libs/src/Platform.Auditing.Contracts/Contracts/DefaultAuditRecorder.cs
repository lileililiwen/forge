using System.Collections.Generic;
using System.Threading;
using System.Threading.Channels;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Auditing.Contracts.Common;

namespace Platform.Auditing.Contracts;

/// <summary>
/// Default <see cref="IAuditRecorder"/>. It enriches, masks, and bounds the metadata of each event,
/// then dispatches it to the configured sink. Synchronous mode awaits the sink inline; bounded
/// async mode enqueues onto a bounded channel and publishes on a background reader (dropping when
/// full). The failure policy decides whether a sink fault aborts (fail-closed) or is logged and
/// dead-lettered (fail-open).
/// </summary>
public sealed class DefaultAuditRecorder : IAuditRecorder, IDisposable, IAsyncDisposable
{
    private readonly AuditOptions _options;
    private readonly IReadOnlyList<IAuditEnricher> _enrichers;
    private readonly IAuditMasker _masker;
    private readonly IAuditSink _sink;
    private readonly IAuditDeadLetterSink _deadLetter;
    private readonly ILogger<DefaultAuditRecorder>? _logger;
    private readonly Channel<AuditEvent>? _channel;
    private readonly Task? _reader;
    private readonly CancellationTokenSource _shutdown = new();
    private readonly object _flushGate = new();
    private int _pending;
    private TaskCompletionSource? _flushCompletion;
    private bool _disposed;

    /// <summary>Initializes a new recorder with the supplied dependencies.</summary>
    public DefaultAuditRecorder(
        IOptions<AuditOptions> options,
        IEnumerable<IAuditEnricher> enrichers,
        IAuditMasker masker,
        IAuditSink sink,
        IAuditDeadLetterSink deadLetter,
        ILogger<DefaultAuditRecorder>? logger = null)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(enrichers);
        ArgumentNullException.ThrowIfNull(masker);
        ArgumentNullException.ThrowIfNull(sink);
        ArgumentNullException.ThrowIfNull(deadLetter);
        _options = options.Value;
        _enrichers = enrichers.ToArray();
        _masker = masker;
        _sink = sink;
        _deadLetter = deadLetter;
        _logger = logger;

        if (_options.PublishMode == AuditPublishMode.BoundedAsync)
        {
            _channel = Channel.CreateBounded<AuditEvent>(new BoundedChannelOptions(_options.BoundedCapacity)
            {
                FullMode = BoundedChannelFullMode.DropWrite,
                SingleReader = true,
                SingleWriter = false,
            });
            _reader = Task.Run(PumpAsync);
        }
    }

    /// <inheritdoc />
    public Task RecordAsync(AuditEvent auditEvent, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(auditEvent);
        if (!_options.IsCategoryEnabled(auditEvent.Category)) return Task.CompletedTask;

        var final = BuildFinalEvent(auditEvent);
        if (_channel is not null)
        {
            if (_channel.Writer.TryWrite(final))
            {
                Interlocked.Increment(ref _pending);
            }

            return Task.CompletedTask;
        }

        return DispatchAsync(final, cancellationToken);
    }

    /// <summary>Waits for the bounded-async reader to drain items written so far. No-op in synchronous mode.</summary>
    /// <param name="cancellationToken">A token that may cancel the wait.</param>
    /// <returns>A task completing when the queue is empty.</returns>
    public Task FlushAsync(CancellationToken cancellationToken = default)
    {
        if (_channel is null) return Task.CompletedTask;
        TaskCompletionSource tcs;
        lock (_flushGate)
        {
            if (_pending == 0) return Task.CompletedTask;
            tcs = _flushCompletion ??= new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        }

        return tcs.Task.WaitAsync(cancellationToken);
    }

    /// <inheritdoc />
    public void Dispose()
    {
        if (_disposed) return;
        DisposeAsync().AsTask().GetAwaiter().GetResult();
    }

    /// <inheritdoc />
    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;
        if (_channel is not null)
        {
            _channel.Writer.TryComplete();
            _shutdown.Cancel();
            if (_reader is not null)
            {
                try
                {
                    await _reader.ConfigureAwait(false);
                }
                catch (OperationCanceledException)
                {
                    // Expected on shutdown.
                }
            }

            lock (_flushGate)
            {
                _flushCompletion?.TrySetCanceled();
                _flushCompletion = null;
            }
        }

        _shutdown.Dispose();
    }

    private AuditEvent BuildFinalEvent(AuditEvent auditEvent)
    {
        var next = auditEvent;
        foreach (var enricher in _enrichers)
        {
            next = enricher.Enrich(next);
        }

        if (next.Metadata.Count == 0) return next;

        var bounded = new Dictionary<string, string>(capacity: Math.Min(next.Metadata.Count, _options.MaxMetadataEntries));
        foreach (var pair in next.Metadata)
        {
            if (bounded.Count >= _options.MaxMetadataEntries) break;
            if (string.IsNullOrWhiteSpace(pair.Key)) continue;
            var masked = _masker.Mask(pair.Key, pair.Value);
            bounded[pair.Key] = masked.Length > _options.MaxMetadataValueLength
                ? masked.Substring(0, _options.MaxMetadataValueLength)
                : masked;
        }

        return next with { Metadata = bounded };
    }

    private async Task DispatchAsync(AuditEvent auditEvent, CancellationToken cancellationToken)
    {
        try
        {
            await _sink.RecordAsync(auditEvent, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (_options.FailurePolicy == AuditFailurePolicy.FailClosed)
        {
            throw new AuditRecordingException("The audit sink failed under the fail-closed policy.", ex);
        }
        catch (Exception ex)
        {
#pragma warning disable CA1848 // Logger source-generator delegates are not used in the contract package.
            _logger?.LogWarning(ex, "Audit sink failed; event dropped under the fail-open policy. Action={Action}.", auditEvent.Action);
#pragma warning restore CA1848
            await _deadLetter.RecordFailedAsync(auditEvent, "sink-failure", cancellationToken).ConfigureAwait(false);
        }
    }

    private async Task PumpAsync()
    {
        var reader = _channel!.Reader;
        while (await reader.WaitToReadAsync(_shutdown.Token).ConfigureAwait(false))
        {
            while (reader.TryRead(out var auditEvent))
            {
                if (_shutdown.IsCancellationRequested) return;
                try
                {
                    await DispatchAsync(auditEvent, _shutdown.Token).ConfigureAwait(false);
                }
                catch (OperationCanceledException) when (_shutdown.IsCancellationRequested)
                {
                    return;
                }
                catch (Exception ex)
                {
#pragma warning disable CA1848
                    _logger?.LogError(ex, "Audit pump failed to dispatch event {Action}.", auditEvent.Action);
#pragma warning restore CA1848
                }
                finally
                {
                    if (Interlocked.Decrement(ref _pending) == 0)
                    {
                        TaskCompletionSource? tcs;
                        lock (_flushGate)
                        {
                            tcs = _flushCompletion;
                            _flushCompletion = null;
                        }

                        tcs?.TrySetResult();
                    }
                }
            }
        }
    }
}
