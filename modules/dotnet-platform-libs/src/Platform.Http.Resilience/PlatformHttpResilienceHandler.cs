using System;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Options;
using Polly;
using Polly.CircuitBreaker;
using Polly.RateLimiting;

namespace Platform.Http.Resilience;

/// <summary>Outbound HTTP resilience handler that applies the platform pipeline (retry, timeout, circuit breaker, concurrency) and records bounded policy telemetry. The handler is opt-in and becomes a pass-through when disabled.</summary>
/// <remarks>Retry decisions are restricted to idempotent methods by stashing the request method on the <see cref="ResilienceContext"/>. Circuit-breaker, concurrency-rejection, and cancellation outcomes are recorded without ever reading request bodies or authorization values.</remarks>
public sealed class PlatformHttpResilienceHandler : DelegatingHandler
{
    private readonly PlatformHttpResilienceOptions _options;
    private readonly IPlatformHttpResilienceTelemetry _telemetry;
    private ResiliencePipeline<HttpResponseMessage>? _pipeline;
    private readonly object _pipelineLock = new();

    /// <summary>Initializes a new instance of the resilience handler.</summary>
    /// <param name="options">The validated resilience options.</param>
    /// <param name="telemetry">The bounded telemetry sink.</param>
    public PlatformHttpResilienceHandler(IOptions<PlatformHttpResilienceOptions> options, IPlatformHttpResilienceTelemetry telemetry)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(telemetry);

        _options = options.Value;
        _telemetry = telemetry;
    }

    /// <inheritdoc/>
    protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(request);

        if (!_options.Enabled)
            return await base.SendAsync(request, cancellationToken).ConfigureAwait(false);

        var pipeline = GetOrBuildPipeline();
        var context = ResilienceContextPool.Shared.Get(cancellationToken);
        context.Properties.Set(PlatformHttpResiliencePipeline.MethodProperty, request.Method.Method);

        try
        {
            return await pipeline.ExecuteAsync(
                static async (ctx, state) => await state.Handler.SendInnerAsync(state.Request, ctx.CancellationToken).ConfigureAwait(false),
                context,
                (Handler: this, Request: request)).ConfigureAwait(false);
        }
        catch (BrokenCircuitException)
        {
            _telemetry.Record(new PlatformHttpResilienceEvent(PlatformHttpResilienceDecision.CircuitOpen, request.Method.Method, 0, 0));
            throw;
        }
        catch (ExecutionRejectedException)
        {
            _telemetry.Record(new PlatformHttpResilienceEvent(PlatformHttpResilienceDecision.ConcurrencyRejected, request.Method.Method, 0, 0));
            throw;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            _telemetry.Record(new PlatformHttpResilienceEvent(PlatformHttpResilienceDecision.Cancelled, request.Method.Method, 0, 0));
            throw;
        }
        finally
        {
            ResilienceContextPool.Shared.Return(context);
        }
    }

    private Task<HttpResponseMessage> SendInnerAsync(HttpRequestMessage request, CancellationToken cancellationToken) =>
        base.SendAsync(request, cancellationToken);

    private ResiliencePipeline<HttpResponseMessage> GetOrBuildPipeline()
    {
        if (_pipeline is not null)
            return _pipeline;

        lock (_pipelineLock)
        {
            _pipeline ??= PlatformHttpResiliencePipeline.Build(_options);
            return _pipeline;
        }
    }
}
