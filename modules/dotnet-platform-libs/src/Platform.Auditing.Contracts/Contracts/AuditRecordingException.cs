namespace Platform.Auditing.Contracts;

/// <summary>Thrown by the recorder when a sink fails under the fail-closed policy.</summary>
public sealed class AuditRecordingException : Exception
{
    /// <summary>Initializes a new instance with a message.</summary>
    /// <param name="message">The message.</param>
    public AuditRecordingException(string message) : base(message)
    {
    }

    /// <summary>Initializes a new instance with a message and inner exception.</summary>
    /// <param name="message">The message.</param>
    /// <param name="innerException">The underlying sink failure.</param>
    public AuditRecordingException(string message, Exception innerException) : base(message, innerException)
    {
    }
}
