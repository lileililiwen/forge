using Platform.Eventing;

namespace Platform.Testing.Eventing;

/// <summary>
/// Recording <see cref="IEventBus"/> implementation for tests. Stores every
/// published envelope in invocation order and supports filter and reset
/// helpers. The fake never throws; tests that need failure behaviour
/// should pair it with <c>Platform.Testing.FailureInjection</c>.
/// </summary>
public sealed class RecordingEventBus : IEventBus
{
    private readonly object _gate = new();
    private readonly List<IntegrationEventEnvelope> _envelopes = new();

    /// <inheritdoc />
    public Task PublishAsync(IntegrationEventEnvelope envelope, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(envelope);
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate)
        {
            _envelopes.Add(envelope);
        }
        return Task.CompletedTask;
    }

    /// <summary>Returns a snapshot of every published envelope in order.</summary>
    public IReadOnlyList<IntegrationEventEnvelope> Envelopes
    {
        get
        {
            lock (_gate)
            {
                return _envelopes.ToArray();
            }
        }
    }

    /// <summary>Returns the envelopes whose <see cref="IntegrationEventEnvelope.PayloadType"/> matches the supplied assembly-qualified name.</summary>
    public IReadOnlyList<IntegrationEventEnvelope> EnvelopesOfType(string payloadType)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(payloadType);
        lock (_gate)
        {
            return _envelopes
                .Where(e => string.Equals(e.PayloadType, payloadType, StringComparison.Ordinal))
                .ToArray();
        }
    }

    /// <summary>Clears every recorded envelope.</summary>
    public void Reset()
    {
        lock (_gate)
        {
            _envelopes.Clear();
        }
    }
}
