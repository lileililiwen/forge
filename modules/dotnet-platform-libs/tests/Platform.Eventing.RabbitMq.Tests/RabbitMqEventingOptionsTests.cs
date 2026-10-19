using Microsoft.Extensions.Options;

namespace Platform.Eventing.RabbitMq.Tests;

public class RabbitMqEventingOptionsTests
{
    [Fact]
    public void Defaults_are_safe_except_the_exchange_is_required()
    {
        var options = new RabbitMqEventingOptions();

        Assert.Equal("localhost", options.HostName);
        Assert.Equal(5672, options.Port);
        Assert.Equal("/", options.VirtualHost);
        Assert.Equal("guest", options.UserName);
        Assert.Equal(string.Empty, options.Password);
        Assert.False(options.UseSsl);
        Assert.Equal("topic", options.ExchangeType);
        Assert.False(options.DeclareExchange);
        Assert.True(options.DurableExchange);
        Assert.Equal(TimeSpan.FromSeconds(10), options.ConnectTimeout);
        Assert.Equal(TimeSpan.FromSeconds(5), options.ConfirmTimeout);
        Assert.Equal("Eventing:RabbitMq", RabbitMqEventingOptions.SectionName);

        var exception = Assert.Throws<ArgumentException>(options.Validate);
        Assert.Contains("exchange is required", exception.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Valid_configuration_passes_validation()
    {
        var options = TestOptions.Create();

        options.Validate();
    }

    [Theory]
    [InlineData(0)]
    [InlineData(65536)]
    [InlineData(-1)]
    public void Port_must_be_bounded(int port)
    {
        var options = TestOptions.Create(o => o.Port = port);

        Assert.Throws<ArgumentOutOfRangeException>(options.Validate);
    }

    [Theory]
    [InlineData(999)]
    [InlineData(301)]
    public void Connect_timeout_must_be_bounded(int seconds)
    {
        var options = TestOptions.Create(o => o.ConnectTimeout = TimeSpan.FromSeconds(seconds));

        Assert.Throws<ArgumentOutOfRangeException>(options.Validate);
    }

    [Fact]
    public void Confirm_timeout_must_be_bounded()
    {
        var options = TestOptions.Create(o => o.ConfirmTimeout = TimeSpan.FromMilliseconds(50));

        Assert.Throws<ArgumentOutOfRangeException>(options.Validate);
    }

    [Fact]
    public void Validation_error_messages_never_carry_secrets()
    {
        var secret = "super-secret-password";
        var options = TestOptions.Create(o =>
        {
            o.Password = secret;
            o.Port = 0;
        });

        var exception = Assert.Throws<ArgumentOutOfRangeException>(options.Validate);

        Assert.DoesNotContain(secret, exception.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Control_characters_are_rejected()
    {
        var options = TestOptions.Create(o => o.Exchange = "platform\u0000events");

        Assert.Throws<ArgumentException>(options.Validate);
    }
}

public class RabbitMqEventTopologyTests
{
    [Fact]
    public void Map_and_resolve_round_trip()
    {
        var topology = new RabbitMqEventTopology()
            .Map("order.created", new RabbitMqEventBinding("order.created"))
            .Map("order.paid", new RabbitMqEventBinding("paid.v1", "billing.events"));

        Assert.Equal(new RabbitMqEventBinding("order.created"), topology.Resolve("order.created"));
        Assert.Equal(new RabbitMqEventBinding("paid.v1", "billing.events"), topology.Resolve("order.paid"));
        Assert.Null(topology.Resolve("unknown"));
    }

    [Fact]
    public void Unconfigured_topology_resolves_nothing()
    {
        Assert.Null(new UnconfiguredRabbitMqEventTopology().Resolve("order.created"));
    }

    [Fact]
    public void Map_rejects_invalid_payload_types_and_bindings()
    {
        var topology = new RabbitMqEventTopology();

        Assert.Throws<ArgumentException>(() => topology.Map("", new RabbitMqEventBinding("key")));
        Assert.Throws<ArgumentException>(() => topology.Map("bad\u0000type", new RabbitMqEventBinding("key")));
        Assert.Throws<ArgumentNullException>(() => topology.Map("order.created", null!));
        Assert.Throws<ArgumentException>(() => topology.Map("order.created", new RabbitMqEventBinding("")));
    }

    [Fact]
    public void Binding_validate_rejects_invalid_values()
    {
        Assert.Throws<ArgumentException>(() => new RabbitMqEventBinding("key", "").Validate());
        Assert.Throws<ArgumentException>(() => new RabbitMqEventBinding("bad\u0000key").Validate());
        new RabbitMqEventBinding("key", "exchange").Validate();
    }
}
