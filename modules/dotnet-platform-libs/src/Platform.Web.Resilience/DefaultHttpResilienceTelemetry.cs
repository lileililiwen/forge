using System.Globalization;
using Microsoft.Extensions.Logging;
using Platform.Web.Telemetry;

namespace Platform.Web.Resilience;

/// <summary>Default <see cref="IHttpResilienceTelemetry"/> that bridges to the platform web telemetry sink.</summary>
public sealed class DefaultHttpResilienceTelemetry : IHttpResilienceTelemetry
{
    private static readonly Action<ILogger<DefaultHttpResilienceTelemetry>, string, string, int, int, Exception?> LogDecision =
        LoggerMessage.Define<string, string, int, int>(LogLevel.Debug, new EventId(1, "Decision"), "HTTP resilience: {Decision} for {Operation} attempt {Attempt} status {Status}");

    private readonly IPlatformWebTelemetry _telemetry;
    private readonly ILogger<DefaultHttpResilienceTelemetry> _logger;

    /// <summary>Initializes a new instance of the <see cref="DefaultHttpResilienceTelemetry"/> class.</summary>
    public DefaultHttpResilienceTelemetry(IPlatformWebTelemetry telemetry, ILogger<DefaultHttpResilienceTelemetry> logger)
    {
        _telemetry = telemetry ?? throw new ArgumentNullException(nameof(telemetry));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <inheritdoc />
    public void Record(HttpResilienceEvent resilienceEvent)
    {
        ArgumentNullException.ThrowIfNull(resilienceEvent);
        var errorCode = resilienceEvent.Decision switch
        {
            HttpResilienceDecision.Retried => "resilience.retry",
            HttpResilienceDecision.TimedOut => "resilience.timeout",
            HttpResilienceDecision.CircuitBroken => "resilience.circuit_open",
            _ => null,
        };
        if (resilienceEvent.Decision == HttpResilienceDecision.None) return;
        _telemetry.RecordProvider(new PlatformWebProviderEvent(
            Operation: resilienceEvent.Operation,
            Provider: "http",
            Status: resilienceEvent.StatusCode,
            Duration: TimeSpan.Zero,
            Attempt: resilienceEvent.Attempt,
            ErrorCode: errorCode));
        LogDecision(_logger, resilienceEvent.Decision.ToString(), resilienceEvent.Operation, resilienceEvent.Attempt, resilienceEvent.StatusCode, null);
    }
}
