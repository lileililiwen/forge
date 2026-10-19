namespace Platform.Eventing.RabbitMq;

/// <summary>Documented health of the RabbitMQ adapter derived from the most recent publish attempt.</summary>
public enum RabbitMqEventingProviderState
{
    /// <summary>The last publish attempt was confirmed by the broker.</summary>
    Healthy,

    /// <summary>The last publish attempt could not reach the broker or was not confirmed.</summary>
    Unavailable,
}

/// <summary>
/// Documented snapshot of the RabbitMQ adapter status. The snapshot carries
/// only the provider name, health state, and the last stable error code; it
/// never carries broker response bodies or credentials.
/// </summary>
/// <param name="Provider">The stable provider name, always <c>rabbitmq</c>.</param>
/// <param name="State">The derived health state.</param>
/// <param name="LastErrorCode">The last stable error code, or <c>null</c> when the last attempt succeeded.</param>
public sealed record RabbitMqEventingProviderStatus(string Provider, RabbitMqEventingProviderState State, string? LastErrorCode = null)
{
    /// <summary>The stable provider name reported by <see cref="RabbitMqDurableEventPublisher"/>.</summary>
    public const string ProviderName = "rabbitmq";
}
