using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Platform.Web.Telemetry;

/// <summary>Default <see cref="IPlatformWebTelemetry"/> that emits structured log records via <see cref="ILogger"/>.</summary>
public sealed class DefaultPlatformWebTelemetry : IPlatformWebTelemetry
{
    private static readonly Action<ILogger, string, string, int, long, Exception?> LogRequestSucceeded =
        LoggerMessage.Define<string, string, int, long>(LogLevel.Information, new EventId(1, "RequestSucceeded"), "web.request {Method} {Route} -> {Status} in {Duration}ms");

    private static readonly Action<ILogger, string, string, int, long, string, Exception?> LogRequestFailed =
        LoggerMessage.Define<string, string, int, long, string>(LogLevel.Warning, new EventId(2, "RequestFailed"), "web.request {Method} {Route} -> {Status} in {Duration}ms ({ErrorCode})");

    private static readonly Action<ILogger, string, string, int, long, int, Exception?> LogProviderSucceeded =
        LoggerMessage.Define<string, string, int, long, int>(LogLevel.Information, new EventId(3, "ProviderSucceeded"), "web.provider {Operation} {Provider} -> {Status} in {Duration}ms attempt {Attempt}");

    private static readonly Action<ILogger, string, string, int, long, int, string, Exception?> LogProviderFailed =
        LoggerMessage.Define<string, string, int, long, int, string>(LogLevel.Warning, new EventId(4, "ProviderFailed"), "web.provider {Operation} {Provider} -> {Status} in {Duration}ms attempt {Attempt} ({ErrorCode})");

    private readonly ILogger<DefaultPlatformWebTelemetry> _logger;
    private readonly PlatformWebTelemetrySafeValuePolicy _policy;
    private readonly PlatformWebTelemetryOptions _options;

    /// <summary>Initializes a new instance of the <see cref="DefaultPlatformWebTelemetry"/> class.</summary>
    public DefaultPlatformWebTelemetry(IOptions<PlatformWebTelemetryOptions> options, ILogger<DefaultPlatformWebTelemetry> logger, IPlatformWebTelemetryRedactor redactor)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(logger);
        ArgumentNullException.ThrowIfNull(redactor);
        _options = options.Value;
        _logger = logger;
        _policy = new PlatformWebTelemetrySafeValuePolicy(redactor, _options.MaxTagLength, _options.MaxOperationLength, _options.TruncateOversizedValues);
    }

    /// <inheritdoc />
    public PlatformWebTelemetrySafeValuePolicy SafeValuePolicy => _policy;

    /// <inheritdoc />
    public void RecordRequest(PlatformWebRequestEvent request)
    {
        ArgumentNullException.ThrowIfNull(request);
        using var scope = _logger.BeginScope(BuildRequestScope(request));
        var method = _policy.RedactTag(request.Method);
        var route = _policy.RedactTag(request.Route);
        var status = request.Status;
        var duration = (long)request.Duration.TotalMilliseconds;
        if (request.ErrorCode is null)
            LogRequestSucceeded(_logger, method, route, status, duration, null);
        else
            LogRequestFailed(_logger, method, route, status, duration, _policy.RedactTag(request.ErrorCode), null);
    }

    /// <inheritdoc />
    public void RecordProvider(PlatformWebProviderEvent provider)
    {
        ArgumentNullException.ThrowIfNull(provider);
        using var scope = _logger.BeginScope(BuildProviderScope(provider));
        var operation = _policy.RequireOperation(provider.Operation);
        var providerName = _policy.RedactTag(provider.Provider);
        var status = provider.Status;
        var duration = (long)provider.Duration.TotalMilliseconds;
        var attempt = provider.Attempt;
        if (provider.ErrorCode is null)
            LogProviderSucceeded(_logger, operation, providerName, status, duration, attempt, null);
        else
            LogProviderFailed(_logger, operation, providerName, status, duration, attempt, _policy.RedactTag(provider.ErrorCode), null);
    }

    private Dictionary<string, object> BuildRequestScope(PlatformWebRequestEvent request) => new()
    {
        [PlatformWebTelemetryNames.TagOperation] = _policy.RequireOperation(request.Operation),
        [PlatformWebTelemetryNames.TagMethod] = _policy.RedactTag(request.Method),
        [PlatformWebTelemetryNames.TagRoute] = _policy.RedactTag(request.Route),
        [PlatformWebTelemetryNames.TagStatus] = request.Status,
        [PlatformWebTelemetryNames.TagErrorCode] = _policy.RedactTag(request.ErrorCode ?? string.Empty),
    };

    private Dictionary<string, object> BuildProviderScope(PlatformWebProviderEvent provider) => new()
    {
        [PlatformWebTelemetryNames.TagOperation] = _policy.RequireOperation(provider.Operation),
        [PlatformWebTelemetryNames.TagProvider] = _policy.RedactTag(provider.Provider),
        [PlatformWebTelemetryNames.TagStatus] = provider.Status,
        [PlatformWebTelemetryNames.TagAttempt] = provider.Attempt,
        [PlatformWebTelemetryNames.TagErrorCode] = _policy.RedactTag(provider.ErrorCode ?? string.Empty),
    };
}
