using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;
using Platform.Tenant.Lifecycle.Contracts;

#pragma warning disable CA1848 // Use LoggerMessage delegates for high-performance logging.

namespace Platform.Tenant.Lifecycle;

/// <summary>
/// Default <see cref="ITenantLifecycleOrchestrator"/>. Runs ordered steps, skips
/// already-completed steps on resume, installs tenant scope around each
/// tenant-scoped step, and records durable state through the supplied
/// <see cref="ITenantLifecycleStore"/>.
/// </summary>
public sealed class TenantLifecycleOrchestrator : ITenantLifecycleOrchestrator
{
    private readonly ITenantLifecycleStore _store;
    private readonly ITenantLifecycleScopeCallback _scopeCallback;
    private readonly ILogger<TenantLifecycleOrchestrator> _logger;

    /// <summary>Creates a new orchestrator.</summary>
    public TenantLifecycleOrchestrator(
        ITenantLifecycleStore store,
        ITenantLifecycleScopeCallback scopeCallback,
        ILogger<TenantLifecycleOrchestrator>? logger = null)
    {
        _store = store ?? throw new ArgumentNullException(nameof(store));
        _scopeCallback = scopeCallback ?? throw new ArgumentNullException(nameof(scopeCallback));
        _logger = logger ?? NullLogger<TenantLifecycleOrchestrator>.Instance;
    }

