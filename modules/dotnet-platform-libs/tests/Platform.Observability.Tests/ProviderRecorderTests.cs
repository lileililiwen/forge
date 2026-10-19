using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Observability;
using Platform.Observability.Diagnostics;
using Platform.Observability.Redaction;

namespace Platform.Observability.Tests;

public sealed class ProviderRecorderTests
{
    [Fact]
    public void RecordCall_increments_provider_call_counter()
    {
        var options = Options.Create(new PlatformObservabilityOptions());
        var redactor = new DefaultPlatformObservabilityRedactor();
        var activity = new DefaultPlatformActivityRecorder(options, redactor);
        var recorder = new DefaultPlatformObservabilityProviderRecorder(activity, options, redactor);

        var beforeCount = PlatformDiagnostics.RecordProviderCall(PlatformObservabilityNames.OutcomeSuccess);
        recorder.RecordCall(new PlatformObservabilityProviderCall("http.fetch", "github", PlatformObservabilityNames.OutcomeSuccess, TimeSpan.FromMilliseconds(5)));
        recorder.RecordCall(new PlatformObservabilityProviderCall("http.fetch", "github", PlatformObservabilityNames.OutcomeTransient, TimeSpan.FromMilliseconds(20), "rate_limited"));

        var afterCount = PlatformDiagnostics.RecordProviderCall(PlatformObservabilityNames.OutcomeSuccess);
        Assert.True(afterCount >= beforeCount + 2);
        Assert.True(PlatformDiagnostics.LastProviderCallDurationMs >= 20);
    }

    [Fact]
    public void Provider_status_source_does_not_emit_secrets()
    {
        var source = new DefaultPlatformObservabilityProviderStatusSource();
        var statuses = source.GetStatuses();
        Assert.All(statuses, s =>
        {
            Assert.DoesNotContain("password", s.Name, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("secret", s.Name, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("token", s.Name, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("account", s.Name, StringComparison.OrdinalIgnoreCase);
        });
    }
}
