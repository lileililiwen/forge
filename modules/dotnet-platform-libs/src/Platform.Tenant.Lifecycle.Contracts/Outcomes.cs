namespace Platform.Tenant.Lifecycle.Contracts;

/// <summary>Classified outcome of a tenant lifecycle step. The orchestrator uses this to decide whether to stop, retry, or fail closed.</summary>
public enum TenantLifecycleStepOutcome
{
    /// <summary>Step applied without error and is durably recorded.</summary>
    Succeeded = 0,
    /// <summary>Step failed but the application considers the failure transient. The orchestrator pauses and exposes <see cref="TenantLifecycleOperationState.Retryable"/>.</summary>
    Retryable = 1,
    /// <summary>Step failed and the application considers the failure final. The orchestrator exposes <see cref="TenantLifecycleOperationState.PermanentlyFailed"/>.</summary>
    Permanent = 2,
    /// <summary>Step was canceled before completion.</summary>
    Canceled = 3,
    /// <summary>Step was denied by application policy (configuration, plan, etc.).</summary>
    PolicyDenied = 4,
}

/// <summary>Overall state of a tenant lifecycle operation. Status snapshots report the latest state and the index of the last completed step.</summary>
public enum TenantLifecycleOperationState
{
    /// <summary>The operation has been created but no step has run.</summary>
    Pending = 0,
    /// <summary>At least one step is currently running.</summary>
    Running = 1,
    /// <summary>All steps succeeded.</summary>
    Succeeded = 2,
    /// <summary>The most recent failure is retryable; the operator can resume.</summary>
    Retryable = 3,
    /// <summary>The most recent failure is permanent and cannot be retried without a configuration change.</summary>
    PermanentlyFailed = 4,
    /// <summary>The operation was canceled.</summary>
    Canceled = 5,
    /// <summary>The operation was denied by application policy.</summary>
    PolicyDenied = 6,
}
