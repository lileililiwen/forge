using Platform.Eventing;
using Platform.Testing.Eventing;
using Platform.Testing.FailureInjection;

namespace Platform.Testing.Tests;

public sealed class RecordingEventBusTests
{
    [Fact]
    public async Task PublishAsync_records_envelope_in_invocation_order()
    {
        var bus = new RecordingEventBus();
        var first = NewEnvelope("first", correlationId: "c-1");
        var second = NewEnvelope("second", correlationId: "c-1");

        await bus.PublishAsync(first);
        await bus.PublishAsync(second);

        Assert.Equal(new[] { first, second }, bus.Envelopes);
    }

    [Fact]
    public async Task EnvelopesOfType_filters_by_payload_type()
    {
        var bus = new RecordingEventBus();
        var first = NewEnvelope("first");
        var second = NewEnvelope("second");

        await bus.PublishAsync(first);
        await bus.PublishAsync(second);

        var matches = bus.EnvelopesOfType("second");
        Assert.Equal(new[] { second }, matches);
    }

    [Fact]
    public async Task Reset_clears_recorded_envelopes()
    {
        var bus = new RecordingEventBus();
        await bus.PublishAsync(NewEnvelope("first"));
        bus.Reset();
        Assert.Empty(bus.Envelopes);
    }

    [Fact]
    public async Task PublishAsync_throws_when_envelope_is_null()
    {
        var bus = new RecordingEventBus();
        await Assert.ThrowsAsync<ArgumentNullException>(() => bus.PublishAsync(null!));
    }

    [Fact]
    public async Task PublishAsync_respects_cancellation()
    {
        var bus = new RecordingEventBus();
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        await Assert.ThrowsAsync<OperationCanceledException>(() => bus.PublishAsync(NewEnvelope("cancelled"), cts.Token));
    }

    private static IntegrationEventEnvelope NewEnvelope(string payloadType, string? correlationId = null) =>
        new(
            MessageId: Guid.NewGuid().ToString("N"),
            PayloadType: payloadType,
            PayloadJson: "{}",
            OccurredAt: DateTimeOffset.UnixEpoch,
            CorrelationId: correlationId);
}

public sealed class TransientFailureInjectorTests
{
    [Fact]
    public void Transient_injection_throws_once_then_passes_through()
    {
        var injector = new TransientFailureInjector()
            .WithTransient("checkout", new InvalidOperationException("transient"));

        Assert.Throws<InvalidOperationException>(() => injector.Run("checkout", () => { }));
        var called = false;
        injector.Run("checkout", () => called = true);
        Assert.True(called);
    }

    [Fact]
    public void Permanent_injection_throws_until_Reset()
    {
        var injector = new TransientFailureInjector()
            .WithPermanent("checkout", new InvalidOperationException("permanent"));

        Assert.Throws<InvalidOperationException>(() => injector.Run("checkout", () => { }));
        Assert.Throws<InvalidOperationException>(() => injector.Run("checkout", () => { }));
        injector.Reset();
        var called = false;
        injector.Run("checkout", () => called = true);
        Assert.True(called);
    }

    [Fact]
    public async Task RunAsync_propagates_injection_through_async_delegate()
    {
        var injector = new TransientFailureInjector()
            .WithTransient("jobs.dispatch", new TimeoutException("transient"));

        await Assert.ThrowsAsync<TimeoutException>(() => injector.RunAsync("jobs.dispatch", () => Task.CompletedTask));
    }

    [Fact]
    public void History_records_every_emission_in_order()
    {
        var injector = new TransientFailureInjector()
            .WithTransient("a", new InvalidOperationException("a"))
            .WithTransient("b", new InvalidOperationException("b"));

        Assert.Throws<InvalidOperationException>(() => injector.Run("a", () => { }));
        Assert.Throws<InvalidOperationException>(() => injector.Run("b", () => { }));

        var history = injector.History;
        Assert.Equal(2, history.Count);
        Assert.Equal("a", history[0].Label);
        Assert.Equal(InjectedFailureKind.Transient, history[0].Kind);
        Assert.Equal(1, history[0].Sequence);
        Assert.Equal("b", history[1].Label);
        Assert.Equal(2, history[1].Sequence);
    }

    [Fact]
    public void Reset_clears_history_and_pending()
    {
        var injector = new TransientFailureInjector()
            .WithTransient("a", new InvalidOperationException("a"));

        Assert.Throws<InvalidOperationException>(() => injector.Run("a", () => { }));
        injector.Reset();
        Assert.Empty(injector.History);

        var called = false;
        injector.Run("a", () => called = true);
        Assert.True(called);
    }

    [Fact]
    public void Run_with_unknown_label_passes_through()
    {
        var injector = new TransientFailureInjector();
        var called = false;
        injector.Run("never-configured", () => called = true);
        Assert.True(called);
        Assert.Empty(injector.History);
    }
}
