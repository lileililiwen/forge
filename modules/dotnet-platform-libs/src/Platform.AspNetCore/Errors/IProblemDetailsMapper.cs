using Platform.Core.Results;

namespace Platform.AspNetCore.Errors;

#pragma warning disable CA1716 // Intentional: "Error" is the conventional parameter name for the platform error contract.

/// <summary>
/// Maps a <see cref="Error"/> to an ASP.NET Core
/// <see cref="Microsoft.AspNetCore.Mvc.ProblemDetails"/> response.
/// </summary>
public interface IProblemDetailsMapper
{
    /// <summary>
    /// Builds a <see cref="Microsoft.AspNetCore.Mvc.ProblemDetails"/>
    /// for the supplied <paramref name="error"/>.
    /// </summary>
    /// <param name="error">The platform error to translate.</param>
    /// <returns>The mapped <see cref="Microsoft.AspNetCore.Mvc.ProblemDetails"/>.</returns>
    /// <exception cref="System.ArgumentNullException"><paramref name="error"/> is <c>null</c>.</exception>
    Microsoft.AspNetCore.Mvc.ProblemDetails Map(Error error);

    /// <summary>
    /// Returns the HTTP status code associated with the supplied
    /// <paramref name="error"/>.
    /// </summary>
    /// <param name="error">The platform error to inspect.</param>
    /// <returns>The HTTP status code.</returns>
    /// <exception cref="System.ArgumentNullException"><paramref name="error"/> is <c>null</c>.</exception>
    int StatusCodeFor(Error error);
}

#pragma warning restore CA1716