    /// <inheritdoc />
    public async ValueTask<TenantLifecycleOperationStatus> StartAsync(
        ITenantLifecycleWorkflow workflow,
        string tenantId,
        IReadOnlyDictionary<string, string> metadata,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(workflow);
        var resolvedSteps = ResolveSteps(workflow);
        var operationId = await _store.CreateOperationAsync(workflow.Name, tenantId, metadata ?? new Dictionary<string, string>(), cancellationToken).ConfigureAwait(false);
        return await RunAsync(operationId, workflow, resolvedSteps, tenantId, isResume: false, cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async ValueTask<TenantLifecycleOperationStatus> ResumeAsync(
        TenantLifecycleOperationId operationId,
        CancellationToken cancellationToken = default)
    {
        var status = await _store.GetStatusAsync(operationId, cancellationToken).ConfigureAwait(false)
            ?? throw new InvalidOperationException($"Operation '{operationId}' was not found.");
        if (status.State == TenantLifecycleOperationState.Succeeded)
            return status;
        if (status.State is TenantLifecycleOperationState.PermanentlyFailed or TenantLifecycleOperationState.PolicyDenied or TenantLifecycleOperationState.Canceled)
            return status;
        if (_workflowRegistry is null || !_workflowRegistry.TryGet(status.WorkflowName, out var registered))
            throw new InvalidOperationException($"Workflow '{status.WorkflowName}' is not registered with the resume registry.");
        var ordered = ResolveSteps(registered!);
        return await RunAsync(operationId, registered!, ordered, status.TenantId, isResume: true, cancellationToken).ConfigureAwait(false);
    }

    private ITenantLifecycleWorkflowRegistry? _workflowRegistry;

    /// <summary>Attaches a workflow registry so the orchestrator can resume by name.</summary>
    public void AttachRegistry(ITenantLifecycleWorkflowRegistry registry) => _workflowRegistry = registry;

    private async ValueTask<TenantLifecycleOperationStatus> RunAsync(
        TenantLifecycleOperationId operationId,
        ITenantLifecycleWorkflow workflow,
        ITenantLifecycleStep[] orderedSteps,
        string tenantId,
        bool isResume,
        CancellationToken cancellationToken)
    {
        await _store.SetOperationStateAsync(operationId, TenantLifecycleOperationState.Running, null, cancellationToken).ConfigureAwait(false);

        var completedNames = new HashSet<TenantLifecycleStepName>();
        if (isResume)
        {
            var existing = await _store.GetStatusAsync(operationId, cancellationToken).ConfigureAwait(false);
            if (existing is not null)
            {
                foreach (var stepStatus in existing.StepStatuses.Where(s => s.Outcome == TenantLifecycleStepOutcome.Succeeded))
                    completedNames.Add(stepStatus.StepName);
            }
        }

        TenantLifecycleOperationState finalState = TenantLifecycleOperationState.Succeeded;
        string? finalMessage = null;
        try
        {
            foreach (var step in orderedSteps)
            {
                cancellationToken.ThrowIfCancellationRequested();
                if (completedNames.Contains(step.Name))
                    continue;
                var stepResult = await RunStepAsync(step, operationId, workflow.Name, tenantId, cancellationToken).ConfigureAwait(false);
                var attempt = (await _store.GetStatusAsync(operationId, cancellationToken).ConfigureAwait(false))?
                    .StepStatuses.Count(s => s.StepName == step.Name) + 1 ?? 1;
                await _store.RecordStepStatusAsync(operationId, new TenantLifecycleStepStatus(step.Name, stepResult.Outcome, stepResult.SafeMessage, DateTimeOffset.UtcNow, attempt), cancellationToken).ConfigureAwait(false);
                switch (stepResult.Outcome)
                {
                    case TenantLifecycleStepOutcome.Succeeded:
                        completedNames.Add(step.Name);
                        continue;
                    case TenantLifecycleStepOutcome.Retryable:
                        finalState = TenantLifecycleOperationState.Retryable;
                        finalMessage = stepResult.SafeMessage;
                        break;
                    case TenantLifecycleStepOutcome.Permanent:
                        finalState = TenantLifecycleOperationState.PermanentlyFailed;
                        finalMessage = stepResult.SafeMessage;
                        break;
                    case TenantLifecycleStepOutcome.Canceled:
                        finalState = TenantLifecycleOperationState.Canceled;
                        finalMessage = stepResult.SafeMessage;
                        break;
                    case TenantLifecycleStepOutcome.PolicyDenied:
                        finalState = TenantLifecycleOperationState.PolicyDenied;
                        finalMessage = stepResult.SafeMessage;
                        break;
                    default:
                        finalState = TenantLifecycleOperationState.PermanentlyFailed;
                        finalMessage = stepResult.SafeMessage;
                        break;
                }
                break;
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            finalState = TenantLifecycleOperationState.Canceled;
            finalMessage = "orchestrator canceled";
        }

        await _store.SetOperationStateAsync(operationId, finalState, finalMessage, cancellationToken).ConfigureAwait(false);
        return (await _store.GetStatusAsync(operationId, cancellationToken).ConfigureAwait(false))!;
    }

    private async ValueTask<TenantLifecycleStepResult> RunStepAsync(
        ITenantLifecycleStep step,
        TenantLifecycleOperationId operationId,
        TenantLifecycleWorkflowName workflowName,
        string tenantId,
        CancellationToken cancellationToken)
    {
        if (step.IsTenantScoped)
        {
            using var scope = _scopeCallback.BeginTenantScope(tenantId);
            if (_logger.IsEnabled(LogLevel.Debug))
            {
                _logger.LogDebug("Running tenant-scoped step {Step} for {Operation}", step.Name, operationId);
            }
            return await step.ExecuteAsync(new TenantLifecycleStepContext(operationId, workflowName, step.Name, tenantId, Attempt: 1, Metadata: new Dictionary<string, string>()), cancellationToken).ConfigureAwait(false);
        }
        if (_logger.IsEnabled(LogLevel.Debug))
        {
            _logger.LogDebug("Running global step {Step} for {Operation}", step.Name, operationId);
        }
        return await step.ExecuteAsync(new TenantLifecycleStepContext(operationId, workflowName, step.Name, tenantId, Attempt: 1, Metadata: new Dictionary<string, string>()), cancellationToken).ConfigureAwait(false);
    }

    private static ITenantLifecycleStep[] ResolveSteps(ITenantLifecycleWorkflow workflow)
    {
        if (workflow.Steps is null || workflow.Steps.Count == 0)
            throw new ArgumentException("Workflow must declare at least one step.", nameof(workflow));
        var byName = new HashSet<TenantLifecycleStepName>();
        foreach (var step in workflow.Steps)
        {
            if (!byName.Add(step.Name))
                throw new ArgumentException($"Workflow '{workflow.Name}' declares the step '{step.Name}' more than once.", nameof(workflow));
        }
        return workflow.Steps.OrderBy(s => s.Order).ThenBy(s => s.Name.Value, StringComparer.Ordinal).ToArray();
    }
}

#pragma warning restore CA1848

/// <summary>Registry of workflows the orchestrator can resume by name.</summary>
public interface ITenantLifecycleWorkflowRegistry
{
    /// <summary>Returns <c>true</c> when a workflow with the supplied name is registered.</summary>
    bool TryGet(TenantLifecycleWorkflowName name, out ITenantLifecycleWorkflow? workflow);
}

/// <summary>Default <see cref="ITenantLifecycleWorkflowRegistry"/> implementation backed by a dictionary.</summary>
public sealed class TenantLifecycleWorkflowRegistry : ITenantLifecycleWorkflowRegistry
{
    private readonly Dictionary<TenantLifecycleWorkflowName, ITenantLifecycleWorkflow> _workflows = new();

    /// <summary>Registers a workflow. Re-registering the same name replaces the previous registration.</summary>
    public TenantLifecycleWorkflowRegistry Register(ITenantLifecycleWorkflow workflow)
    {
        ArgumentNullException.ThrowIfNull(workflow);
        _workflows[workflow.Name] = workflow;
        return this;
    }

    /// <inheritdoc />
    public bool TryGet(TenantLifecycleWorkflowName name, out ITenantLifecycleWorkflow? workflow)
    {
        if (_workflows.TryGetValue(name, out var found))
        {
            workflow = found;
            return true;
        }
        workflow = null;
        return false;
    }
}
