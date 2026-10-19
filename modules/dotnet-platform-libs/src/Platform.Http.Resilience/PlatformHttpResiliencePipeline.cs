using System;
using System.Collections.Generic;
using System.Net;
using System.Threading.RateLimiting;
using Polly;
using Polly.CircuitBreaker;
using Polly.RateLimiting;
using Polly.Retry;
using Polly.Timeout;

namespace Platform.Http.Resilience;

/// <summary>Maps the platform <see cref="PlatformHttpResilienceOptions"/> onto a standard Polly resilience pipeline (retry, per-attempt timeout, circuit breaker, concurrency, total timeout).</summary>
/// <remarks>The pipeline is built once and executed through a <see cref="ResilienceContext"/> that carries the request method so retries stay restricted to idempotent methods. Retries are restricted to idempotent methods so writes are never replayed automatically.</remarks>
internal static class PlatformHttpResiliencePipeline
{
    /// <summary>The resilience-context property key under which the request HTTP method is stashed so the retry predicate can classify idempotency.</summary>
    internal static readonly ResiliencePropertyKey<string> MethodProperty = new("platform-http-resilience-method");

    public static ResiliencePipeline<HttpResponseMessage> Build(PlatformHttpResilienceOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);

        var idempotent = DefaultIdempotentMethods(options);

        var retry = new RetryStrategyOptions<HttpResponseMessage>
        {
            MaxRetryAttempts = Math.Max(0, options.MaxRetryAttempts),
            Delay = options.RetryBaseDelay,
            BackoffType = DelayBackoffType.Exponential,
            UseJitter = true,
            ShouldHandle = args => new ValueTask<bool>(ShouldRetry(args, idempotent)),
        };

        var attemptTimeout = new TimeoutStrategyOptions { Timeout = options.AttemptTimeout };

        var circuitBreaker = new CircuitBreakerStrategyOptions<HttpResponseMessage>
        {
            FailureRatio = options.CircuitBreakerFailureRatio,
            MinimumThroughput = options.CircuitBreakerMinimumThroughput,
            SamplingDuration = options.CircuitBreakerSamplingDuration,
            BreakDuration = options.CircuitBreakerBreakDuration,
            ShouldHandle = args => new ValueTask<bool>(ShouldBreak(args)),
        };

        var concurrencyLimiter = new ConcurrencyLimiter(new ConcurrencyLimiterOptions
        {
            PermitLimit = options.MaxConcurrentCalls,
            QueueLimit = options.MaxQueueLength,
        });
        var rateLimiter = new RateLimiterStrategyOptions
        {
            RateLimiter = arguments => concurrencyLimiter.AcquireAsync(1, arguments.Context.CancellationToken),
        };

        var totalTimeout = new TimeoutStrategyOptions { Timeout = options.TotalTimeout };

        var builder = new ResiliencePipelineBuilder<HttpResponseMessage>();
        builder.AddTimeout(totalTimeout);
        builder.AddRetry(retry);
        builder.AddTimeout(attemptTimeout);
        builder.AddCircuitBreaker(circuitBreaker);
        builder.AddRateLimiter(rateLimiter);
        return builder.Build();
    }

    private static bool ShouldRetry(RetryPredicateArguments<HttpResponseMessage> args, HashSet<string> idempotent)
    {
        if (args.Outcome.Exception is OperationCanceledException or TaskCanceledException)
            return false;

        if (args.Context.Properties.TryGetValue(MethodProperty, out var method)
            && method is not null
            && !idempotent.Contains(method))
            return false;

        return args.Outcome.Exception is HttpRequestException or TimeoutException
            || (args.Outcome.Result is { } response
                && ((int)response.StatusCode >= 500 || response.StatusCode == HttpStatusCode.RequestTimeout));
    }

    private static bool ShouldBreak(CircuitBreakerPredicateArguments<HttpResponseMessage> args)
    {
        if (args.Outcome.Exception is OperationCanceledException or TaskCanceledException)
            return false;

        return args.Outcome.Exception is HttpRequestException or TimeoutException
            || (args.Outcome.Result is { } response
                && ((int)response.StatusCode >= 500 || response.StatusCode == HttpStatusCode.RequestTimeout));
    }

    private static HashSet<string> DefaultIdempotentMethods(PlatformHttpResilienceOptions options)
    {
        var set = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "GET", "HEAD", "OPTIONS", "TRACE",
        };
        foreach (var method in options.IdempotentMethods)
            set.Add(method);
        return set;
    }
}
