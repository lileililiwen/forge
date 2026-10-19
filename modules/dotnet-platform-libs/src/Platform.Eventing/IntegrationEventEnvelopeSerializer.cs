using System.Text.Json;
using Platform.Core.Time;

namespace Platform.Eventing;

/// <summary>
/// Default <see cref="IIntegrationEventEnvelopeSerializer"/> backed by
/// <see cref="JsonSerializer"/>. The serializer reads the current time
/// from an injected <see cref="IClock"/> when the supplied event does
/// not already carry an <see cref="IntegrationEvent.OccurredAt"/> value
/// that differs from <see cref="DateTimeOffset.MinValue"/>.
/// </summary>
public sealed class IntegrationEventEnvelopeSerializer : IIntegrationEventEnvelopeSerializer
{
    private readonly IClock _clock;
    private readonly JsonSerializerOptions _options;

    /// <summary>
    /// Initializes a new <see cref="IntegrationEventEnvelopeSerializer"/>
    /// that reads the current time from the supplied
    /// <paramref name="clock"/> and uses the supplied JSON options.
    /// </summary>
    /// <param name="clock">The platform clock.</param>
    /// <param name="options">The JSON serializer options. When <c>null</c>, the documented defaults are applied.</param>
    /// <exception cref="ArgumentNullException"><paramref name="clock"/> is <c>null</c>.</exception>
    public IntegrationEventEnvelopeSerializer(IClock clock, JsonSerializerOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(clock);
        _clock = clock;
        _options = options ?? DefaultOptions();
    }

    /// <inheritdoc />
    public IntegrationEventEnvelope Serialize(IIntegrationEvent payload, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(payload);
        cancellationToken.ThrowIfCancellationRequested();

        var payloadType = payload.GetType();
        var occurredAt = payload is IntegrationEvent integrationEvent && integrationEvent.OccurredAt != default
            ? integrationEvent.OccurredAt
            : _clock.UtcNow;
        var correlationId = payload is IntegrationEvent integration
            ? integration.CorrelationId
            : null;
        var messageId = payload is IntegrationEvent integrationId
            ? integrationId.EventId.ToString("D")
            : Guid.NewGuid().ToString("D");
        var payloadJson = JsonSerializer.Serialize(payload, payloadType, _options);

        return new IntegrationEventEnvelope(
            MessageId: messageId,
            PayloadType: payloadType.AssemblyQualifiedName ?? payloadType.FullName ?? payloadType.Name,
            PayloadJson: payloadJson,
            OccurredAt: occurredAt,
            CorrelationId: correlationId);
    }

    private static JsonSerializerOptions DefaultOptions() => new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = false,
    };
}
