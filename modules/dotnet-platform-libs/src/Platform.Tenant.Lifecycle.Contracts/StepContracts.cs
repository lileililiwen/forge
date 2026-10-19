namespace Platform.Tenant.Lifecycle.Contracts;

/// <summary>Context supplied to every step. Carries the operation identity, tenant identifier, and an opaque metadata bag the application populates.</summary>
public sealed record TenantLifecycleStepContext(
    TenantLifecycleOperationId OperationId,
    TenantLifecycleWorkflowName WorkflowName,
    TenantLifecycleStepName StepName,
    string TenantId,
    int Attempt,
    IReadOnlyDictionary<string, string> Metadata);

/// <summary>Result of a step execution. <see cref="SafeMessage"/> MUST NOT contain connection strings, secrets, provider response bodies, or stack traces.</summary>
public sealed record TenantLifecycleStepResult(
    TenantLifecycleStepOutcome Outcome,
    string? SafeMessage)
{
    /// <summary>Returns a succeeded result.</summary>
    public static TenantLifecycleStepResult Succeeded(string? safeMessage = null) => new(TenantLifecycleStepOutcome.Succeeded, safeMessage);
    /// <summary>Returns a retryable result. <paramref name="safeMessage"/> MUST be a stable, secret-free diagnostic.</summary>
    public static TenantLifecycleStepResult Retryable(string? safeMessage = null) => new(TenantLifecycleStepOutcome.Retryable, safeMessage);
    /// <summary>Returns a permanent result. <paramref name="safeMessage"/> MUST be a stable, secret-free diagnostic.</summary>
    public static TenantLifecycleStepResult Permanent(string? safeMessage = null) => new(TenantLifecycleStepOutcome.Permanent, safeMessage);
    /// <summary>Returns a canceled result.</summary>
    public static TenantLifecycleStepResult Canceled(string? safeMessage = null) => new(TenantLifecycleStepOutcome.Canceled, safeMessage);
    /// <summary>Returns a policy-denied result. <paramref name="safeMessage"/> MUST be a stable, secret-free diagnostic.</summary>
    public static TenantLifecycleStepResult PolicyDenied(string? safeMessage = null) => new(TenantLifecycleStepOutcome.PolicyDenied, safeMessage);
}

/// <summary>Step status recorded by the orchestrator. The status is the operation's durable checkpoint and is the source of truth for resume.</summary>
public sealed record TenantLifecycleStepStatus(
    TenantLifecycleStepName StepName,
    TenantLifecycleStepOutcome Outcome,
    string? SafeMessage,
    DateTimeOffset RecordedAt,
    int Attempt);

/// <summary>Status snapshot returned by the store. The snapshot is provider-neutral and safe to expose to readiness or admin adapters.</summary>
public sealed record TenantLifecycleOperationStatus(
    TenantLifecycleOperationId OperationId,
    TenantLifecycleWorkflowName WorkflowName,
    string TenantId,
    TenantLifecycleOperationState State,
    int CompletedStepCount,
    IReadOnlyList<TenantLifecycleStepStatus> StepStatuses,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt,
    string? SafeFailureMessage);
