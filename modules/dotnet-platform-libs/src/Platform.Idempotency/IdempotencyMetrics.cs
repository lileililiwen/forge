namespace Platform.Idempotency;

#pragma warning disable CA1715 // Intentional: "IdempotencyMetrics" matches the documented type name in this style of API.

/// <summary>
/// Documented counter surface for the idempotency package. Consumers
/// wire these counters to their existing observability layer so
/// existing dashboards keep receiving the documented names after the
/// consumer migrates to the platform package.
/// </summary>
public interface IdempotencyMetrics
{
    /// <summary>
    /// Increments the documented <c>idempotency.hit</c> counter when
    /// a duplicate request replays a stored response.
    /// </summary>
    void Hit();

    /// <summary>
    /// Increments the documented <c>idempotency.miss</c> counter
    /// when a request does not match any stored record.
    /// </summary>
    void Miss();

    /// <summary>
    /// Increments the documented
    /// <c>idempotency.fingerprint_mismatch</c> counter when a key
    /// matches a stored record but the request fingerprint differs.
    /// </summary>
    void FingerprintMismatch();
}

#pragma warning restore CA1715
