namespace Platform.Jobs.Hangfire;

/// <summary>Documented health of the Hangfire storage derived from the most recent readiness probe.</summary>
public enum HangfireJobsProviderState
{
    /// <summary>The most recent storage probe succeeded.</summary>
    Healthy,

    /// <summary>The most recent storage probe failed or has not run yet.</summary>
    Unavailable,
}

/// <summary>
/// Documented snapshot of the Hangfire adapter status. The snapshot carries
/// only the provider name, health state, and the last stable error code;
/// it never carries connection strings, credentials, or storage responses.
/// </summary>
/// <param name="Provider">The stable provider name, always <c>hangfire</c>.</param>
/// <param name="State">The derived health state.</param>
/// <param name="LastErrorCode">The last stable error code, or <c>null</c> when the last probe succeeded.</param>
public sealed record HangfireJobsProviderStatus(string Provider, HangfireJobsProviderState State, string? LastErrorCode = null)
{
    /// <summary>The stable provider name reported by the adapter.</summary>
    public const string ProviderName = "hangfire";
}
