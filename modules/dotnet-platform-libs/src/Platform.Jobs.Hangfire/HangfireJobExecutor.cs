using System.Reflection;
using System.Text.Json;
using Hangfire;
using Hangfire.Common;
using Hangfire.Server;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Results;
using Platform.Core.Time;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// The Hangfire-invoked entry point for platform jobs. Hangfire resolves
/// the executor from the job's DI scope (created by
/// <see cref="ScopedJobActivator"/>), so dispatched payloads run inside the
/// restored tenant/subject context. Failures are recorded through the
/// optional <see cref="IJobTelemetry"/> with redacted, stable error codes
/// and then rethrown so Hangfire's own retry model stays in charge.
/// </summary>
public sealed class HangfireJobExecutor
{
    /// <summary>Stable error code recorded when a dispatched or recurring job fails.</summary>
    public const string ExecutionFailedCode = "jobs.execution_failed";

    /// <summary>
    /// The JSON options used to serialize dispatched payloads. Argument
    /// values round-trip as JSON values; complex values arrive as
    /// <see cref="System.Text.Json.JsonElement"/>.
    /// </summary>
    public static readonly JsonSerializerOptions PayloadJsonOptions = new(JsonSerializerDefaults.Web);

    private readonly IServiceProvider _serviceProvider;
    private readonly IRecurringJobRegistry _registry;
    private readonly IClock _clock;
    private readonly IJobTelemetry? _telemetry;

    /// <summary>
    /// Initializes a new instance of the <see cref="HangfireJobExecutor"/>
    /// class. Resolved per job execution from the Hangfire DI scope.
    /// </summary>
    /// <param name="serviceProvider">The job execution scope's service provider.</param>
    /// <param name="registry">The recurring-job registry used to resolve recurring handlers.</param>
    /// <param name="clock">The platform clock.</param>
    /// <param name="telemetry">The optional platform job telemetry.</param>
    public HangfireJobExecutor(
        IServiceProvider serviceProvider,
        IRecurringJobRegistry registry,
        IClock clock,
        IJobTelemetry? telemetry = null)
    {
        ArgumentNullException.ThrowIfNull(serviceProvider);
        ArgumentNullException.ThrowIfNull(registry);
        ArgumentNullException.ThrowIfNull(clock);
        _serviceProvider = serviceProvider;
        _registry = registry;
        _clock = clock;
        _telemetry = telemetry;
    }

    /// <summary>
    /// Handles a dispatched <see cref="JobPayload"/>. Invoked by Hangfire;
    /// applications never call this method directly.
    /// </summary>
    /// <param name="payloadJson">The JSON-serialized payload created by the dispatcher.</param>
    /// <param name="cancellationToken">The Hangfire shutdown token, substituted by Hangfire.</param>
    /// <returns>A task that completes when the payload is handled.</returns>
    public async Task Execute(string payloadJson, IJobCancellationToken? cancellationToken)
    {
        var token = cancellationToken?.ShutdownToken ?? CancellationToken.None;
        var payload = DeserializePayload(payloadJson);
        var handler = ResolvePayloadHandler();

        try
        {
            await handler.HandleAsync(payload, token);
            _telemetry?.JobExecuted(payload.Name, _clock.UtcNow);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception ex)
        {
            _telemetry?.JobFailed(payload.Name, ToSafeError(ex), _clock.UtcNow);
            throw;
        }
    }

    /// <summary>
    /// Handles a recurring job by resolving its descriptor from the
    /// platform registry and invoking the registered handler. Invoked by
    /// Hangfire; applications never call this method directly.
    /// </summary>
    /// <param name="jobName">The stable recurring-job name.</param>
    /// <param name="cancellationToken">The Hangfire shutdown token, substituted by Hangfire.</param>
    /// <returns>A task that completes when the recurring job is handled.</returns>
    public async Task ExecuteRecurring(string jobName, IJobCancellationToken? cancellationToken)
    {
        var token = cancellationToken?.ShutdownToken ?? CancellationToken.None;
        var descriptor = _registry.Registered.FirstOrDefault(d => string.Equals(d.Name, jobName, StringComparison.Ordinal))
            ?? throw new InvalidOperationException(
                $"Recurring job '{jobName}' is not registered in the platform recurring-job registry. Register the descriptor on startup via IRecurringJobRegistry.");
        var handler = ResolveRecurringHandler(descriptor);

        try
        {
            await handler.ExecuteAsync(token);
            _telemetry?.JobExecuted(descriptor.Name, _clock.UtcNow);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception ex)
        {
            _telemetry?.JobFailed(descriptor.Name, ToSafeError(ex), _clock.UtcNow);
            throw;
        }
    }

