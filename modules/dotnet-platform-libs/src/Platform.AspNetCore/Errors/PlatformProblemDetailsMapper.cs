using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Platform.Core.Results;

namespace Platform.AspNetCore.Errors;

/// <summary>
/// Default <see cref="IProblemDetailsMapper"/> implementation. Maps
/// known platform error codes to HTTP status codes and a stable
/// category URI; never includes stack traces or exception details.
/// </summary>
public sealed class PlatformProblemDetailsMapper : IProblemDetailsMapper
{
    private const string ProblemTypeBase = "https://platform.example/problems/";

    /// <inheritdoc />
    public ProblemDetails Map(Error error)
    {
        ArgumentNullException.ThrowIfNull(error);

        var statusCode = StatusCodeFor(error);
        var category = CategoryFor(error.Code);

        var problem = new ProblemDetails
        {
            Type = ProblemTypeBase + category,
            Title = category,
            Status = statusCode,
            Detail = error.Message,
        };

        problem.Extensions["code"] = error.Code;

        if (error.Metadata is { Count: > 0 })
        {
            foreach (var (key, value) in error.Metadata)
            {
                problem.Extensions[key] = value;
            }
        }

        return problem;
    }

    /// <inheritdoc />
    public int StatusCodeFor(Error error)
    {
        ArgumentNullException.ThrowIfNull(error);

        return error.Code switch
        {
            Error.ValidationCode => StatusCodes.Status400BadRequest,
            Error.NotFoundCode => StatusCodes.Status404NotFound,
            _ => StatusCodes.Status500InternalServerError,
        };
    }

    private static string CategoryFor(string code)
    {
        var separator = code.IndexOf('.');
        return separator < 0 ? code : code[(separator + 1)..];
    }
}
