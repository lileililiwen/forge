using Microsoft.Extensions.Options;
using Platform.Auditing.Contracts;
using Platform.Auditing.Contracts.Common;

namespace Platform.Auditing.Tests.Contracts;

public class AuditRecorderTests
{
    [Fact]
    public async Task Fail_open_policy_drops_event_and_records_dead_letter_on_sink_failure()
    {
        var sink = new ThrowingAuditSink();
        var deadLetter = new InMemoryAuditDeadLetterSink();
        var recorder = BuildRecorder(new AuditOptions { FailurePolicy = AuditFailurePolicy.FailOpen }, sink, deadLetter);

        await recorder.RecordAsync(AuditEvent.Create("http.request", "http", AuditOutcome.Success, FixedUtc()));

        Assert.Empty(sink.Recorded);
        var failed = Assert.Single(deadLetter.GetFailed());
        Assert.Equal("http.request", failed.Event.Action);
        Assert.Equal("sink-failure", failed.Failure);
    }

    [Fact]
    public async Task Fail_closed_policy_surfaces_sink_failure()
    {
        var sink = new ThrowingAuditSink();
        var deadLetter = new InMemoryAuditDeadLetterSink();
        var recorder = BuildRecorder(new AuditOptions { FailurePolicy = AuditFailurePolicy.FailClosed }, sink, deadLetter);

        await Assert.ThrowsAsync<AuditRecordingException>(() =>
            recorder.RecordAsync(AuditEvent.Create("http.request", "http", AuditOutcome.Success, FixedUtc())));
        Assert.Empty(deadLetter.GetFailed());
    }

    [Fact]
    public async Task Recorder_masks_sensitive_metadata_before_dispatch()
    {
        var sink = new CapturingAuditSink();
        var recorder = BuildRecorder(new AuditOptions(), sink, new NoOpAuditDeadLetterSink());

        await recorder.RecordAsync(AuditEvent.Create("http.request", "http", AuditOutcome.Success, FixedUtc())
            .WithMetadata("user_password", "hunter2")
            .WithMetadata("username", "alice"));

        var recorded = Assert.Single(sink.Recorded);
        Assert.Equal(DefaultAuditMasker.Redacted, recorded.Metadata["user_password"]);
        Assert.Equal("alice", recorded.Metadata["username"]);
    }

    [Fact]
    public async Task Recorder_applies_enricher_metadata()
    {
        var sink = new CapturingAuditSink();
        var enricher = new StaticEnricher("enriched", "yes");
        var recorder = BuildRecorder(new AuditOptions(), sink, new NoOpAuditDeadLetterSink(), enricher);

        await recorder.RecordAsync(AuditEvent.Create("http.request", "http", AuditOutcome.Success, FixedUtc()));

        var recorded = Assert.Single(sink.Recorded);
        Assert.Equal("yes", recorded.Metadata["enriched"]);
    }

    [Fact]
    public async Task Recorder_skips_excluded_categories()
    {
        var sink = new CapturingAuditSink();
        var recorder = BuildRecorder(new AuditOptions { ExcludedCategories = new List<string> { "security" } }, sink, new NoOpAuditDeadLetterSink());

        await recorder.RecordAsync(AuditEvent.Create("authorization.denied", "security", AuditOutcome.Denied, FixedUtc()));

        Assert.Empty(sink.Recorded);
    }

    [Fact]
    public async Task Bounded_async_mode_dispatches_and_flushes()
    {
        var sink = new CapturingAuditSink();
        var recorder = BuildRecorder(new AuditOptions { PublishMode = AuditPublishMode.BoundedAsync, BoundedCapacity = 16 }, sink, new NoOpAuditDeadLetterSink());

        await recorder.RecordAsync(AuditEvent.Create("http.request", "http", AuditOutcome.Success, FixedUtc()));
        await recorder.FlushAsync();

        Assert.Single(sink.Recorded);
    }

    [Fact]
    public async Task Metadata_is_bounded_to_configured_entry_count()
    {
        var sink = new CapturingAuditSink();
        var recorder = BuildRecorder(new AuditOptions { MaxMetadataEntries = 2 }, sink, new NoOpAuditDeadLetterSink());

        var auditEvent = AuditEvent.Create("http.request", "http", AuditOutcome.Success, FixedUtc())
            .WithMetadata("a", "1").WithMetadata("b", "2").WithMetadata("c", "3");
        await recorder.RecordAsync(auditEvent);

        var recorded = Assert.Single(sink.Recorded);
        Assert.True(recorded.Metadata.Count <= 2);
    }

    private static DefaultAuditRecorder BuildRecorder(
        AuditOptions options,
        IAuditSink sink,
        IAuditDeadLetterSink deadLetter,
        params IAuditEnricher[] enrichers) =>
        new(Options.Create(options), enrichers, new DefaultAuditMasker(), sink, deadLetter);

    private static DateTimeOffset FixedUtc() => new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
}
