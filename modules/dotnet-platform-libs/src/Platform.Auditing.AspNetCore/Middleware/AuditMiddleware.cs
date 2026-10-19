using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Auditing.AspNetCore.Capture;
using Platform.Auditing.AspNetCore.Common;
using Platform.Auditing.Contracts;
using Platform.Core.Time;

namespace Platform.Auditing.AspNetCore.Middleware;

/// <summary>
/// Captures an HTTP request lifecycle as audit events. It records a normalized request event after
/// the pipeline completes, a security event for denied status codes, and a normalized exception
/// event when the pipeline throws. Capture is fail-open: audit failures never abort the request.
/// Raw request bodies are never captured except for an opt-in, bounded preview.
/// </summary>
public sealed class AuditMiddleware : IMiddleware
{
    private readonly IAuditRecorder _recorder;
    private readonly IClock _clock;
    private readonly IAuditSubjectResolver _subjectResolver;
    private readonly IOptions<AuditAspNetCoreOptions> _options;
    private readonly ILogger<AuditMiddleware>? _logger;

    /// <summary>Initializes a new middleware with the supplied dependencies.</summary>
    public AuditMiddleware(
        IAuditRecorder recorder,
        IClock clock,
        IAuditSubjectResolver subjectResolver,
        IOptions<AuditAspNetCoreOptions> options,
        ILogger<AuditMiddleware>? logger = null)
    {
        ArgumentNullException.ThrowIfNull(recorder);
        ArgumentNullException.ThrowIfNull(clock);
        ArgumentNullException.ThrowIfNull(subjectResolver);
        ArgumentNullException.ThrowIfNull(options);
        _recorder = recorder;
        _clock = clock;
        _subjectResolver = subjectResolver;
        _options = options;
        _logger = logger;
    }

    /// <inheritdoc />
    public async Task InvokeAsync(HttpContext context, RequestDelegate next)
    {
        ArgumentNullException.ThrowIfNull(context);
        ArgumentNullException.ThrowIfNull(next);
        var options = _options.Value;
        if (!options.Enabled || options.IsExempt(context.Request.Path.Value ?? string.Empty))
        {
            await next(context).ConfigureAwait(false);
            return;
        }

        var startedAt = _clock.UtcNow;
        var subject = _subjectResolver.Resolve(context);
        var correlationId = ResolveCorrelation(context, options);
        var bodyPreview = await ReadBodyPreviewAsync(context, options).ConfigureAwait(false);

        Exception? thrown = null;
        try
        {
            await next(context).ConfigureAwait(false);
        }
        catch (Exception exception)
        {
            thrown = exception;
            await RecordExceptionAsync(context, exception, subject, correlationId, startedAt).ConfigureAwait(false);
            throw;
        }
        finally
        {
            await RecordRequestAsync(context, options, subject, correlationId, startedAt, bodyPreview, thrown).ConfigureAwait(false);
        }
    }

    private async Task RecordRequestAsync(
        HttpContext context,
        AuditAspNetCoreOptions options,
        AuditSubjectResolution subject,
        string? correlationId,
        DateTimeOffset startedAt,
        string? bodyPreview,
        Exception? error = null)
    {
        var status = context.Response.StatusCode;
        var (outcome, severity) = error is not null ? (AuditOutcome.Error, AuditSeverity.Error) : ClassifyStatus(status);
        var duration = _clock.UtcNow - startedAt;
        var metadata = new Dictionary<string, string>
        {
            ["http.method"] = context.Request.Method,
            ["http.path"] = context.Request.Path.Value ?? string.Empty,
            ["http.status_code"] = status.ToString(System.Globalization.CultureInfo.InvariantCulture),
            ["http.duration_ms"] = duration.TotalMilliseconds.ToString("F1", System.Globalization.CultureInfo.InvariantCulture),
        };
        if (bodyPreview is not null) metadata["request.body_preview"] = bodyPreview;

        var auditEvent = AuditEvent.Create("http.request", "http", outcome, startedAt)
            .WithSeverity(severity)
            .WithCorrelation(correlationId)
            .WithActor(subject.SubjectId, subject.TenantId)
            .WithMetadata(metadata);

        await _recorder.RecordAsync(auditEvent, context.RequestAborted).ConfigureAwait(false);

        if (options.IsSecurityStatus(status))
        {
            var securityEvent = AuditEvent.Create("authorization.denied", "security", AuditOutcome.Denied, startedAt)
                .WithSeverity(AuditSeverity.Warning)
                .WithCorrelation(correlationId)
                .WithActor(subject.SubjectId, subject.TenantId)
                .WithMetadata(new Dictionary<string, string>
                {
                    ["http.method"] = context.Request.Method,
                    ["http.path"] = context.Request.Path.Value ?? string.Empty,
                    ["http.status_code"] = status.ToString(System.Globalization.CultureInfo.InvariantCulture),
                });
            await _recorder.RecordAsync(securityEvent, context.RequestAborted).ConfigureAwait(false);
        }
    }

