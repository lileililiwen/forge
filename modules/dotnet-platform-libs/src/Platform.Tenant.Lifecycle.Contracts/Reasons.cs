namespace Platform.Tenant.Lifecycle.Contracts;

/// <summary>Stable, secret-free reason categories for readiness or status reports.</summary>
public static class TenantLifecycleReasons
{
    /// <summary>The most recent operation succeeded and the tenant is ready.</summary>
    public const string Ready = "ready";
    /// <summary>The most recent operation is still running.</summary>
    public const string Running = "running";
    /// <summary>The most recent operation is in a retryable state.</summary>
    public const string Retryable = "retryable";
    /// <summary>The most recent operation is permanently failed.</summary>
    public const string PermanentlyFailed = "permanently_failed";
    /// <summary>The most recent operation was canceled.</summary>
    public const string Canceled = "canceled";
    /// <summary>The most recent operation was denied by policy.</summary>
    public const string PolicyDenied = "policy_denied";
    /// <summary>No operation has been recorded for the tenant yet.</summary>
    public const string Unknown = "unknown";
}
