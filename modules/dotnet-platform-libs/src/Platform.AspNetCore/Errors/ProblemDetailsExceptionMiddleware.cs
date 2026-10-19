using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using Platform.Core.Results;

namespace Platform.AspNetCore.Errors;

#pragma warning disable CA1848 // Acceptable here: the call sites are invoked on the error path and source-generated delegates would not add clarity.

/// <summary>
/// Middleware that converts <see cref="PlatformProblemException"/> into
/// a sanitized <see cref="ProblemDetails"/> response. Unknown
/// exceptions are logged at error level and result in a generic
/// <c>500</c> response with no internal details exposed to the
/// client.
/// </summary>
public sealed class ProblemDetailsExceptionMiddleware
{
    private readonly RequestDelegate _next;
    private readonly IProblemDetailsMapper _mapper;
    private readonly ILogger<ProblemDetailsExceptionMiddleware> _logger;

    /// <summary>
    /// Initializes a new instance of the
    /// <see cref="ProblemDetailsExceptionMiddleware"/> class.
    /// </summary>
    /// <param name="next">The next delegate in the pipeline.</param>
    /// <param name="mapper">The platform problem details mapper.</param>
    /// <param name="logger">The logger used for diagnostic output.</param>
    public ProblemDetailsExceptionMiddleware(
        RequestDelegate next,
        IProblemDetailsMapper mapper,
        ILogger<ProblemDetailsExceptionMiddleware> logger)
    {
        _next = next;
        _mapper = mapper;
        _logger = logger;
    }

    /// <summary>
    /// Invokes the middleware for the current request.
    /// </summary>
    /// <param name="context">The current HTTP context.</param>
    public async Task InvokeAsync(HttpContext context)
    {
        try
        {
            await _next(context);
        }
        catch (PlatformProblemException ex)
        {
            _logger.LogWarning(ex, "Platform failure {Code} returned to the client.", ex.Error.Code);
            await WriteProblemAsync(context, _mapper.Map(ex.Error));
        }
        catch (Exception ex) when (!context.RequestAborted.IsCancellationRequested)
        {
            _logger.LogError(ex, "Unhandled exception escaped the request pipeline.");
            await WriteProblemAsync(context, UnknownProblem(context));
        }
    }

    private static async Task WriteProblemAsync(HttpContext context, ProblemDetails problem)
    {
        if (context.Response.HasStarted)
        {
            return;
        }

        context.Response.StatusCode = problem.Status ?? StatusCodes.Status500InternalServerError;
        context.Response.ContentType = "application/problem+json";
        await System.Text.Json.JsonSerializer.SerializeAsync(
            context.Response.Body,
            problem,
            options: null,
            cancellationToken: context.RequestAborted);
    }

    private static ProblemDetails UnknownProblem(HttpContext context) => new()
    {
        Type = "https://platform.example/problems/internal",
        Title = "internal",
        Status = StatusCodes.Status500InternalServerError,
        Detail = "An unexpected error occurred. The error has been logged.",
        Instance = context.TraceIdentifier,
    };
}

#pragma warning restore CA1848