    private async Task RecordExceptionAsync(
        HttpContext context,
        Exception exception,
        AuditSubjectResolution subject,
        string? correlationId,
        DateTimeOffset startedAt)
    {
        var classification = AuditExceptionClassifier.Classify(exception);
        var auditEvent = AuditEvent.Create("http.exception", "exception", AuditOutcome.Error, startedAt)
            .WithSeverity(AuditSeverity.Error)
            .WithCorrelation(correlationId)
            .WithActor(subject.SubjectId, subject.TenantId)
            .WithMetadata(new Dictionary<string, string>
            {
                ["http.method"] = context.Request.Method,
                ["http.path"] = context.Request.Path.Value ?? string.Empty,
                ["exception.kind"] = classification.Kind,
            });

        try
        {
            await _recorder.RecordAsync(auditEvent, context.RequestAborted).ConfigureAwait(false);
        }
        catch (Exception recordEx)
        {
#pragma warning disable CA1848 // Logger source-generator delegates are not used in the capture adapter.
            _logger?.LogWarning(recordEx, "Failed to record audit exception event for {Kind}.", classification.Kind);
#pragma warning restore CA1848
        }
    }

    private static string? ResolveCorrelation(HttpContext context, AuditAspNetCoreOptions options)
    {
        if (context.Request.Headers.TryGetValue(options.CorrelationHeaderName, out var values))
        {
            var value = values.ToString();
            if (!string.IsNullOrWhiteSpace(value)) return value;
        }

        return context.TraceIdentifier;
    }

    private static (AuditOutcome Outcome, AuditSeverity Severity) ClassifyStatus(int status) => status switch
    {
        401 or 403 => (AuditOutcome.Denied, AuditSeverity.Warning),
        >= 200 and < 400 => (AuditOutcome.Success, AuditSeverity.Information),
        >= 400 and < 500 => (AuditOutcome.Failure, AuditSeverity.Warning),
        >= 500 => (AuditOutcome.Error, AuditSeverity.Error),
        _ => (AuditOutcome.Unknown, AuditSeverity.Information),
    };

    private static async Task<string?> ReadBodyPreviewAsync(HttpContext context, AuditAspNetCoreOptions options)
    {
        if (!options.CaptureRequestBodyPreview) return null;
        if (context.Request.ContentLength is null or <= 0) return null;
        if (context.Request.ContentLength > options.MaxBodyPreviewBytes) return null;
        var contentType = context.Request.ContentType;
        if (contentType is null
            || !(contentType.Contains("json", StringComparison.OrdinalIgnoreCase)
                 || contentType.Contains("text", StringComparison.OrdinalIgnoreCase)
                 || contentType.Contains("xml", StringComparison.OrdinalIgnoreCase)))
        {
            return null;
        }

        context.Request.EnableBuffering();
        try
        {
            using var reader = new StreamReader(context.Request.Body, leaveOpen: true);
            var preview = await reader.ReadToEndAsync().ConfigureAwait(false);
            return preview.Length > options.BodyPreviewLimit
                ? preview.Substring(0, options.BodyPreviewLimit)
                : preview;
        }
        finally
        {
            if (context.Request.Body.CanSeek) context.Request.Body.Position = 0;
        }
    }
}
