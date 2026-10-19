using Hangfire;
using Hangfire.Client;
using Hangfire.Common;
using Hangfire.Logging;
using Microsoft.Extensions.DependencyInjection;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// Client filter that captures the ambient tenant/subject context through
/// the application-owned <see cref="IJobExecutionContext"/> bridge when a
/// job is created and stores it as a Hangfire job parameter. Jobs created
/// without an active context (for example scheduler-triggered recurring
/// jobs) carry no context parameter and run context-free.
/// </summary>
public sealed class JobContextCaptureFilter : JobFilterAttribute, IClientFilter
{
    /// <summary>The Hangfire job parameter name the snapshot is stored under.</summary>
    public const string JobParameterName = "Platform.JobContext";

    private static readonly ILog Logger = LogProvider.GetCurrentClassLogger();

    private readonly IServiceProvider _services;

    /// <summary>
    /// Initializes a new instance of the <see cref="JobContextCaptureFilter"/> class.
    /// </summary>
    /// <param name="services">The application service provider used to resolve the context bridge.</param>
    public JobContextCaptureFilter(IServiceProvider services)
    {
        ArgumentNullException.ThrowIfNull(services);
        _services = services;
    }

    /// <inheritdoc />
    public void OnCreating(CreatingContext context)
    {
        ArgumentNullException.ThrowIfNull(context);

        using var scope = _services.CreateScope();
        var bridge = scope.ServiceProvider.GetService<IJobExecutionContext>();
        if (bridge is null)
        {
            return;
        }

        var snapshot = bridge.Capture();
        if (snapshot is null)
        {
            return;
        }

        context.SetJobParameter(JobParameterName, snapshot);
        Logger.DebugFormat(
            "Captured tenant/subject context for job {0}.{1}.",
            context.Job?.Type.FullName,
            context.Job?.Method.Name);
    }

    /// <inheritdoc />
    public void OnCreated(CreatedContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
    }
}
