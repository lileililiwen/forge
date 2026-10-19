using Microsoft.Extensions.Options;
using Platform.Mailing.SendGrid;

namespace Platform.Mailing.ProviderAdapters.Tests;

public sealed class SendGridMailOptionsTests
{
    [Fact]
    public void Defaults_are_safe()
    {
        var options = new SendGridMailOptions { ApiKey = "SG.test-key" };

        Assert.Equal(TimeSpan.FromSeconds(30), options.OperationTimeout);
        options.Validate();
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    public void Api_key_is_required(string apiKey)
    {
        var options = new SendGridMailOptions { ApiKey = apiKey };

        Assert.Throws<ArgumentException>(options.Validate);
    }

    [Fact]
    public void Operation_timeout_must_be_bounded()
    {
        var tooFast = new SendGridMailOptions { ApiKey = "SG.test-key", OperationTimeout = TimeSpan.FromMilliseconds(500) };
        var tooSlow = new SendGridMailOptions { ApiKey = "SG.test-key", OperationTimeout = TimeSpan.FromMinutes(6) };

        Assert.Throws<ArgumentOutOfRangeException>(tooFast.Validate);
        Assert.Throws<ArgumentOutOfRangeException>(tooSlow.Validate);
    }

    [Fact]
    public void Validation_error_messages_never_carry_secrets()
    {
        var options = new SendGridMailOptions { ApiKey = "SG.secret-value", OperationTimeout = TimeSpan.Zero };

        try
        {
            options.Validate();
        }
        catch (ArgumentException exception)
        {
            Assert.DoesNotContain("SG.secret-value", exception.Message, StringComparison.Ordinal);
            return;
        }

        Assert.Fail("The options validation was expected to fail.");
    }

    [Fact]
    public void Service_validates_options_at_construction()
    {
        var client = new FakeSendGridClient();

        Assert.Throws<ArgumentException>(() => new SendGridMailService(client, Options.Create(new SendGridMailOptions())));
    }
}
