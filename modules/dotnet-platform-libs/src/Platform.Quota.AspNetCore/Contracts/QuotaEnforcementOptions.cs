using Microsoft.AspNetCore.Http;

namespace Platform.Quota.AspNetCore.Contracts;

/// <summary>How the adapter treats a request with no resolved subject or tenant context.</summary>
public enum MissingContextPolicy
{
    /// <summary>Deny the request (fail-closed). The default.</summary>
    FailClosed,
    /// <summary>Allow the request using the configured anonymous subject.</summary>
    Allow
}

/// <summary>How the adapter treats a quota store that is unreachable or throws.</summary>
public enum QuotaUnavailablePolicy
{
    /// <summary>Deny the request (fail-closed). The default.</summary>
    FailClosed,
    /// <summary>Allow the request and record the failure.</summary>
    Allow
}

/// <summary>ASP.NET Core quota enforcement settings.</summary>
public sealed class QuotaEnforcementOptions
{
    /// <summary>When false, the middleware no-ops and never consumes quota.</summary>
    public bool Enabled { get; set; } = true;

    /// <summary>Policy applied when no subject or tenant can be resolved.</summary>
    public MissingContextPolicy MissingContextPolicy { get; set; } = MissingContextPolicy.FailClosed;

    /// <summary>Policy applied when the quota store is unavailable.</summary>
    public QuotaUnavailablePolicy ProviderUnavailablePolicy { get; set; } = QuotaUnavailablePolicy.FailClosed;

    /// <summary>Subject used when <see cref="MissingContextPolicy"/> is <see cref="MissingContextPolicy.Allow"/>.</summary>
    public string AnonymousSubject { get; set; } = "anonymous";

    /// <summary>Request path prefixes exempt from enforcement (case-insensitive ordinal prefix match).</summary>
    public List<string> ExemptPathPrefixes { get; } = new() { "/health", "/healthz", "/ready", "/live", "/alive", "/metrics" };

    /// <summary>HTTP methods exempt from enforcement (uppercase, e.g. "GET").</summary>
    public List<string> ExemptMethods { get; } = new();

    /// <summary>HTTP status returned when the quota is exceeded.</summary>
    public int QuotaExceededStatusCode { get; set; } = StatusCodes.Status429TooManyRequests;

    /// <summary>HTTP status returned when context is missing and the policy is fail-closed.</summary>
    public int MissingContextStatusCode { get; set; } = StatusCodes.Status403Forbidden;

    /// <summary>HTTP status returned when the quota provider is unavailable and the policy is fail-closed.</summary>
    public int ProviderUnavailableStatusCode { get; set; } = StatusCodes.Status503ServiceUnavailable;

    /// <summary>RFC 9457 problem type for quota-exceeded responses.</summary>
    public string QuotaExceededType { get; set; } = "https://datatracker.ietf.org/doc/html/rfc6585#section-4";

    /// <summary>RFC 9457 problem title for quota-exceeded responses.</summary>
    public string QuotaExceededTitle { get; set; } = "Quota Exceeded";

    /// <summary>Request header consulted by <see cref="Platform.Quota.AspNetCore.Resolvers.HeaderQuotaSubjectResolver"/> (null disables the header).</summary>
    public string? SubjectHeaderName { get; set; } = "X-Quota-Subject";

    /// <summary>Validates the configuration.</summary>
    public void Validate()
    {
        if (QuotaExceededStatusCode is < 400 or > 599) throw new ArgumentOutOfRangeException(nameof(QuotaExceededStatusCode));
        if (MissingContextStatusCode is < 400 or > 599) throw new ArgumentOutOfRangeException(nameof(MissingContextStatusCode));
        if (ProviderUnavailableStatusCode is < 400 or > 599) throw new ArgumentOutOfRangeException(nameof(ProviderUnavailableStatusCode));
        if (string.IsNullOrWhiteSpace(AnonymousSubject)) throw new ArgumentException("The anonymous subject must be a non-empty value.", nameof(AnonymousSubject));
        if (string.IsNullOrWhiteSpace(QuotaExceededType)) throw new ArgumentException("The quota-exceeded type must be a non-empty value.", nameof(QuotaExceededType));
        if (string.IsNullOrWhiteSpace(QuotaExceededTitle)) throw new ArgumentException("The quota-exceeded title must be a non-empty value.", nameof(QuotaExceededTitle));
    }
}
