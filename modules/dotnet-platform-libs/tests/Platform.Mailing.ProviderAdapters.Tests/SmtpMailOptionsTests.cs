using Microsoft.Extensions.Options;
using Platform.Mailing.Smtp;

namespace Platform.Mailing.ProviderAdapters.Tests;

public sealed class SmtpMailOptionsTests
{
    [Fact]
    public void Defaults_are_safe()
    {
        var options = new SmtpMailOptions
        {
            Host = "smtp.example.test",
        };

        Assert.Equal(587, options.Port);
        Assert.Equal(SmtpSecureMode.StartTls, options.SecureMode);
        Assert.Equal(TimeSpan.FromSeconds(30), options.OperationTimeout);
        options.Validate();
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    public void Host_is_required(string host)
    {
        var options = new SmtpMailOptions { Host = host };

        Assert.Throws<ArgumentException>(options.Validate);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(65536)]
    [InlineData(-1)]
    public void Port_must_be_in_range(int port)
    {
        var options = new SmtpMailOptions { Host = "smtp.example.test", Port = port };

        Assert.Throws<ArgumentOutOfRangeException>(options.Validate);
    }

    [Fact]
    public void Operation_timeout_must_be_bounded()
    {
        var tooFast = new SmtpMailOptions { Host = "smtp.example.test", OperationTimeout = TimeSpan.FromMilliseconds(500) };
        var tooSlow = new SmtpMailOptions { Host = "smtp.example.test", OperationTimeout = TimeSpan.FromMinutes(6) };

        Assert.Throws<ArgumentOutOfRangeException>(tooFast.Validate);
        Assert.Throws<ArgumentOutOfRangeException>(tooSlow.Validate);
    }

    [Fact]
    public void Credentials_must_be_configured_together()
    {
        var options = new SmtpMailOptions { Host = "smtp.example.test", UserName = "user" };

        Assert.Throws<ArgumentException>(options.Validate);
    }

    [Fact]
    public void Validation_error_messages_never_carry_secrets()
    {
        var options = new SmtpMailOptions { Host = "smtp.example.test", UserName = "smtp-user-secret" };

        try
        {
            options.Validate();
        }
        catch (ArgumentException exception)
        {
            Assert.DoesNotContain("smtp-user-secret", exception.Message, StringComparison.Ordinal);
            return;
        }

        Assert.Fail("The options validation was expected to fail.");
    }

    [Fact]
    public void Service_validates_options_at_construction()
    {
        Assert.Throws<ArgumentException>(() => new SmtpMailService(Options.Create(new SmtpMailOptions())));
    }
}
