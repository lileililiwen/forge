using System.Diagnostics;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Platform.Observability;
using Platform.Observability.Diagnostics;
using Platform.Observability.Redaction;

namespace Platform.Observability.Tests;

[Collection("ActivityRecorder")]
public sealed class ActivityRecorderTests
{
    [Fact]
    public void Disabled_options_skip_activity_creation()
    {
        var options = Options.Create(new PlatformObservabilityOptions
        {
            EnableHostLifecycle = false,
            EnableRequestEnrichment = false,
            EnableProviderEnrichment = false,
        });
        using var listener = new ActivityListener
        {
            ShouldListenTo = source => true,
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData,
        };
        ActivitySource.AddActivityListener(listener);

        var recorder = new DefaultPlatformActivityRecorder(options, new DefaultPlatformObservabilityRedactor());
        Assert.Null(recorder.StartHostLifecycle(PlatformObservabilityNames.HostStartupOperation));
        Assert.Null(recorder.StartRequestCorrelation(PlatformObservabilityNames.RequestCorrelationOperation, "/api", "GET"));
        Assert.Null(recorder.StartProviderCall(PlatformObservabilityNames.ProviderCallOperation, "test-provider"));
    }

    [Fact]
    public void StartProviderCall_emits_bounded_outcome_and_redacts_error_code()
    {
        var options = Options.Create(new PlatformObservabilityOptions());
        using var listener = new CapturingListener();
        ActivitySource.AddActivityListener(listener.Listener);

        var recorder = new DefaultPlatformActivityRecorder(options, new DefaultPlatformObservabilityRedactor());
        using var activity = recorder.StartProviderCall(PlatformObservabilityNames.ProviderCallOperation, "github");
        Assert.NotNull(activity);
        recorder.Complete(activity, PlatformObservabilityNames.OutcomeTransient, "rate_limited", durationMs: 12.5);

        var captured = listener.Activities.Single(a => a.Source.Name == PlatformObservabilityNames.ProviderActivitySource);
        Assert.Equal(PlatformObservabilityNames.ProviderCallOperation, captured.OperationName);
        var tag = captured.TagObjects.ToDictionary(t => (string)t.Key, t => t.Value);
        Assert.Equal("[REDACTED]", tag[PlatformObservabilityNames.TagProvider]);
        Assert.Equal(PlatformObservabilityNames.OutcomeTransient, tag[PlatformObservabilityNames.TagOutcome]);
        Assert.Equal("[REDACTED]", tag[PlatformObservabilityNames.TagErrorCode]);
        Assert.Equal(ActivityStatusCode.Error, captured.Status);
    }

    [Fact]
    public void Complete_with_success_marks_activity_ok()
    {
        var options = Options.Create(new PlatformObservabilityOptions());
        using var listener = new CapturingListener();
        ActivitySource.AddActivityListener(listener.Listener);

        var recorder = new DefaultPlatformActivityRecorder(options, new DefaultPlatformObservabilityRedactor());
        using var activity = recorder.StartHostLifecycle(PlatformObservabilityNames.HostStartupOperation);
        Assert.NotNull(activity);
        recorder.Complete(activity, PlatformObservabilityNames.OutcomeSuccess);

        var captured = listener.Activities.Single(a => a.Source.Name == PlatformObservabilityNames.HostActivitySource);
        Assert.Equal(ActivityStatusCode.Ok, captured.Status);
    }

    private sealed class CapturingListener : IDisposable
    {
        private readonly List<Activity> _activities = new();
        public ActivityListener Listener { get; }
        public IReadOnlyList<Activity> Activities => _activities;
        public CapturingListener()
        {
            Listener = new ActivityListener
            {
                ShouldListenTo = source => source.Name == PlatformObservabilityNames.HostActivitySource
                    || source.Name == PlatformObservabilityNames.RequestActivitySource
                    || source.Name == PlatformObservabilityNames.ProviderActivitySource,
                Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData,
                ActivityStarted = activity => _activities.Add(activity),
            };
            ActivitySource.AddActivityListener(Listener);
        }
        public void Dispose() => Listener.Dispose();
    }
}
