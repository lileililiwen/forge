using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;

namespace Platform.Web;

internal sealed class PlatformSecurityHeadersMiddleware
{
    private readonly RequestDelegate _next;
    public PlatformSecurityHeadersMiddleware(RequestDelegate next) => _next = next;

    public async Task InvokeAsync(HttpContext context, IOptions<PlatformWebOptions> options)
    {
        if (options.Value.EnableSecurityHeaders)
        {
            context.Response.Headers["X-Content-Type-Options"] = "nosniff";
            context.Response.Headers["X-Frame-Options"] = "DENY";
            context.Response.Headers["Referrer-Policy"] = "no-referrer";
            context.Response.Headers["Content-Security-Policy"] = "default-src 'none'; frame-ancestors 'none'";
        }

        await _next(context);
    }
}

internal sealed class PlatformRequestLimitsMiddleware
{
    private readonly RequestDelegate _next;
    public PlatformRequestLimitsMiddleware(RequestDelegate next) => _next = next;

    public async Task InvokeAsync(HttpContext context, IOptions<PlatformWebOptions> options)
    {
        var limit = options.Value.MaxRequestBodyBytes;
        if (context.Request.ContentLength > limit)
        {
            context.Response.StatusCode = StatusCodes.Status413PayloadTooLarge;
            await context.Response.WriteAsync("Request body exceeds the configured limit.", context.RequestAborted);
            return;
        }

        await _next(context);
    }
}

internal sealed class PlatformRequestTimeoutMiddleware
{
    private readonly RequestDelegate _next;
    public PlatformRequestTimeoutMiddleware(RequestDelegate next) => _next = next;

    public async Task InvokeAsync(HttpContext context, IOptions<PlatformWebOptions> options)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(context.RequestAborted);
        timeout.CancelAfter(options.Value.RequestTimeout);
        context.RequestAborted = timeout.Token;
        await _next(context);
    }
}
