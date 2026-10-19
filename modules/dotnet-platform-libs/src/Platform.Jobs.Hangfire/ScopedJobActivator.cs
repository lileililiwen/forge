using Hangfire;
using Hangfire.Server;
using Microsoft.Extensions.DependencyInjection;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// Hangfire job activator that creates a DI scope per job execution and
/// restores the captured tenant/subject context through the
/// application-owned <see cref="IJobExecutionContext"/> bridge before any
/// handler is resolved. The restoration and the DI scope are disposed when
/// the job scope ends, so ambient context never leaks between jobs.
/// </summary>
public sealed class ScopedJobActivator : JobActivator
{
    private readonly IServiceScopeFactory _scopeFactory;

    /// <summary>
    /// Initializes a new instance of the <see cref="ScopedJobActivator"/> class.
    /// </summary>
    /// <param name="scopeFactory">The scope factory used to create the per-job DI scope.</param>
    public ScopedJobActivator(IServiceScopeFactory scopeFactory)
    {
        ArgumentNullException.ThrowIfNull(scopeFactory);
        _scopeFactory = scopeFactory;
    }

    /// <inheritdoc />
    public override JobActivatorScope BeginScope(PerformContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        return new Scope(context, _scopeFactory.CreateScope());
    }

    private sealed class Scope : JobActivatorScope
    {
        private readonly IServiceScope _scope;
        private readonly IDisposable? _restoration;

        public Scope(PerformContext context, IServiceScope scope)
        {
            _scope = scope;
            _restoration = RestoreContext(context, scope.ServiceProvider);
        }

        public override object Resolve(Type type) =>
            ActivatorUtilities.GetServiceOrCreateInstance(_scope.ServiceProvider, type);

        public override void DisposeScope()
        {
            _restoration?.Dispose();
            _scope.Dispose();
        }

        private static IDisposable? RestoreContext(PerformContext context, IServiceProvider services)
        {
            var snapshot = context.GetJobParameter<JobContextSnapshot>(JobContextCaptureFilter.JobParameterName);
            if (snapshot is null)
            {
                return null;
            }

            var bridge = services.GetService<IJobExecutionContext>()
                ?? throw new InvalidOperationException(
                    "The job carries a captured tenant/subject context but no IJobExecutionContext is registered. Register the application-owned context bridge to restore job context.");

            return bridge.Restore(snapshot);
        }
    }
}
