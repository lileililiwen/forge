using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenant.Lifecycle;

/// <summary>Extensions for composing the default orchestrator with a workflow registry.</summary>
public static class TenantLifecycleOrchestratorExtensions
{
    /// <summary>Attaches a workflow registry so <see cref="ITenantLifecycleOrchestrator.ResumeAsync"/> can locate the workflow by name.</summary>
    public static TenantLifecycleOrchestrator WithWorkflowRegistry(this TenantLifecycleOrchestrator orchestrator, ITenantLifecycleWorkflowRegistry registry)
    {
        ArgumentNullException.ThrowIfNull(orchestrator);
        ArgumentNullException.ThrowIfNull(registry);
        orchestrator.AttachRegistry(registry);
        return orchestrator;
    }
}
