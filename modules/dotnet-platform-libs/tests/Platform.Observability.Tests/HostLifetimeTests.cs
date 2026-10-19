using System.Diagnostics;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Platform.Observability;
using Platform.Observability.DependencyInjection;
using Platform.Observability.Diagnostics;
using Platform.Observability.Hosting;

namespace Platform.Observability.Tests;

public sealed class HostLifetimeTests
{
    [Fact]
    public async Task Host_lifecycle_emits_startup_activity()
    {
        using var listener = new HostListener();
        ActivitySource.AddActivityListener(listener.Listener);
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformObservability();
        await using var sp = services.BuildServiceProvider();

        var lifetime = ActivatorUtilities.CreateInstance<PlatformObservabilityHostLifetime>(sp,
            sp.GetRequiredService<Platform.Observability.Diagnostics.IPlatformActivityRecorder>(),
            sp.GetRequiredService<Microsoft.Extensions.Options.IOptions<PlatformObservabilityOptions>>(),
            sp.GetRequiredService<ILogger<PlatformObservabilityHostLifetime>>());

        await lifetime.StartingAsync(CancellationToken.None);
        await lifetime.StoppedAsync(CancellationToken.None);

        Assert.Contains(listener.Activities, a => a.OperationName == PlatformObservabilityNames.HostStartupOperation);
        Assert.Contains(listener.Activities, a => a.OperationName == PlatformObservabilityNames.HostShutdownOperation);
    }

    [Fact]
    public async Task Disabled_lifecycle_does_not_emit_activity()
    {
        using var listener = new HostListener();
        ActivitySource.AddActivityListener(listener.Listener);
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformObservability(o => o.EnableHostLifecycle = false);
        await using var sp = services.BuildServiceProvider();

        var lifetime = ActivatorUtilities.CreateInstance<PlatformObservabilityHostLifetime>(sp,
            sp.GetRequiredService<Platform.Observability.Diagnostics.IPlatformActivityRecorder>(),
            sp.GetRequiredService<Microsoft.Extensions.Options.IOptions<PlatformObservabilityOptions>>(),
            sp.GetRequiredService<ILogger<PlatformObservabilityHostLifetime>>());

        await lifetime.StartingAsync(CancellationToken.None);
        Assert.DoesNotContain(listener.Activities, a => a.OperationName == PlatformObservabilityNames.HostStartupOperation);
    }

    private sealed class HostListener : IDisposable
    {
        private readonly List<System.Diagnostics.Activity> _activities = new();
        public System.Diagnostics.ActivityListener Listener { get; }
        public IReadOnlyList<System.Diagnostics.Activity> Activities => _activities;
        public HostListener()
        {
            Listener = new System.Diagnostics.ActivityListener
            {
                ShouldListenTo = source => source.Name == PlatformObservabilityNames.HostActivitySource,
                Sample = (ref System.Diagnostics.ActivityCreationOptions<System.Diagnostics.ActivityContext> _) => System.Diagnostics.ActivitySamplingResult.AllData,
                ActivityStarted = activity => _activities.Add(activity),
            };
            System.Diagnostics.ActivitySource.AddActivityListener(Listener);
        }
        public void Dispose() => Listener.Dispose();
    }
}
