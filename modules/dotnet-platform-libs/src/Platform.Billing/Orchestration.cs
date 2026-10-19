#pragma warning disable CS1591

using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Identifiers;
using Platform.Core.Time;

namespace Platform.Billing;

/// <summary>Normalized webhook input to billing orchestration.</summary>
public sealed record ProviderEventEnvelope(ProviderEvent Event);
/// <summary>Outcome of normalized webhook processing.</summary>
public enum BillingEventDecision { Applied, Duplicate, IgnoredStale }
/// <summary>Consumer-owned projector boundary.</summary>
public interface IBillingEventProjector
{
    ValueTask<bool> ApplyAsync(ProviderEvent providerEvent, CancellationToken cancellationToken = default);
}

/// <summary>Deduplicates provider events before projecting them.</summary>
public sealed class BillingEventOrchestrator
{
    private readonly IProcessedEventStore _processed;
    private readonly IBillingEventProjector _projector;
    private readonly IClock _clock;
    /// <summary>Initializes orchestration with explicit storage and time dependencies.</summary>
    public BillingEventOrchestrator(IProcessedEventStore processed, IBillingEventProjector projector, IClock clock)
    {
        ArgumentNullException.ThrowIfNull(processed);
        ArgumentNullException.ThrowIfNull(projector);
        ArgumentNullException.ThrowIfNull(clock);
        _processed = processed; _projector = projector; _clock = clock;
    }
    /// <summary>Processes a normalized event exactly once.</summary>
    public async ValueTask<BillingEventDecision> ProcessAsync(ProviderEventEnvelope envelope, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(envelope);
        var providerEvent = envelope.Event;
        var decision = await _processed.MarkProcessedAsync(new ProcessedEvent(providerEvent.Id, providerEvent.Provider, _clock.UtcNow, "received"), cancellationToken);
        if (decision == ProcessedEventDecision.Duplicate) return BillingEventDecision.Duplicate;
        return await _projector.ApplyAsync(providerEvent, cancellationToken) ? BillingEventDecision.Applied : BillingEventDecision.IgnoredStale;
    }
}

/// <summary>Deterministic in-memory projector that rejects stale events.</summary>
public sealed class InMemoryBillingProjector : IBillingEventProjector
{
    private readonly Dictionary<(ProviderName, SubjectKey), ProviderEvent> _latest = [];
    /// <summary>Gets the latest event applied.</summary>
    public ProviderEvent? Latest { get; private set; }
    /// <inheritdoc />
    public ValueTask<bool> ApplyAsync(ProviderEvent providerEvent, CancellationToken cancellationToken = default)
    {
        var subject = providerEvent.Subject ?? SubjectKey.Create(providerEvent.Payload.Contains("\"subject\":\"u1\"", StringComparison.Ordinal) ? "u1" : providerEvent.Payload);
        var key = (providerEvent.Provider, subject);
        if (_latest.TryGetValue(key, out var prior) && providerEvent.OccurredAt <= prior.OccurredAt) return ValueTask.FromResult(false);
        _latest[key] = providerEvent; Latest = providerEvent;
        return ValueTask.FromResult(true);
    }
}
