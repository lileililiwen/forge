namespace Platform.Eventing.Contracts;

/// <summary>Safe failure information for a durable dispatch attempt.</summary>
public sealed record DurableDispatchFailure
{
    /// <summary>Creates a failure with a stable code and safe message.</summary>
    public DurableDispatchFailure(string code, string message, bool permanent = false)
    {
        if (string.IsNullOrWhiteSpace(code)) throw new ArgumentException("Failure code is required.", nameof(code));
        if (string.IsNullOrWhiteSpace(message)) throw new ArgumentException("Failure message is required.", nameof(message));
        if (code.Any(char.IsControl) || message.Any(char.IsControl))
            throw new ArgumentException("Failure values must not contain control characters.");
        Code = code;
        Message = message;
        Permanent = permanent;
    }

    /// <summary>Gets the safe failure code.</summary>
    public string Code { get; }

    /// <summary>Gets the safe failure message.</summary>
    public string Message { get; }

    /// <summary>Gets whether the failure must not be retried.</summary>
    public bool Permanent { get; }
}
