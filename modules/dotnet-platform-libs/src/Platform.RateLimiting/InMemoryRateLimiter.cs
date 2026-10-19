using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.RateLimiting;

/// <summary>
/// Default in-memory implementation of <see cref="IRateLimiter"/>.
/// The limiter tracks a per-key windowed counter, returns a
/// <see cref="RateLimitDecision"/> with the documented fields, and
/// reads the current time from an injected
/// <see cref="IClock"/> so the bucket is deterministic in tests.
/// </summary>
public sealed class InMemoryRateLimiter : IRateLimiter
{
    private readonly IClock _clock;
    private readonly IOptions<RateLimitingOptions> _options;
    private readonly Dictionary<string, Bucket> _buckets = new(StringComparer.Ordinal);
    private readonly object _lock = new();

    /// <summary>
    /// Initializes a new <see cref="InMemoryRateLimiter"/> with the
    /// supplied dependencies.
    /// </summary>
    /// <param name="clock">The platform clock.</param>
    /// <param name="options">The rate-limiting options.</param>
    /// <exception cref="ArgumentNullException">A required dependency is <c>null</c>.</exception>
    public InMemoryRateLimiter(IClock clock, IOptions<RateLimitingOptions> options)
    {
        ArgumentNullException.ThrowIfNull(clock);
        ArgumentNullException.ThrowIfNull(options);
        _clock = clock;
        _options = options;
    }

    /// <inheritdoc />
    public Task<RateLimitDecision> CheckAsync(RateLimitKey key, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        ValidateKey(key);

        var policy = _options.Value.Policies.Find(key.Policy)
            ?? throw new ArgumentException(
                $"Unknown rate-limit policy '{key.Policy}'.",
                nameof(key));

        var now = _clock.UtcNow;
        var window = TimeSpan.FromSeconds(policy.WindowSeconds);
        var composite = key.Composite;

        lock (_lock)
        {
            if (!_buckets.TryGetValue(composite, out var bucket)
                || now - bucket.WindowStart >= window)
            {
                _buckets[composite] = new Bucket(WindowStart: now, Count: 1);
                return Task.FromResult(new RateLimitDecision(
                    Allowed: true,
                    Limit: policy.Limit,
                    Remaining: policy.Limit - 1,
                    RetryAfterSeconds: 0));
            }

            if (bucket.Count >= policy.Limit)
            {
                var elapsed = now - bucket.WindowStart;
                var remaining = window - elapsed;
                var retryAfter = (int)Math.Max(1, Math.Ceiling(remaining.TotalSeconds));
                return Task.FromResult(new RateLimitDecision(
                    Allowed: false,
                    Limit: policy.Limit,
                    Remaining: 0,
                    RetryAfterSeconds: retryAfter));
            }

            _buckets[composite] = bucket with { Count = bucket.Count + 1 };
            var remaining2 = policy.Limit - bucket.Count - 1;
            return Task.FromResult(new RateLimitDecision(
                Allowed: true,
                Limit: policy.Limit,
                Remaining: remaining2,
                RetryAfterSeconds: 0));
        }
    }

    private static void ValidateKey(RateLimitKey key)
    {
        if (string.IsNullOrWhiteSpace(key.Policy))
        {
            throw new ArgumentException("Policy must be a non-empty string.", nameof(key));
        }
        if (string.IsNullOrWhiteSpace(key.Subject))
        {
            throw new ArgumentException("Subject must be a non-empty string.", nameof(key));
        }
    }

    private sealed record Bucket(DateTimeOffset WindowStart, int Count);
}
