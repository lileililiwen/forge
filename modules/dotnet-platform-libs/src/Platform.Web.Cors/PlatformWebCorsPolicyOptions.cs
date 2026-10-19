namespace Platform.Web.Cors;

/// <summary>Configures one named CORS policy. The platform applies a strict default; loosen with intent.</summary>
public sealed class PlatformWebCorsPolicyOptions
{
    /// <summary>The default maximum length of an origin or header value.</summary>
    public const int DefaultMaxOriginLength = 256;

    /// <summary>The default maximum number of allowed origins per policy.</summary>
    public const int DefaultMaxOriginsPerPolicy = 32;

    /// <summary>Gets or sets the policy name registered with ASP.NET Core.</summary>
    public string Name { get; set; } = "default";

    /// <summary>Gets or sets the list of allowed origins. Wildcard (<c>*</c>) is not allowed with credentials.</summary>
    public IList<string> AllowedOrigins { get; set; } = new List<string>();

    /// <summary>Gets or sets the list of allowed request headers.</summary>
    public IList<string> AllowedHeaders { get; set; } = new List<string>();

    /// <summary>Gets or sets the list of allowed response headers exposed to the browser.</summary>
    public IList<string> ExposedHeaders { get; set; } = new List<string>();

    /// <summary>Gets or sets the allowed HTTP methods. Defaults to safe methods only.</summary>
    public IList<string> AllowedMethods { get; set; } = new List<string> { "GET", "HEAD", "OPTIONS" };

    /// <summary>Gets or sets whether the policy allows credentials. Cannot be combined with wildcard origins.</summary>
    public bool AllowCredentials { get; set; }

    /// <summary>Gets or sets the preflight cache duration. Defaults to five minutes.</summary>
    public TimeSpan PreflightMaxAge { get; set; } = TimeSpan.FromMinutes(5);

    /// <summary>Validates option values and returns human-readable failures.</summary>
    public IReadOnlyList<string> Validate(string? environmentName)
    {
        var errors = new List<string>();
        if (string.IsNullOrWhiteSpace(Name)) errors.Add("Name is required.");
        if (Name is not null && Name.Contains(' ')) errors.Add("Name must not contain whitespace.");
        if (AllowedOrigins.Count > DefaultMaxOriginsPerPolicy) errors.Add($"At most {DefaultMaxOriginsPerPolicy} allowed origins are supported per policy.");
        if (AllowedMethods.Count == 0) errors.Add("At least one allowed method is required.");
        if (PreflightMaxAge < TimeSpan.Zero || PreflightMaxAge > TimeSpan.FromHours(24)) errors.Add("PreflightMaxAge must be between zero and 24 hours.");

        var hasWildcardOrigin = AllowedOrigins.Any(o => string.Equals(o, "*", StringComparison.Ordinal));
        if (hasWildcardOrigin && AllowCredentials) errors.Add("Wildcard origins cannot be combined with credentials.");
        if (hasWildcardOrigin && AllowedHeaders.Any(h => string.Equals(h, "*", StringComparison.OrdinalIgnoreCase))) errors.Add("Wildcard origins cannot be combined with wildcard headers.");

        if (string.Equals(environmentName, "Production", StringComparison.OrdinalIgnoreCase))
        {
            if (AllowedOrigins.Count == 0) errors.Add("At least one allowed origin is required in production.");
            if (hasWildcardOrigin) errors.Add("Wildcard origins are not allowed in production.");
        }

        foreach (var origin in AllowedOrigins)
        {
            if (string.IsNullOrWhiteSpace(origin)) { errors.Add("Allowed origins must be non-empty."); continue; }
            if (origin.Length > DefaultMaxOriginLength) { errors.Add($"Allowed origins must be at most {DefaultMaxOriginLength} characters."); continue; }
            if (string.Equals(origin, "*", StringComparison.Ordinal)) continue;
            if (string.Equals(origin, "http://localhost", StringComparison.OrdinalIgnoreCase)) continue;
            if (!Uri.TryCreate(origin, UriKind.Absolute, out var parsed) || (parsed.Scheme != Uri.UriSchemeHttps && parsed.Scheme != Uri.UriSchemeHttp))
            {
                errors.Add($"Origin '{origin}' is not an absolute URL.");
                continue;
            }
            if (parsed.Scheme != Uri.UriSchemeHttps)
            {
                if (string.Equals(environmentName, "Production", StringComparison.OrdinalIgnoreCase))
                    errors.Add($"Origin '{origin}' must use HTTPS in production.");
            }
        }

        foreach (var header in AllowedHeaders)
            if (string.IsNullOrWhiteSpace(header)) errors.Add("Allowed headers must be non-empty.");
        foreach (var method in AllowedMethods)
            if (string.IsNullOrWhiteSpace(method)) errors.Add("Allowed methods must be non-empty.");
        foreach (var header in ExposedHeaders)
            if (string.IsNullOrWhiteSpace(header)) errors.Add("Exposed headers must be non-empty.");

        return errors;
    }
}
