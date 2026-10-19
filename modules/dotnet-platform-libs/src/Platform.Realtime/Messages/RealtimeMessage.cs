namespace Platform.Realtime.Messages;

/// <summary>
/// A single realtime message. Payloads are bounded; oversized values are
/// rejected by <see cref="Create"/> so a transport never forwards an unbounded
/// frame. Delivery is non-durable unless the application supplies its own
/// replay or resynchronization mechanism (see <see cref="Delivery.RealtimeResyncRequest"/>).
/// </summary>
public sealed class RealtimeMessage
{
    /// <summary>Gets the optional logical channel or topic of the message.</summary>
    public string? Channel { get; private init; }

    /// <summary>Gets the optional target tenant. <c>null</c> is treated as a broadcast.</summary>
    public string? TargetTenantId { get; private init; }

    /// <summary>Gets the optional target subject (user) within the tenant.</summary>
    public string? TargetSubjectId { get; private init; }

    /// <summary>Gets the optional encoded payload bytes (bounded by the transport options).</summary>
    public ReadOnlyMemory<byte>? Payload { get; private init; }

    /// <summary>Gets the optional text payload (bounded by the transport options).</summary>
    public string? PayloadText { get; private init; }

    private RealtimeMessage()
    {
    }

    /// <summary>
    /// Creates a bounded message. Either <paramref name="payloadText"/> or
    /// <paramref name="payload"/> may be supplied, not both.
    /// </summary>
    /// <param name="maxPayloadBytes">The maximum accepted payload size in bytes.</param>
    /// <param name="channel">The optional channel name.</param>
    /// <param name="targetTenantId">The optional target tenant.</param>
    /// <param name="targetSubjectId">The optional target subject.</param>
    /// <param name="payloadText">The optional text payload.</param>
    /// <param name="payload">The optional binary payload.</param>
    /// <exception cref="ArgumentException">When both payload forms are supplied or either exceeds <paramref name="maxPayloadBytes"/>.</exception>
    public static RealtimeMessage Create(
        int maxPayloadBytes,
        string? channel = null,
        string? targetTenantId = null,
        string? targetSubjectId = null,
        string? payloadText = null,
        ReadOnlyMemory<byte>? payload = null)
    {
        if (payloadText is not null && payload is not null)
            throw new ArgumentException("Provide either payloadText or payload, not both.", nameof(payloadText));

        if (payloadText is { Length: > 0 } && payloadText.Length > maxPayloadBytes)
            throw new ArgumentException(
                $"payloadText exceeds the maximum allowed size of {maxPayloadBytes} bytes.", nameof(payloadText));

        if (payload is { } bytes && bytes.Length > maxPayloadBytes)
            throw new ArgumentException(
                $"payload exceeds the maximum allowed size of {maxPayloadBytes} bytes.", nameof(payload));

        return new RealtimeMessage
        {
            Channel = channel,
            TargetTenantId = targetTenantId,
            TargetSubjectId = targetSubjectId,
            PayloadText = payloadText,
            Payload = payload,
        };
    }

    /// <summary>Returns the effective size in bytes of the selected payload form.</summary>
    public int GetPayloadByteCount() =>
        Payload.HasValue ? Payload.Value.Length : (PayloadText?.Length ?? 0);
}
