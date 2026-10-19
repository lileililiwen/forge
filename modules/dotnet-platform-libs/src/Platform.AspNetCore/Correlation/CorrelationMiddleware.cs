using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;

namespace Platform.AspNetCore.Correlation;

/// <summary>
/// Middleware that establishes a correlation identifier for every
/// request. The identifier is read from a configured request header
/// (when the host opts in) or generated as a new
/// <see cref="Guid"/>. The identifier is stored on
/// <see cref="HttpContext.Items"/> for the duration of the request and
/// echoed on the response.
/// </summary>
public sealed class CorrelationMiddleware
{
    private readonly RequestDelegate _next;

    /// <summary>
    /// Initializes a new instance of the
    /// <see cref="CorrelationMiddleware"/> class.
    /// </summary>
    /// <param name="next">The next delegate in the pipeline.</param>
    public CorrelationMiddleware(RequestDelegate next)
    {
        _next = next;
    }

    /// <summary>
    /// Invokes the middleware for the current request.
    /// </summary>
    /// <param name="context">The current HTTP context.</param>
    /// <param name="options">The platform options.</param>
    public async Task InvokeAsync(HttpContext context, IOptions<PlatformAspNetCoreOptions> options)
    {
        ArgumentNullException.ThrowIfNull(options);

        var header = options.Value.CorrelationHeader;
        var acceptIncoming = options.Value.AcceptIncomingCorrelationHeader;
        var correlationId = ResolveCorrelationId(context, header, acceptIncoming, options.Value.MaxCorrelationIdLength);

        context.Items[HttpCorrelationAccessor.HttpContextItemsKey] = correlationId;
        context.Response.OnStarting(() =>
        {
            context.Response.Headers[header] = correlationId;
            return Task.CompletedTask;
        });

        await _next(context);
    }

    private static string ResolveCorrelationId(HttpContext context, string header, bool acceptIncoming, int maxLength)
    {
        if (acceptIncoming
            && context.Request.Headers.TryGetValue(header, out var values)
            && !string.IsNullOrWhiteSpace(values)
            && IsSafeCorrelationId(values.ToString(), maxLength))
        {
            return values.ToString();
        }

        return Guid.NewGuid().ToString("D");
    }

    private static bool IsSafeCorrelationId(string value, int maxLength)
    {
        if (value.Length > maxLength)
        {
            return false;
        }

        return value.All(character => character is >= '!' and <= '~');
    }
}
