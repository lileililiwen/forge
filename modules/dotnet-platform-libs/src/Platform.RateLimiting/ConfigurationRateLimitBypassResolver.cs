using Microsoft.Extensions.Options;

namespace Platform.RateLimiting;

/// <summary>
/// Default <see cref="IRateLimitBypassResolver"/> implementation
/// that matches the configured bypass tokens against the request's
/// <see cref="HttpContextAbstraction.BypassToken"/>. Tokens are
/// compared using the ordinal-ignore-case rule.
/// </summary>
public sealed class ConfigurationRateLimitBypassResolver : IRateLimitBypassResolver
{
    private readonly IOptions<RateLimitingOptions> _options;

    /// <summary>
    /// Initializes a new <see cref="ConfigurationRateLimitBypassResolver"/>
    /// that reads the configured bypass tokens from the supplied
    /// options.
    /// </summary>
    /// <param name="options">The rate-limiting options.</param>
    /// <exception cref="ArgumentNullException"><paramref name="options"/> is <c>null</c>.</exception>
    public ConfigurationRateLimitBypassResolver(IOptions<RateLimitingOptions> options)
    {
        ArgumentNullException.ThrowIfNull(options);
        _options = options;
    }

    /// <inheritdoc />
    public RateLimitBypassDecision Evaluate(HttpContextAbstraction context)
    {
        ArgumentNullException.ThrowIfNull(context);

        if (string.IsNullOrWhiteSpace(context.BypassToken))
        {
            return new RateLimitBypassDecision(Allowed: false);
        }

        var tokens = _options.Value.BypassTokens;
        if (tokens is null || tokens.Count == 0)
        {
            return new RateLimitBypassDecision(Allowed: false);
        }

        foreach (var token in tokens)
        {
            if (string.Equals(token, context.BypassToken, StringComparison.OrdinalIgnoreCase))
            {
                return new RateLimitBypassDecision(Allowed: true, Label: token);
            }
        }

        return new RateLimitBypassDecision(Allowed: false);
    }
}
