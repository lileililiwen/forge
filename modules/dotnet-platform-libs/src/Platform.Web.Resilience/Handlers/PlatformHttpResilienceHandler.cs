using System.Net;
using System.Net.Http.Headers;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Web.Telemetry;

namespace Platform.Web.Resilience.Handlers;

/// <summary>Retry + timeout + circuit-breaker <see cref="DelegatingHandler"/> with bounded defaults.</summary>
/// <remarks>The handler only retries requests whose method is in the configured idempotent set. The default idempotent set is the IETF-safe set: <c>GET</c>, <c>HEAD</c>, <c>OPTIONS</c>, and <c>PUT</c> with an <c>If-Match</c> header. POST, PATCH, and DELETE are not retried unless the host explicitly opts in via <see cref="PlatformHttpResilienceOptions.IdempotentMethods"/>.</remarks>
public sealed class PlatformHttpResilienceHandler : DelegatingHandler
{
    private static readonly HashSet<string> SafeDefaultMethods = new(StringComparer.OrdinalIgnoreCase) { "GET", "HEAD", "OPTIONS" };

    private readonly IHttpResilienceTelemetry _telemetry;
    private readonly PlatformWebTelemetrySafeValuePolicy _safeValuePolicy;
    private readonly PlatformHttpResilienceOptions _options;
    private readonly ILogger<PlatformHttpResilienceHandler> _logger;
    private readonly CircuitState _circuit = new();

