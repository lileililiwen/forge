namespace Platform.Eventing.RabbitMq;

/// <summary>
/// Application-owned mapping from durable payload types to AMQP bindings.
/// The adapter resolves bindings through this contract and fails closed when
/// a payload type has no registration.
/// </summary>
public interface IRabbitMqEventTopology
{
    /// <summary>Resolves the binding for a payload type, or <c>null</c> when the type is not registered.</summary>
    RabbitMqEventBinding? Resolve(string payloadType);
}

/// <summary>Fail-closed default topology that registers no payload types.</summary>
public sealed class UnconfiguredRabbitMqEventTopology : IRabbitMqEventTopology
{
    /// <inheritdoc />
    public RabbitMqEventBinding? Resolve(string payloadType) => null;
}

/// <summary>
/// Thread-safe <see cref="IRabbitMqEventTopology"/> that applications populate
/// explicitly. The adapter never infers bindings from payload types.
/// </summary>
public sealed class RabbitMqEventTopology : IRabbitMqEventTopology
{
    private readonly System.Collections.Concurrent.ConcurrentDictionary<string, RabbitMqEventBinding> _bindings =
        new(StringComparer.Ordinal);

    /// <summary>Registers the binding for a payload type.</summary>
    /// <param name="payloadType">The durable payload type.</param>
    /// <param name="binding">The AMQP binding.</param>
    /// <returns>The same topology for chaining.</returns>
    /// <exception cref="ArgumentException">The payload type is invalid.</exception>
    /// <exception cref="ArgumentNullException">The binding is <c>null</c>.</exception>
    public RabbitMqEventTopology Map(string payloadType, RabbitMqEventBinding binding)
    {
        if (string.IsNullOrWhiteSpace(payloadType) || payloadType.Any(char.IsControl))
        {
            throw new ArgumentException("A payload type is required and must not contain control characters.", nameof(payloadType));
        }

        ArgumentNullException.ThrowIfNull(binding);
        binding.Validate();
        _bindings[payloadType] = binding;
        return this;
    }

    /// <inheritdoc />
    public RabbitMqEventBinding? Resolve(string payloadType) =>
        _bindings.TryGetValue(payloadType, out var binding) ? binding : null;
}
