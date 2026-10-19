namespace Platform.Idempotency;

/// <summary>
/// Documented value type for a stored idempotent response. Consumers
/// compute a <see cref="Fingerprint"/> from the request, look up the
/// record through <see cref="IIdempotencyStore.TryGetAsync"/>, and
/// write a new record through
/// <see cref="IIdempotencyStore.SaveAsync"/>.
/// </summary>
/// <param name="Key">The documented idempotency key. MUST be non-null and non-empty.</param>
/// <param name="Fingerprint">The documented request fingerprint. MUST be non-null and non-empty.</param>
/// <param name="StatusCode">The HTTP status code of the original response.</param>
/// <param name="ContentType">The response content type.</param>
/// <param name="ResponseHeaders">The response headers, keyed by header name.</param>
/// <param name="ResponseBody">The response body, if any.</param>
/// <param name="CreatedAt">The UTC time the record was created.</param>
public sealed record IdempotencyRecord(
    string Key,
    string Fingerprint,
    int StatusCode,
    string? ContentType,
    IReadOnlyDictionary<string, string> ResponseHeaders,
    string? ResponseBody,
    DateTimeOffset CreatedAt)
{
    /// <summary>
    /// Initializes a new <see cref="IdempotencyRecord"/> with no
    /// response headers and a <c>null</c> body. The current time
    /// MUST be supplied by the caller (typically from
    /// <see cref="Platform.Core.Time.IClock"/>).
    /// </summary>
    public IdempotencyRecord(
        string Key,
        string Fingerprint,
        int StatusCode,
        string? ContentType,
        string? ResponseBody,
        DateTimeOffset CreatedAt)
        : this(
            Key,
            Fingerprint,
            StatusCode,
            ContentType,
            ResponseHeaders: new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase),
            ResponseBody,
            CreatedAt)
    {
    }
}
