namespace Platform.Eventing.RabbitMq;

/// <summary>Safe failure information for a RabbitMQ publish attempt.</summary>
/// <param name="Code">The stable failure code.</param>
/// <param name="Message">The fixed safe message; never carries broker response text.</param>
/// <param name="Permanent">Whether retrying the same publish cannot succeed.</param>
public sealed record RabbitMqPublishFailure(string Code, string Message, bool Permanent)
{
    /// <summary>Creates a transient failure that is eligible for durable retry.</summary>
    /// <param name="code">The stable failure code.</param>
    /// <param name="message">The fixed safe message.</param>
    /// <returns>The transient failure.</returns>
    public static RabbitMqPublishFailure Transient(string code, string message) => new(code, message, Permanent: false);

    /// <summary>Creates a configuration failure that must not be retried.</summary>
    /// <param name="code">The stable failure code.</param>
    /// <param name="message">The fixed safe message.</param>
    /// <returns>The permanent failure.</returns>
    public static RabbitMqPublishFailure Configuration(string code, string message) => new(code, message, Permanent: true);
}

/// <summary>
/// The only exception the RabbitMQ publisher surfaces. The message is fixed
/// and safe; broker response bodies, credentials, and inner exception text are
/// never attached.
/// </summary>
public sealed class RabbitMqPublishException : Exception
{
    /// <summary>Creates the exception from a safe failure.</summary>
    /// <param name="failure">The safe failure classification.</param>
    public RabbitMqPublishException(RabbitMqPublishFailure failure)
        : base(failure.Message)
    {
        Failure = failure;
    }

    /// <summary>Gets the safe failure classification.</summary>
    public RabbitMqPublishFailure Failure { get; }
}
