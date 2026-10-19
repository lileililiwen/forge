namespace Platform.Eventing.RabbitMq;

/// <summary>
/// Configuration for <see cref="RabbitMqDurableEventPublisher"/>. Values are
/// validated by <see cref="Validate"/> at registration and construction; error
/// messages never echo the password or any other secret.
/// </summary>
public sealed class RabbitMqEventingOptions
{
    /// <summary>
    /// The configuration section name applications typically bind this
    /// options type from.
    /// </summary>
    public const string SectionName = "Eventing:RabbitMq";

    /// <summary>Gets or sets the broker host name. Defaults to <c>localhost</c>.</summary>
    public string HostName { get; set; } = "localhost";

    /// <summary>Gets or sets the broker port. Defaults to <c>5672</c>; must be between 1 and 65535.</summary>
    public int Port { get; set; } = 5672;

    /// <summary>Gets or sets the virtual host. Defaults to <c>/</c>.</summary>
    public string VirtualHost { get; set; } = "/";

    /// <summary>Gets or sets the user name. Defaults to <c>guest</c>.</summary>
    public string UserName { get; set; } = "guest";

    /// <summary>Gets or sets the password. The value is never written to diagnostics.</summary>
    public string Password { get; set; } = string.Empty;

    /// <summary>Gets or sets whether the connection uses TLS. Defaults to <c>false</c>.</summary>
    public bool UseSsl { get; set; }

    /// <summary>
    /// Gets or sets the application-owned exchange envelopes are published to
    /// when a binding does not override it. Required; the adapter never
    /// invents an exchange name.
    /// </summary>
    public string Exchange { get; set; } = string.Empty;

    /// <summary>Gets or sets the AMQP exchange type used when the adapter declares the exchange. Defaults to <c>topic</c>.</summary>
    public string ExchangeType { get; set; } = "topic";

    /// <summary>
    /// Gets or sets whether the adapter declares the configured exchange when
    /// it opens a channel. Defaults to <c>false</c>; topology creation stays
    /// application-owned. Queues are never declared by the adapter.
    /// </summary>
    public bool DeclareExchange { get; set; }

    /// <summary>Gets or sets whether a declared exchange is durable. Defaults to <c>true</c>.</summary>
    public bool DurableExchange { get; set; } = true;

    /// <summary>Gets or sets the bounded connect wait. Defaults to 10 seconds; must be between 1 second and 5 minutes.</summary>
    public TimeSpan ConnectTimeout { get; set; } = TimeSpan.FromSeconds(10);

    /// <summary>Gets or sets the bounded publisher-confirmation wait. Defaults to 5 seconds; must be between 100 milliseconds and 5 minutes.</summary>
    public TimeSpan ConfirmTimeout { get; set; } = TimeSpan.FromSeconds(5);

    /// <summary>
    /// Validates the configuration. Secret values are never included in the
    /// exception messages.
    /// </summary>
    /// <exception cref="ArgumentException">A required string is missing or contains control characters.</exception>
    /// <exception cref="ArgumentOutOfRangeException">A bounded value is outside the documented range.</exception>
    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(HostName))
        {
            throw new ArgumentException("A RabbitMQ host name is required.", nameof(HostName));
        }

        ArgumentOutOfRangeException.ThrowIfLessThan(Port, 1, nameof(Port));
        ArgumentOutOfRangeException.ThrowIfGreaterThan(Port, 65535, nameof(Port));

        if (string.IsNullOrWhiteSpace(VirtualHost))
        {
            throw new ArgumentException("A RabbitMQ virtual host is required.", nameof(VirtualHost));
        }

        if (string.IsNullOrWhiteSpace(Exchange))
        {
            throw new ArgumentException("A RabbitMQ exchange is required; the application owns topology.", nameof(Exchange));
        }

        if (string.IsNullOrWhiteSpace(ExchangeType))
        {
            throw new ArgumentException("A RabbitMQ exchange type is required.", nameof(ExchangeType));
        }

        if (HasControlCharacters(HostName) || HasControlCharacters(VirtualHost) || HasControlCharacters(UserName)
            || HasControlCharacters(Exchange) || HasControlCharacters(ExchangeType))
        {
            throw new ArgumentException("RabbitMQ connection values must not contain control characters.");
        }

        if (ConnectTimeout < TimeSpan.FromSeconds(1) || ConnectTimeout > TimeSpan.FromMinutes(5))
        {
            throw new ArgumentOutOfRangeException(nameof(ConnectTimeout), ConnectTimeout, "The RabbitMQ connect timeout must be between 1 second and 5 minutes.");
        }

        if (ConfirmTimeout < TimeSpan.FromMilliseconds(100) || ConfirmTimeout > TimeSpan.FromMinutes(5))
        {
            throw new ArgumentOutOfRangeException(nameof(ConfirmTimeout), ConfirmTimeout, "The RabbitMQ confirm timeout must be between 100 milliseconds and 5 minutes.");
        }
    }

    private static bool HasControlCharacters(string value) => value.Any(char.IsControl);
}
