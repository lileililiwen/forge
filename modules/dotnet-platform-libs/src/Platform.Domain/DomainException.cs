using Platform.Core.Results;

#pragma warning disable CA1716 // Intentional: "Error" matches the Platform.Core.Results.Error surface carried by this exception.

namespace Platform.Domain;

/// <summary>
/// Provider-neutral domain exception that carries a stable
/// <see cref="Error"/>. Carries no HTTP or provider-specific status code;
/// web boundaries translate <see cref="Error"/> through their own mapper.
/// The exception message is always the safe <see cref="Error.Message"/>.
/// </summary>
public class DomainException : Exception
{
    /// <summary>
    /// Initializes a new instance of the <see cref="DomainException"/> class.
    /// </summary>
    /// <param name="error">The stable, caller-safe error.</param>
    public DomainException(Error error)
        : base(GetMessage(error))
    {
        Error = error;
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="DomainException"/> class.
    /// </summary>
    /// <param name="error">The stable, caller-safe error.</param>
    /// <param name="innerException">The exception that caused this failure. Never surfaced to callers; inspect it only in logs.</param>
    public DomainException(Error error, Exception innerException)
        : base(GetMessage(error, innerException), innerException)
    {
        Error = error;
    }

    /// <summary>
    /// Gets the stable, caller-safe error carried by this exception.
    /// </summary>
    public Error Error { get; }

    private static string GetMessage(Error error)
    {
        ArgumentNullException.ThrowIfNull(error);
        return error.Message;
    }

    private static string GetMessage(Error error, Exception innerException)
    {
        ArgumentNullException.ThrowIfNull(error);
        ArgumentNullException.ThrowIfNull(innerException);
        return error.Message;
    }
}

/// <summary>
/// Domain exception for validation failures. Carries
/// <see cref="Error.ValidationCode"/> and maps to a client error at the
/// application-owned web boundary.
/// </summary>
public sealed class DomainValidationException : DomainException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="DomainValidationException"/> class.
    /// </summary>
    /// <param name="message">The safe validation message.</param>
    public DomainValidationException(string message)
        : base(Error.Validation(message))
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="DomainValidationException"/> class.
    /// </summary>
    /// <param name="error">The stable validation error.</param>
    public DomainValidationException(Error error)
        : base(error)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="DomainValidationException"/> class.
    /// </summary>
    /// <param name="error">The stable validation error.</param>
    /// <param name="innerException">The exception that caused this failure.</param>
    public DomainValidationException(Error error, Exception innerException)
        : base(error, innerException)
    {
    }
}

/// <summary>
/// Domain exception for missing domain objects. Carries
/// <see cref="Error.NotFoundCode"/> and maps to a not-found outcome at the
/// application-owned web boundary.
/// </summary>
public sealed class DomainNotFoundException : DomainException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="DomainNotFoundException"/> class.
    /// </summary>
    /// <param name="message">The safe not-found message.</param>
    public DomainNotFoundException(string message)
        : base(Error.NotFound(message))
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="DomainNotFoundException"/> class.
    /// </summary>
    /// <param name="error">The stable not-found error.</param>
    public DomainNotFoundException(Error error)
        : base(error)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="DomainNotFoundException"/> class.
    /// </summary>
    /// <param name="error">The stable not-found error.</param>
    /// <param name="innerException">The exception that caused this failure.</param>
    public DomainNotFoundException(Error error, Exception innerException)
        : base(error, innerException)
    {
    }
}

/// <summary>
/// Domain exception for operations that conflict with current state.
/// Carries <see cref="DomainErrorCodes.Conflict"/>.
/// </summary>
public sealed class DomainConflictException : DomainException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="DomainConflictException"/> class.
    /// </summary>
    /// <param name="message">The safe conflict message.</param>
    public DomainConflictException(string message)
        : base(new Error(DomainErrorCodes.Conflict, message))
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="DomainConflictException"/> class.
    /// </summary>
    /// <param name="error">The stable conflict error.</param>
    public DomainConflictException(Error error)
        : base(error)
    {
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="DomainConflictException"/> class.
    /// </summary>
    /// <param name="error">The stable conflict error.</param>
    /// <param name="innerException">The exception that caused this failure.</param>
    public DomainConflictException(Error error, Exception innerException)
        : base(error, innerException)
    {
    }
}
