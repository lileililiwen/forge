using Platform.Core.Results;

namespace Platform.AspNetCore.Errors;

/// <summary>
/// Exception type that carries a <see cref="Error"/> from application
/// code to the platform error boundary. Throw this exception to signal
/// that a known platform failure should be translated to
/// <see cref="Microsoft.AspNetCore.Mvc.ProblemDetails"/>.
/// </summary>
public sealed class PlatformProblemException : Exception
{
    /// <summary>
    /// Initializes a new instance of the
    /// <see cref="PlatformProblemException"/> class with the supplied
    /// <paramref name="error"/>.
    /// </summary>
    /// <param name="error">The platform error to surface.</param>
    /// <exception cref="System.ArgumentNullException"><paramref name="error"/> is <c>null</c>.</exception>
    public PlatformProblemException(Error error)
        : base(error.Message)
    {
        ArgumentNullException.ThrowIfNull(error);
        Error = error;
    }

    /// <summary>
    /// Gets the platform error carried by this exception.
    /// </summary>
    public Error Error { get; }
}
