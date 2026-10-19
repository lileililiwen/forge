using System.Diagnostics;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Observability.Diagnostics;

namespace Platform.Observability.Hosting;

/// <summary>Records host lifecycle activities, metrics, and structured log records for startup and shutdown.</summary>
public sealed class PlatformObservabilityHostLifetime : IHostedLifecycleService
{
    private static readonly Action<ILogger, string, Exception?> LogStartup =
        LoggerMessage.Define<string>(LogLevel.Information, new EventId(1, "HostStartup"), "platform.host.startup {ApplicationName}");

    private static readonly Action<ILogger, string, Exception?> LogShutdown =
        LoggerMessage.Define<string>(LogLevel.Information, new EventId(2, "HostShutdown"), "platform.host.shutdown {ApplicationName}");

    private readonly IPlatformActivityRecorder _recorder;
    private readonly ILogger<PlatformObservabilityHostLifetime> _logger;
    private readonly PlatformObservabilityOptions _options;

    /// <summary>Initializes a new instance of the <see cref="PlatformObservabilityHostLifetime"/> class.</summary>
    public PlatformObservabilityHostLifetime(IPlatformActivityRecorder recorder, IOptions<PlatformObservabilityOptions> options, ILogger<PlatformObservabilityHostLifetime> logger)
    {
        ArgumentNullException.ThrowIfNull(recorder);
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(logger);
        _recorder = recorder;
        _options = options.Value;
        _logger = logger;
    }

    /// <inheritdoc />
    public Task StartingAsync(CancellationToken cancellationToken)
    {
        var stopwatch = Stopwatch.StartNew();
        using var activity = _recorder.StartHostLifecycle(PlatformObservabilityNames.HostStartupOperation);
        try
        {
            using (_logger.BeginScope(BuildScope(PlatformObservabilityNames.HostStartupOperation)))
            {
                LogStartup(_logger, _options.ApplicationName, null);
            }
            PlatformDiagnostics.RecordLifecycle(PlatformObservabilityNames.OutcomeSuccess);
            return Task.CompletedTask;
        }
        finally
        {
            stopwatch.Stop();
            PlatformDiagnostics.SetLastStartupDuration(stopwatch.Elapsed.TotalMilliseconds);
            _recorder.Complete(activity, PlatformObservabilityNames.OutcomeSuccess, durationMs: stopwatch.Elapsed.TotalMilliseconds);
        }
    }

    /// <inheritdoc />
    public Task StartAsync(CancellationToken cancellationToken) => Task.CompletedTask;

    /// <inheritdoc />
    public Task StartedAsync(CancellationToken cancellationToken) => Task.CompletedTask;

    /// <inheritdoc />
    public Task StoppingAsync(CancellationToken cancellationToken) => Task.CompletedTask;

    /// <inheritdoc />
    public Task StopAsync(CancellationToken cancellationToken) => Task.CompletedTask;

    /// <inheritdoc />
    public Task StoppedAsync(CancellationToken cancellationToken)
    {
        var stopwatch = Stopwatch.StartNew();
        using var activity = _recorder.StartHostLifecycle(PlatformObservabilityNames.HostShutdownOperation);
        try
        {
            using (_logger.BeginScope(BuildScope(PlatformObservabilityNames.HostShutdownOperation)))
            {
                LogShutdown(_logger, _options.ApplicationName, null);
            }
            PlatformDiagnostics.RecordLifecycle(PlatformObservabilityNames.OutcomeSuccess);
            return Task.CompletedTask;
        }
        finally
        {
            stopwatch.Stop();
            PlatformDiagnostics.SetLastShutdownDuration(stopwatch.Elapsed.TotalMilliseconds);
            _recorder.Complete(activity, PlatformObservabilityNames.OutcomeSuccess, durationMs: stopwatch.Elapsed.TotalMilliseconds);
        }
    }

    private Dictionary<string, object> BuildScope(string operation) => new()
    {
        [PlatformObservabilityNames.TagServiceName] = _options.ApplicationName,
        [PlatformObservabilityNames.TagOperation] = operation,
    };
}
