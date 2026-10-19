using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Platform.Observability.Diagnostics;
using Platform.Observability.Redaction;

namespace Platform.Observability.Hosting;

/// <summary>Enriches incoming requests with a bounded correlation identifier and a request activity.</summary>
/// <remarks>The middleware never logs the raw header value verbatim; when <c>AcceptIncomingCorrelationHeader</c> is enabled the supplied value is bounded and validated before being attached to the request scope.</remarks>
public sealed class PlatformObservabilityCorrelationMiddleware
{
    private readonly RequestDelegate _next;

    /// <summary>Initializes a new instance of the <see cref="PlatformObservabilityCorrelationMiddleware"/> class.</summary>
    public PlatformObservabilityCorrelationMiddleware(RequestDelegate next)
    {
        ArgumentNullException.ThrowIfNull(next);
        _next = next;
    }

    /// <summary>Processes the request, attaches the correlation identifier, and emits the request activity.</summary>
    public async Task InvokeAsync(HttpContext context, IOptions<PlatformObservabilityOptions> options, IPlatformActivityRecorder recorder, IPlatformObservabilityRedactor redactor, ICorrelationIdGenerator generator)
    {
        ArgumentNullException.ThrowIfNull(context);
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(recorder);
        ArgumentNullException.ThrowIfNull(redactor);
        ArgumentNullException.ThrowIfNull(generator);

        var opts = options.Value;
        var header = opts.CorrelationHeader;
        var supplied = context.Request.Headers[header].ToString();
        string correlationId;
        if (!string.IsNullOrEmpty(supplied) && opts.AcceptIncomingCorrelationHeader)
        {
            correlationId = generator.Normalize(supplied, opts.MaxCorrelationIdLength);
        }
        else
        {
            correlationId = generator.Generate();
        }

        context.Items[HttpPlatformCorrelationAccessor.HttpContextItemKey] = correlationId;
        context.Response.OnStarting(() =>
        {
            if (!context.Response.Headers.ContainsKey(header))
                context.Response.Headers[header] = correlationId;
            return Task.CompletedTask;
        });

        var policy = new PlatformObservabilitySafeValuePolicy(redactor, opts.MaxTagLength, opts.MaxOperationLength, opts.TruncateOversizedValues);
        var route = policy.BoundTag(context.Request.Path.HasValue ? context.Request.Path.Value! : "/");
        var method = policy.BoundTag(context.Request.Method);
        using var activity = recorder.StartRequestCorrelation(PlatformObservabilityNames.RequestCorrelationOperation, route, method);
        if (activity is not null)
            activity.SetTag(PlatformObservabilityNames.TagCorrelationId, policy.RequireCorrelationId(correlationId));

        await _next(context);

        recorder.Complete(activity, PlatformObservabilityNames.OutcomeSuccess);
    }
}

/// <summary>Generates and normalizes correlation identifiers without exposing platform internals.</summary>
public interface ICorrelationIdGenerator
{
    /// <summary>Returns a fresh correlation identifier.</summary>
    string Generate();

    /// <summary>Normalizes and bounds a supplied identifier to the configured maximum length.</summary>
    string Normalize(string value, int maxLength);
}

/// <summary>Default <see cref="ICorrelationIdGenerator"/> that emits <see cref="Guid.NewGuid"/> values formatted with the <c>N</c> specifier.</summary>
public sealed class DefaultCorrelationIdGenerator : ICorrelationIdGenerator
{
    /// <inheritdoc />
    public string Generate() => Guid.NewGuid().ToString("N");

    /// <inheritdoc />
    public string Normalize(string value, int maxLength)
    {
        if (string.IsNullOrWhiteSpace(value)) return Generate();
        var trimmed = value.Trim();
        if (maxLength > 0 && trimmed.Length > maxLength) trimmed = trimmed[..maxLength];
        return trimmed;
    }
}
