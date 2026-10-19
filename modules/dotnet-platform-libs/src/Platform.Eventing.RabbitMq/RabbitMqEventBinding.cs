namespace Platform.Eventing.RabbitMq;

/// <summary>
/// Application-owned AMQP binding for one durable event payload type. The
/// routing key is resolved deterministically from the registration; the
/// adapter never derives names from payload contents.
/// </summary>
/// <param name="RoutingKey">The routing key used for the payload type.</param>
/// <param name="Exchange">The optional exchange override; <c>null</c> publishes to the configured default exchange.</param>
public sealed record RabbitMqEventBinding(string RoutingKey, string? Exchange = null)
{
    /// <summary>
    /// Validates the binding values.
    /// </summary>
    /// <exception cref="ArgumentException">A required value is missing or contains control characters.</exception>
    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(RoutingKey) || RoutingKey.Any(char.IsControl))
        {
            throw new ArgumentException("A routing key is required and must not contain control characters.", nameof(RoutingKey));
        }

        if (Exchange is not null && (string.IsNullOrWhiteSpace(Exchange) || Exchange.Any(char.IsControl)))
        {
            throw new ArgumentException("The exchange override must not be empty or contain control characters.", nameof(Exchange));
        }
    }
}