    internal static Job CreateExecuteJob(string payloadJson) =>
        new(typeof(HangfireJobExecutor), ExecuteMethodInfo, new object?[] { payloadJson, null });

    internal static Job CreateExecuteRecurringJob(string jobName) =>
        new(typeof(HangfireJobExecutor), ExecuteRecurringMethodInfo, new object?[] { jobName, null });

    private static MethodInfo ExecuteMethodInfo { get; } = GetExecutorMethod(nameof(Execute));

    private static MethodInfo ExecuteRecurringMethodInfo { get; } = GetExecutorMethod(nameof(ExecuteRecurring));

    private static MethodInfo GetExecutorMethod(string name) =>
        typeof(HangfireJobExecutor).GetMethod(name, BindingFlags.Public | BindingFlags.Instance)
        ?? throw new InvalidOperationException($"The executor method '{name}' could not be resolved.");

    private static JobPayload DeserializePayload(string payloadJson)
    {
        if (string.IsNullOrWhiteSpace(payloadJson))
        {
            throw new InvalidOperationException("The dispatched job payload is empty.");
        }

        JobPayload? payload;
        try
        {
            payload = JsonSerializer.Deserialize<JobPayload>(payloadJson, PayloadJsonOptions);
        }
        catch (JsonException ex)
        {
            throw new InvalidOperationException("The dispatched job payload is not a valid platform job payload.", ex);
        }

        if (payload is null || string.IsNullOrWhiteSpace(payload.Name))
        {
            throw new InvalidOperationException("The dispatched job payload does not carry a valid job name.");
        }

        return payload with { Arguments = NormalizeArguments(payload.Arguments) };
    }

    private static IReadOnlyDictionary<string, object?>? NormalizeArguments(IReadOnlyDictionary<string, object?>? arguments)
    {
        if (arguments is null || arguments.Count == 0)
        {
            return arguments;
        }

        var normalized = new Dictionary<string, object?>(arguments.Count, StringComparer.Ordinal);
        foreach (var (key, value) in arguments)
        {
            normalized[key] = NormalizeValue(value);
        }

        return normalized;
    }

    private static object? NormalizeValue(object? value) => value switch
    {
        JsonElement element => ConvertElement(element),
        _ => value,
    };

    private static object? ConvertElement(JsonElement element)
    {
        if (element.ValueKind == JsonValueKind.Number && element.TryGetInt64(out var integer))
        {
            return integer;
        }

        return element.ValueKind switch
        {
            JsonValueKind.String => element.GetString(),
            JsonValueKind.Number => element.GetDouble(),
            JsonValueKind.True => true,
            JsonValueKind.False => false,
            JsonValueKind.Null => null,
            _ => element,
        };
    }

    private IJobPayloadHandler ResolvePayloadHandler() =>
        _serviceProvider.GetService(typeof(IJobPayloadHandler)) as IJobPayloadHandler
        ?? throw new InvalidOperationException(
            "No IJobPayloadHandler is registered. Register an application-owned IJobPayloadHandler to handle dispatched job payloads.");

    private IRecurringJobHandler ResolveRecurringHandler(RecurringJobDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor.HandlerType);
        var handler = _serviceProvider.GetService(descriptor.HandlerType) as IRecurringJobHandler
            ?? _serviceProvider.GetServices<IRecurringJobHandler>()
                .FirstOrDefault(candidate => candidate.GetType() == descriptor.HandlerType);
        return handler
            ?? throw new InvalidOperationException(
                $"The recurring job handler '{descriptor.HandlerType.FullName}' is not registered or does not implement IRecurringJobHandler.");
    }

    private static Error ToSafeError(Exception exception) => new(
        ExecutionFailedCode,
        "A background job failed.",
        new Dictionary<string, object?>
        {
            ["exceptionType"] = exception.GetType().Name,
        });
}
