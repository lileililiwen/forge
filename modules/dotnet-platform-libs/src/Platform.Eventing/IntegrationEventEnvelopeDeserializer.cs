using System.Text.Json;
using Platform.Core.Time;

namespace Platform.Eventing;

/// <summary>
/// Default <see cref="IIntegrationEventEnvelopeDeserializer"/> backed
/// by <see cref="JsonSerializer"/>. The deserializer resolves the
/// payload type from <see cref="IntegrationEventEnvelope.PayloadType"/>
/// and reconstructs the typed event with the envelope's
/// <see cref="IntegrationEventEnvelope.OccurredAt"/> and
/// <see cref="IntegrationEventEnvelope.CorrelationId"/> values.
/// </summary>
public sealed class IntegrationEventEnvelopeDeserializer : IIntegrationEventEnvelopeDeserializer
{
    private readonly IClock _clock;
    private readonly JsonSerializerOptions _options;

    /// <summary>
    /// Initializes a new <see cref="IntegrationEventEnvelopeDeserializer"/>
    /// that reads the current time from the supplied
    /// <paramref name="clock"/> and uses the supplied JSON options.
    /// </summary>
    /// <param name="clock">The platform clock.</param>
    /// <param name="options">The JSON serializer options. When <c>null</c>, the documented defaults are applied.</param>
    /// <exception cref="ArgumentNullException"><paramref name="clock"/> is <c>null</c>.</exception>
    public IntegrationEventEnvelopeDeserializer(IClock clock, JsonSerializerOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(clock);
        _clock = clock;
        _options = options ?? DefaultOptions();
    }

    /// <inheritdoc />
    public IIntegrationEvent Deserialize(IntegrationEventEnvelope envelope, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(envelope);
        cancellationToken.ThrowIfCancellationRequested();

        var payloadType = ResolveType(envelope.PayloadType);
        if (payloadType is null)
        {
            throw new InvalidOperationException(
                $"Cannot resolve payload type '{envelope.PayloadType}'.");
        }

        var payload = JsonSerializer.Deserialize(envelope.PayloadJson, payloadType, _options)
            ?? throw new InvalidOperationException(
                $"Deserialiser returned null for payload type '{envelope.PayloadType}'.");

        if (payload is IntegrationEvent integrationEvent
            && integrationEvent.OccurredAt == default
            && integrationEvent.CorrelationId is null)
        {
            var occurredAt = envelope.OccurredAt == default ? _clock.UtcNow : envelope.OccurredAt;
            return integrationEvent with
            {
                OccurredAt = occurredAt,
                CorrelationId = envelope.CorrelationId,
            };
        }

        return (IIntegrationEvent)payload;
    }

    private static Type? ResolveType(string payloadType)
    {
        if (string.IsNullOrWhiteSpace(payloadType))
        {
            return null;
        }

        var resolved = Type.GetType(payloadType, throwOnError: false);
        if (resolved is not null)
        {
            return resolved;
        }

        foreach (var assembly in AppDomain.CurrentDomain.GetAssemblies())
        {
            var found = assembly.GetType(payloadType, throwOnError: false);
            if (found is not null)
            {
                return found;
            }
        }

        return null;
    }

    private static JsonSerializerOptions DefaultOptions() => new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
    };
}