    /// <summary>Initializes a new instance of the <see cref="PlatformHttpResilienceHandler"/> class.</summary>
    public PlatformHttpResilienceHandler(IOptions<PlatformHttpResilienceOptions> options, IHttpResilienceTelemetry telemetry, IPlatformWebTelemetry webTelemetry, ILogger<PlatformHttpResilienceHandler> logger)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(telemetry);
        ArgumentNullException.ThrowIfNull(webTelemetry);
        ArgumentNullException.ThrowIfNull(logger);
        _options = options.Value;
        _telemetry = telemetry;
        _logger = logger;
        _safeValuePolicy = webTelemetry.SafeValuePolicy;
    }

    /// <inheritdoc />
    protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(request);
        var operation = ResolveOperation(request);
        var isIdempotent = IsIdempotent(request);
        var maxAttempts = isIdempotent ? _options.MaxRetryAttempts : 1;

        if (_circuit.IsOpen(_options))
        {
            _telemetry.Record(new HttpResilienceEvent(HttpResilienceDecision.CircuitBroken, operation, 1, 0));
            throw new PlatformHttpCircuitOpenException(operation);
        }

        HttpResponseMessage? response = null;
        Exception? lastException = null;
        for (var attempt = 1; attempt <= maxAttempts; attempt++)
        {
            using var attemptCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            attemptCts.CancelAfter(_options.AttemptTimeout);
            try
            {
                if (_options.EmitRetryAttemptHeader)
                {
                    var attemptValue = attempt.ToString(System.Globalization.CultureInfo.InvariantCulture);
                    if (request.Headers.Contains("X-Retry-Attempt"))
                        request.Headers.Remove("X-Retry-Attempt");
                    request.Headers.TryAddWithoutValidation("X-Retry-Attempt", attemptValue);
                }
                response = await base.SendAsync(request, attemptCts.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (attemptCts.IsCancellationRequested && !cancellationToken.IsCancellationRequested)
            {
                lastException = new TimeoutException($"HTTP attempt exceeded {_options.AttemptTimeout.TotalMilliseconds:N0}ms.");
                _telemetry.Record(new HttpResilienceEvent(HttpResilienceDecision.TimedOut, operation, attempt, 0));
                if (attempt < maxAttempts) continue;
                throw lastException;
            }
            catch (HttpRequestException ex)
            {
                lastException = ex;
                if (attempt < maxAttempts)
                {
                    _telemetry.Record(new HttpResilienceEvent(HttpResilienceDecision.Retried, operation, attempt, 0));
                    await BackOffAsync(attempt, cancellationToken).ConfigureAwait(false);
                    continue;
                }
                _circuit.RecordFailure(_options);
                throw;
            }

            if (IsTransientStatus(response.StatusCode))
            {
                if (attempt < maxAttempts)
                {
                    _telemetry.Record(new HttpResilienceEvent(HttpResilienceDecision.Retried, operation, attempt, (int)response.StatusCode));
                    response.Dispose();
                    response = null;
                    await BackOffAsync(attempt, cancellationToken).ConfigureAwait(false);
                    continue;
                }
                _circuit.RecordFailure(_options);
                return response;
            }

            _circuit.RecordSuccess();
            return response;
        }

        if (response is not null) return response;
        throw lastException ?? new InvalidOperationException("Resilience handler exited without a response.");
    }

    private async Task BackOffAsync(int attempt, CancellationToken cancellationToken)
    {
        var delay = TimeSpan.FromMilliseconds(_options.RetryBaseDelay.TotalMilliseconds * attempt);
        if (delay > TimeSpan.Zero)
        {
            try { await Task.Delay(delay, cancellationToken).ConfigureAwait(false); }
            catch (OperationCanceledException) { throw; }
        }
    }

    private bool IsIdempotent(HttpRequestMessage request)
    {
        var method = request.Method.Method;
        if (SafeDefaultMethods.Contains(method)) return true;
        if (_options.IdempotentMethods.Contains(method)) return true;
        if (string.Equals(method, "PUT", StringComparison.OrdinalIgnoreCase) && request.Headers.IfMatch is { Count: > 0 })
            return true;
        if (string.Equals(method, "DELETE", StringComparison.OrdinalIgnoreCase) && request.Headers.IfMatch is { Count: > 0 })
            return true;
        return false;
    }

    private static bool IsTransientStatus(HttpStatusCode statusCode) => statusCode is HttpStatusCode.RequestTimeout or HttpStatusCode.TooManyRequests or HttpStatusCode.InternalServerError or HttpStatusCode.BadGateway or HttpStatusCode.ServiceUnavailable or HttpStatusCode.GatewayTimeout;

    private string ResolveOperation(HttpRequestMessage request)
    {
        var route = request.RequestUri?.GetLeftPart(UriPartial.Path) ?? "unknown";
        return _safeValuePolicy.RequireOperation($"{request.Method.Method} {route}");
    }

    private sealed class CircuitState
    {
        private readonly object _gate = new();
        private int _failures;
        private int _throughput;
        private DateTimeOffset _windowStart = DateTimeOffset.UtcNow;
        private DateTimeOffset _openUntil = DateTimeOffset.MinValue;

        public bool IsOpen(PlatformHttpResilienceOptions options)
        {
            lock (_gate)
            {
                if (_openUntil > DateTimeOffset.UtcNow) return true;
                if (_openUntil > DateTimeOffset.MinValue && _openUntil <= DateTimeOffset.UtcNow) _openUntil = DateTimeOffset.MinValue;
                if (DateTimeOffset.UtcNow - _windowStart > options.CircuitBreakerSamplingDuration)
                {
                    _windowStart = DateTimeOffset.UtcNow;
                    _failures = 0;
                    _throughput = 0;
                }
                return false;
            }
        }

        public void RecordFailure(PlatformHttpResilienceOptions options)
        {
            lock (_gate)
            {
                _failures++;
                _throughput++;
                if (_throughput >= options.CircuitBreakerMinimumThroughput
                    && (double)_failures / _throughput >= options.CircuitBreakerFailureRatio)
                {
                    _openUntil = DateTimeOffset.UtcNow + options.CircuitBreakerBreakDuration;
                }
            }
        }

        public void RecordSuccess()
        {
            lock (_gate)
            {
                _failures = 0;
                _throughput = 0;
                _windowStart = DateTimeOffset.UtcNow;
            }
        }
    }
}
