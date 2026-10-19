#pragma warning disable CS1591, CA1848
using System.Diagnostics;
using Microsoft.Extensions.Logging;
using Platform.Ai.Contracts;

namespace Platform.Ai;

public sealed class SingleAiProviderRouter : IAiProviderRouter
{
    private readonly IAiTextProvider _provider;
    public SingleAiProviderRouter(IAiTextProvider provider) => _provider = provider ?? throw new ArgumentNullException(nameof(provider));
    public IAiTextProvider Resolve(AiFeatureKey feature) => _provider;
}

public sealed class FeatureAiProviderRouter : IAiProviderRouter
{
    private readonly Dictionary<string, IAiTextProvider> _providers;
    public FeatureAiProviderRouter(IReadOnlyDictionary<AiFeatureKey, IAiTextProvider> providers)
    {
        ArgumentNullException.ThrowIfNull(providers);
        _providers = providers.ToDictionary(pair => pair.Key.Value, pair => pair.Value, StringComparer.OrdinalIgnoreCase);
    }
    public IAiTextProvider Resolve(AiFeatureKey feature) => _providers.TryGetValue(feature.Value, out var provider)
        ? provider
        : throw new InvalidOperationException($"No AI provider is configured for feature '{feature.Value}'.");
}

public sealed class AiClient
{
    private readonly IAiProviderRouter _router;
    private readonly IAiFeaturePolicy _policy;
    private readonly IAiUsageSink? _usage;
    private readonly IAiTelemetry? _telemetry;
    private readonly ILogger<AiClient> _logger;

    public AiClient(IAiProviderRouter router, IAiFeaturePolicy policy, IAiUsageSink? usage = null, IAiTelemetry? telemetry = null, ILogger<AiClient>? logger = null)
    {
        _router = router ?? throw new ArgumentNullException(nameof(router));
        _policy = policy ?? throw new ArgumentNullException(nameof(policy));
        _usage = usage; _telemetry = telemetry; _logger = logger ?? Microsoft.Extensions.Logging.Abstractions.NullLogger<AiClient>.Instance;
    }

    public async ValueTask<AiGenerationResult> GenerateAsync(AiTextRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        var decision = await _policy.EvaluateAsync(request, cancellationToken).ConfigureAwait(false);
        if (!decision.Allowed) return new(null, AiUsage.Empty, Failure: decision.Failure ?? AiFailure.Safe(AiFailureCategory.QuotaExceeded, "AI request rejected by policy."));
        var provider = _router.Resolve(request.Feature);
        var timer = Stopwatch.StartNew();
        try
        {
            using var timeout = request.Timeout is { } value ? CancellationTokenSource.CreateLinkedTokenSource(cancellationToken) : null;
            if (timeout is not null) timeout.CancelAfter(request.Timeout!.Value);
            var result = await provider.GenerateAsync(request, timeout?.Token ?? cancellationToken).ConfigureAwait(false);
            timer.Stop();
            if (result.Succeeded) { if (_usage is not null) await _usage.RecordAsync(request.Feature, result.Usage, cancellationToken); _telemetry?.Completed(request.Feature, provider.Name, result.Model, timer.Elapsed, result.Usage); }
            else _telemetry?.Failed(request.Feature, provider.Name, result.Failure!, timer.Elapsed);
            if (_logger.IsEnabled(LogLevel.Debug))
            {
                _logger.LogDebug("AI request completed. Feature={Feature} Provider={Provider} Model={Model} InputTokens={InputTokens} OutputTokens={OutputTokens}", request.Feature.Value, provider.Name.Value, result.Model, result.Usage.InputTokens, result.Usage.OutputTokens);
            }
            return result;
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        { return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.Timeout, "The AI request timed out.")); }
        catch (Exception ex)
        {
            _logger.LogWarning("AI provider request failed. Feature={Feature} Provider={Provider} FailureType={FailureType}", request.Feature.Value, provider.Name.Value, ex.GetType().Name);
            return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.ProviderError, "The AI provider request failed."));
        }
    }

    public ValueTask<AiGenerationResult> GenerateStructuredAsync(AiStructuredRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        var provider = _router.Resolve(request.Text.Feature);
        if (!provider.Capabilities.Supports("structured-output"))
            return ValueTask.FromResult(new AiGenerationResult(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.UnsupportedCapability, "The selected AI provider does not support structured output.")));
        return GenerateAsync(request.Text with { Metadata = MergeSchema(request.Text.Metadata, request.SchemaJson) }, cancellationToken);
    }

    private static Dictionary<string, string> MergeSchema(IReadOnlyDictionary<string, string>? metadata, string schema) =>
        (metadata ?? new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)).Concat([new KeyValuePair<string, string>("ai.schema", schema)]).ToDictionary(pair => pair.Key, pair => pair.Value, StringComparer.OrdinalIgnoreCase);
}

public sealed class AllowAllAiFeaturePolicy : IAiFeaturePolicy
{
    public ValueTask<AiPolicyDecision> EvaluateAsync(AiTextRequest request, CancellationToken cancellationToken = default) => ValueTask.FromResult(AiPolicyDecision.Allow());
}
