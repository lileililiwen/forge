using System.Text;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Platform.Eventing.Contracts;

namespace Platform.Eventing.RabbitMq.Tests;

public class RabbitMqDurableEventPublisherTests
{
    private static RabbitMqDurableEventPublisher BuildPublisher(
        FakeRabbitMqChannelFactory factory,
        IRabbitMqEventTopology? topology = null,
        Action<RabbitMqEventingOptions>? configure = null)
    {
        return new RabbitMqDurableEventPublisher(
            topology ?? new RabbitMqEventTopology().Map("order.created", new RabbitMqEventBinding("order.created.v1")),
            factory,
            TestOptions.Wrap(TestOptions.Create(configure)),
            NullLogger<RabbitMqDurableEventPublisher>.Instance);
    }

    [Fact]
    public async Task Confirmed_publish_uses_the_registered_topology_and_safe_properties()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory);
        var envelope = TestOptions.Envelope(tenantId: "tenant-1", correlationId: "corr-1");

        await publisher.PublishAsync(envelope);

        var channel = Assert.Single(factory.Created);
        var outbound = Assert.Single(channel.Published);
        Assert.Equal("platform.events", outbound.Exchange);
        Assert.Equal("order.created.v1", outbound.RoutingKey);
        Assert.Equal("msg-1", outbound.MessageId);
        Assert.Equal("order.created", outbound.PayloadType);
        Assert.Equal("tenant-1", outbound.TenantId);
        Assert.Equal("corr-1", outbound.CorrelationId);
        Assert.Equal("{\"orderId\":\"1\"}", Encoding.UTF8.GetString(outbound.Body.Span));
        Assert.Equal(RabbitMqEventingProviderState.Healthy, publisher.Status.State);
        Assert.Null(publisher.Status.LastErrorCode);
    }

    [Fact]
    public async Task Binding_exchange_override_wins_over_the_configured_exchange()
    {
        var factory = new FakeRabbitMqChannelFactory();
        var topology = new RabbitMqEventTopology().Map("order.created", new RabbitMqEventBinding("paid.v1", "billing.events"));
        await using var publisher = BuildPublisher(factory, topology);

        await publisher.PublishAsync(TestOptions.Envelope());

        var outbound = Assert.Single(Assert.Single(factory.Created).Published);
        Assert.Equal("billing.events", outbound.Exchange);
        Assert.Equal("paid.v1", outbound.RoutingKey);
    }

    [Fact]
    public async Task Unregistered_payload_type_fails_with_a_configuration_error_before_publishing()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory);

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope(payloadType: "unknown.event")));

        Assert.Equal("eventing.rabbitmq.unregistered_type", exception.Failure.Code);
        Assert.True(exception.Failure.Permanent);
        Assert.Equal(0, factory.CreateCalls);
        Assert.Equal(RabbitMqEventingProviderState.Healthy, publisher.Status.State);
    }

    [Fact]
    public async Task Unconfigured_topology_fails_with_a_configuration_error()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory, new UnconfiguredRabbitMqEventTopology());

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.Equal("eventing.rabbitmq.topology_unconfigured", exception.Failure.Code);
        Assert.True(exception.Failure.Permanent);
        Assert.Equal(0, factory.CreateCalls);
    }

    [Fact]
    public async Task Broker_publish_failure_is_transient_and_triggers_reconnection()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory);
        factory.OnCreate = _ =>
        {
            var channel = new FakeRabbitMqChannel();
            factory.Created.Add(channel);
            if (factory.Created.Count == 1)
            {
                channel.OnPublish = (_, _) => throw new InvalidOperationException("channel closed");
            }

            return Task.FromResult<IRabbitMqChannel>(channel);
        };

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.Equal("eventing.rabbitmq.publish_failed", exception.Failure.Code);
        Assert.False(exception.Failure.Permanent);
        Assert.Equal(RabbitMqEventingProviderState.Unavailable, publisher.Status.State);
        Assert.Equal("eventing.rabbitmq.publish_failed", publisher.Status.LastErrorCode);

        await publisher.PublishAsync(TestOptions.Envelope());

        Assert.Equal(2, factory.CreateCalls);
        Assert.Single(factory.Created[0].Published);
        Assert.Single(factory.Created[1].Published);
        Assert.Equal(1, factory.Created[0].DisposeCalls);
    }

    [Fact]
    public async Task Connect_failure_is_transient()
    {
        var factory = new FakeRabbitMqChannelFactory();
        factory.OnCreate = _ => throw new InvalidOperationException("connection refused");
        await using var publisher = BuildPublisher(factory);

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.Equal("eventing.rabbitmq.connect_failed", exception.Failure.Code);
        Assert.False(exception.Failure.Permanent);
    }

    [Fact]
    public async Task Connect_timeout_is_transient()
    {
        var factory = new FakeRabbitMqChannelFactory();
        factory.OnCreate = async ct =>
        {
            try
            {
                await Task.Delay(Timeout.InfiniteTimeSpan, ct);
            }
            catch (OperationCanceledException)
            {
            }

            throw new OperationCanceledException(ct);
        };
        await using var publisher = BuildPublisher(factory, configure: o => o.ConnectTimeout = TimeSpan.FromSeconds(1));

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.Equal("eventing.rabbitmq.connect_timeout", exception.Failure.Code);
        Assert.False(exception.Failure.Permanent);
    }

    [Fact]
    public async Task Confirm_timeout_is_transient()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory, configure: o => o.ConfirmTimeout = TimeSpan.FromMilliseconds(200));
        var channel = new FakeRabbitMqChannel
        {
            OnPublish = async (_, ct) =>
            {
                try
                {
                    await Task.Delay(Timeout.InfiniteTimeSpan, ct);
                }
                catch (OperationCanceledException)
                {
                }

                throw new OperationCanceledException(ct);
            },
        };
        factory.OnCreate = _ => Task.FromResult<IRabbitMqChannel>(channel);

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.Equal("eventing.rabbitmq.confirm_timeout", exception.Failure.Code);
        Assert.False(exception.Failure.Permanent);
    }

    [Fact]
    public async Task Caller_cancellation_during_publish_is_rethrown()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory);
        using var cts = new CancellationTokenSource();
        var channel = new FakeRabbitMqChannel
        {
            OnPublish = async (_, ct) =>
            {
                try
                {
                    await Task.Delay(Timeout.InfiniteTimeSpan, ct);
                }
                catch (OperationCanceledException)
                {
                }

                throw new OperationCanceledException(ct);
            },
        };
        factory.OnCreate = _ => Task.FromResult<IRabbitMqChannel>(channel);

        var cancelTask = Task.Run(async () =>
        {
            await Task.Delay(TimeSpan.FromMilliseconds(100));
            await cts.CancelAsync();
        });

        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => publisher.PublishAsync(TestOptions.Envelope(), cts.Token));
        await cancelTask;
    }

    [Fact]
    public async Task Pre_cancelled_caller_token_is_rethrown_without_publishing()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory);
        using var cts = new CancellationTokenSource();
        await cts.CancelAsync();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => publisher.PublishAsync(TestOptions.Envelope(), cts.Token));

        Assert.Equal(0, factory.CreateCalls);
    }

    [Fact]
    public async Task Exchange_is_declared_only_when_enabled()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var enabled = BuildPublisher(factory, configure: o => o.DeclareExchange = true);
        await enabled.PublishAsync(TestOptions.Envelope());
        Assert.Equal(1, Assert.Single(factory.Created).DeclareExchangeCalls);

        var disabledFactory = new FakeRabbitMqChannelFactory();
        await using var disabled = BuildPublisher(disabledFactory);
        await disabled.PublishAsync(TestOptions.Envelope());
        Assert.Equal(0, Assert.Single(disabledFactory.Created).DeclareExchangeCalls);
    }

    [Fact]
    public async Task Exchange_declare_failure_is_transient()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory, configure: o => o.DeclareExchange = true);
        var channel = new FakeRabbitMqChannel
        {
            OnDeclareExchange = _ => throw new InvalidOperationException("access refused"),
        };
        factory.OnCreate = _ => Task.FromResult<IRabbitMqChannel>(channel);

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.Equal("eventing.rabbitmq.connect_failed", exception.Failure.Code);
        Assert.False(exception.Failure.Permanent);
    }

    [Fact]
    public async Task Open_channel_is_reused_across_publishes()
    {
        var factory = new FakeRabbitMqChannelFactory();
        await using var publisher = BuildPublisher(factory);

        await publisher.PublishAsync(TestOptions.Envelope());
        await publisher.PublishAsync(TestOptions.Envelope(payloadJson: "{\"orderId\":\"2\"}"));

        Assert.Equal(1, factory.CreateCalls);
        Assert.Equal(2, Assert.Single(factory.Created).Published.Count);
    }

    [Fact]
    public async Task Invalid_binding_fails_with_a_configuration_error()
    {
        var factory = new FakeRabbitMqChannelFactory();
        var topology = new StubTopology(new RabbitMqEventBinding(""));
        await using var publisher = BuildPublisher(factory, topology);

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.Equal("eventing.rabbitmq.invalid_binding", exception.Failure.Code);
        Assert.True(exception.Failure.Permanent);
        Assert.Equal(0, factory.CreateCalls);
    }

    [Fact]
    public async Task Disposal_releases_the_open_channel()
    {
        var factory = new FakeRabbitMqChannelFactory();
        var publisher = BuildPublisher(factory);
        await publisher.PublishAsync(TestOptions.Envelope());

        await publisher.DisposeAsync();

        Assert.Equal(1, Assert.Single(factory.Created).DisposeCalls);
    }

    private sealed class StubTopology(RabbitMqEventBinding binding) : IRabbitMqEventTopology
    {
        public RabbitMqEventBinding? Resolve(string payloadType) => binding;
    }
}
