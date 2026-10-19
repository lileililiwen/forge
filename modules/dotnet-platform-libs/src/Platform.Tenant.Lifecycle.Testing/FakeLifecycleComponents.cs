using System.Collections.Concurrent;
using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenant.Lifecycle.Testing;

/// <summary>Scripted step that returns a pre-configured outcome. The script is consumed in order; subsequent calls run the supplied result or fail with the configured default.</summary>
public sealed class ScriptedLifecycleStep : ITenantLifecycleStep
{
    private readonly ConcurrentQueue<TenantLifecycleStepResult> _scripted = new();
    private readonly TenantLifecycleStepResult _defaultResult;

    /// <summary>Creates a new scripted step.</summary>
    public ScriptedLifecycleStep(TenantLifecycleStepName name, int order, bool isTenantScoped, TenantLifecycleStepResult defaultResult)
    {
        Name = name;
        Order = order;
        IsTenantScoped = isTenantScoped;
        _defaultResult = defaultResult;
    }

    /// <inheritdoc />
    public TenantLifecycleStepName Name { get; }
    /// <inheritdoc />
    public int Order { get; }
    /// <inheritdoc />
    public bool IsTenantScoped { get; }

    /// <summary>Records every invocation.</summary>
    public ConcurrentBag<TenantLifecycleStepContext> Invocations { get; } = new();

    /// <summary>Enqueues a scripted result.</summary>
    public void Enqueue(TenantLifecycleStepResult result) => _scripted.Enqueue(result);

    /// <inheritdoc />
    public ValueTask<TenantLifecycleStepResult> ExecuteAsync(TenantLifecycleStepContext context, CancellationToken cancellationToken = default)
    {
        Invocations.Add(context);
        if (_scripted.TryDequeue(out var scripted))
            return ValueTask.FromResult(scripted);
        return ValueTask.FromResult(_defaultResult);
    }
}

/// <summary>Delegate-based step. Useful when tests need to assert on the context before returning a result.</summary>
public sealed class DelegateLifecycleStep : ITenantLifecycleStep
{
    private readonly Func<TenantLifecycleStepContext, CancellationToken, ValueTask<TenantLifecycleStepResult>> _action;

    /// <summary>Creates a new delegate step.</summary>
    public DelegateLifecycleStep(TenantLifecycleStepName name, int order, bool isTenantScoped, Func<TenantLifecycleStepContext, CancellationToken, ValueTask<TenantLifecycleStepResult>> action)
    {
        Name = name;
        Order = order;
        IsTenantScoped = isTenantScoped;
        _action = action;
    }

    /// <inheritdoc />
    public TenantLifecycleStepName Name { get; }
    /// <inheritdoc />
    public int Order { get; }
    /// <inheritdoc />
    public bool IsTenantScoped { get; }

    /// <inheritdoc />
    public ValueTask<TenantLifecycleStepResult> ExecuteAsync(TenantLifecycleStepContext context, CancellationToken cancellationToken = default) => _action(context, cancellationToken);
}

/// <summary>Composes a workflow from a list of steps in declaration order.</summary>
public sealed class StaticLifecycleWorkflow : ITenantLifecycleWorkflow
{
    /// <summary>Creates a new workflow.</summary>
    public StaticLifecycleWorkflow(TenantLifecycleWorkflowName name, IReadOnlyList<ITenantLifecycleStep> steps)
    {
        Name = name;
        Steps = steps;
    }

    /// <inheritdoc />
    public TenantLifecycleWorkflowName Name { get; }
    /// <inheritdoc />
    public IReadOnlyList<ITenantLifecycleStep> Steps { get; }
}

/// <summary>Records the tenant id of every scope the orchestrator installs.</summary>
public sealed class RecordingLifecycleScopeCallback : ITenantLifecycleScopeCallback
{
    /// <summary>The tenant ids passed to <see cref="BeginTenantScope"/>, in invocation order.</summary>
    public ConcurrentBag<string> TenantIds { get; } = new();
    /// <summary>Number of currently-active scopes.</summary>
    public int ActiveScopes => _active;
    private int _active;

    /// <inheritdoc />
    public IDisposable BeginTenantScope(string tenantId)
    {
        TenantIds.Add(tenantId);
        Interlocked.Increment(ref _active);
        return new Restorer(this);
    }

    private sealed class Restorer : IDisposable
    {
        private readonly RecordingLifecycleScopeCallback _owner;
        public Restorer(RecordingLifecycleScopeCallback owner) { _owner = owner; }
        public void Dispose() => Interlocked.Decrement(ref _owner._active);
    }
}
