using System.Globalization;

namespace Platform.Tenant.Lifecycle.Contracts;

/// <summary>Factory methods for tenant lifecycle step names and workflow names.</summary>
public static class TenantLifecycleNames
{
    /// <summary>Creates a step name from a string. Trims surrounding whitespace and rejects empty input.</summary>
    public static TenantLifecycleStepName Step(string name) => TenantLifecycleStepName.Parse(name);
    /// <summary>Creates a workflow name from a string. Trims surrounding whitespace and rejects empty input.</summary>
    public static TenantLifecycleWorkflowName Workflow(string name) => TenantLifecycleWorkflowName.Parse(name);
}

/// <summary>Maps operation state to the provider-neutral reason constants used by readiness and status reporting.</summary>
public static class TenantLifecycleStateExtensions
{
    /// <summary>Returns the provider-neutral reason constant for <paramref name="state"/>.</summary>
    public static string ToReason(this TenantLifecycleOperationState state) => state switch
    {
        TenantLifecycleOperationState.Succeeded => TenantLifecycleReasons.Ready,
        TenantLifecycleOperationState.Running => TenantLifecycleReasons.Running,
        TenantLifecycleOperationState.Retryable => TenantLifecycleReasons.Retryable,
        TenantLifecycleOperationState.PermanentlyFailed => TenantLifecycleReasons.PermanentlyFailed,
        TenantLifecycleOperationState.Canceled => TenantLifecycleReasons.Canceled,
        TenantLifecycleOperationState.PolicyDenied => TenantLifecycleReasons.PolicyDenied,
        _ => TenantLifecycleReasons.Unknown,
    };

    /// <summary>Returns a short, stable, secret-free diagnostic for <paramref name="outcome"/>.</summary>
    public static string ToReason(this TenantLifecycleStepOutcome outcome) => outcome switch
    {
        TenantLifecycleStepOutcome.Succeeded => "step_succeeded",
        TenantLifecycleStepOutcome.Retryable => "step_retryable",
        TenantLifecycleStepOutcome.Permanent => "step_permanent",
        TenantLifecycleStepOutcome.Canceled => "step_canceled",
        TenantLifecycleStepOutcome.PolicyDenied => "step_policy_denied",
        _ => "step_unknown",
    };
}

/// <summary>Helpers for the operation id counter used by the default in-memory store.</summary>
internal static class TenantLifecycleCounters
{
    public static long NextId() => Interlocked.Increment(ref s_counter);
    public static string FormatId(long value) => "tlc_" + value.ToString("D20", CultureInfo.InvariantCulture);
    private static long s_counter;
}
