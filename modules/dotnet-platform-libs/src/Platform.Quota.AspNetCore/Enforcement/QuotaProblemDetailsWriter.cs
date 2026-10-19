using System.Diagnostics;
using System.Text.Json;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Platform.Core.Time;
using Platform.Quota.Contracts;

namespace Platform.Quota.AspNetCore.Enforcement;

/// <summary>Writes safe RFC 9457 problem details for quota outcomes.</summary>
public sealed class QuotaProblemDetailsWriter
{
    private readonly IClock _clock;

    /// <summary>Creates the writer.</summary>
    public QuotaProblemDetailsWriter(IClock clock)
    {
        _clock = clock ?? throw new ArgumentNullException(nameof(clock));
    }

    /// <summary>Writes a quota-exceeded problem details response for a denied decision.</summary>
    public async Task WriteExceededAsync(HttpContext context, QuotaDecision decision, int statusCode, string type, string title, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(context);
        ArgumentNullException.ThrowIfNull(decision);

        var retryAfter = ComputeRetryAfter(decision.Window.EndsAt);

        var problem = new ProblemDetails
        {
            Status = statusCode,
            Title = title,
            Type = type,
            Detail = "The request exceeded the allocated quota for the requested resource.",
            Instance = context.Request.Path.ToString()
        };

        problem.Extensions["resource"] = decision.Resource.Value;
        problem.Extensions["limit"] = decision.Limit;
        problem.Extensions["usage"] = decision.Consumed + decision.Reserved;
        problem.Extensions["requested"] = decision.Requested;
        if (retryAfter is not null)
        {
            problem.Extensions["retryAfterSeconds"] = retryAfter.Value;
            context.Response.Headers.RetryAfter = retryAfter.Value.ToString(System.Globalization.CultureInfo.InvariantCulture);
        }
        if (decision.Window.EndsAt.Offset == TimeSpan.Zero)
            problem.Extensions["resetAtUtc"] = decision.Window.EndsAt.ToString("o", System.Globalization.CultureInfo.InvariantCulture);

        AddTraceContext(context, problem);

        context.Response.StatusCode = statusCode;
        await WriteAsync(context, problem, cancellationToken).ConfigureAwait(false);
    }

    /// <summary>Writes a generic quota policy problem details response (missing context or unavailable provider).</summary>
    public static async Task WritePolicyAsync(HttpContext context, int statusCode, string type, string title, string detail, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(context);

        var problem = new ProblemDetails
        {
            Status = statusCode,
            Title = title,
            Type = type,
            Detail = detail,
            Instance = context.Request.Path.ToString()
        };

        AddTraceContext(context, problem);

        context.Response.StatusCode = statusCode;
        await WriteAsync(context, problem, cancellationToken).ConfigureAwait(false);
    }

    private static void AddTraceContext(HttpContext context, ProblemDetails problem)
    {
        var traceId = Activity.Current?.TraceId.ToString() ?? context.TraceIdentifier;
        problem.Extensions["traceId"] = traceId;

        var correlationId = context.Request.Headers["X-Correlation-ID"].FirstOrDefault()
            ?? context.TraceIdentifier;
        problem.Extensions["correlationId"] = correlationId;
    }

    private int? ComputeRetryAfter(DateTimeOffset resetAtUtc)
    {
        if (resetAtUtc.Offset != TimeSpan.Zero) return null;
        var delta = resetAtUtc - _clock.UtcNow;
        return delta.TotalSeconds > 0 ? (int)Math.Ceiling(delta.TotalSeconds) : null;
    }

    private static async Task WriteAsync(HttpContext context, ProblemDetails problem, CancellationToken cancellationToken)
    {
        context.Response.ContentType = "application/problem+json";
        await JsonSerializer.SerializeAsync(context.Response.Body, problem, _webSerializerOptions, cancellationToken).ConfigureAwait(false);
    }

    private static readonly JsonSerializerOptions _webSerializerOptions = new(JsonSerializerDefaults.Web);
}
