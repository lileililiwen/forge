using Microsoft.Extensions.DependencyInjection;
using Platform.Mailing;
using Platform.Mailing.SendGrid;
using Platform.Mailing.SendGrid.DependencyInjection;
using Platform.Mailing.Smtp;
using Platform.Mailing.Smtp.DependencyInjection;
using SendGrid;

namespace Platform.Mailing.ProviderAdapters.Tests;

public sealed class DependencyInjectionTests
{
    private sealed class StubMailService : IMailService
    {
        public Task<MailSendResult> SendAsync(MailMessage message, CancellationToken cancellationToken = default)
            => Task.FromResult(new MailSendResult(MailSendOutcome.Sent));
    }

    [Fact]
    public void AddPlatformSmtpMail_registers_the_adapter_as_the_mail_service()
    {
        var services = new ServiceCollection();
        services.AddPlatformSmtpMail(options => options.Host = "smtp.example.test");

        using var provider = services.BuildServiceProvider();

        Assert.IsType<SmtpMailService>(provider.GetRequiredService<IMailService>());
    }

    [Fact]
    public void AddPlatformSmtpMail_does_not_overwrite_a_consumer_registered_mail_service()
    {
        var services = new ServiceCollection();
        services.AddSingleton<IMailService, StubMailService>();
        services.AddPlatformSmtpMail(options => options.Host = "smtp.example.test");

        using var provider = services.BuildServiceProvider();

        Assert.IsType<StubMailService>(provider.GetRequiredService<IMailService>());
    }

    [Fact]
    public void AddPlatformSmtpMail_validates_configuration_at_registration()
    {
        var services = new ServiceCollection();

        Assert.Throws<ArgumentException>(() => services.AddPlatformSmtpMail(options => options.Host = " "));
    }

    [Fact]
    public void AddPlatformSendGridMail_registers_the_adapter_and_a_default_client()
    {
        var services = new ServiceCollection();
        services.AddPlatformSendGridMail(options => options.ApiKey = "SG.test-key");

        using var provider = services.BuildServiceProvider();

        Assert.IsType<SendGridMailService>(provider.GetRequiredService<IMailService>());
        Assert.NotNull(provider.GetRequiredService<ISendGridClient>());
    }

    [Fact]
    public void AddPlatformSendGridMail_does_not_overwrite_consumer_registrations()
    {
        var services = new ServiceCollection();
        services.AddSingleton<IMailService, StubMailService>();
        services.AddSingleton<ISendGridClient>(new FakeSendGridClient());
        services.AddPlatformSendGridMail(options => options.ApiKey = "SG.test-key");

        using var provider = services.BuildServiceProvider();

        Assert.IsType<StubMailService>(provider.GetRequiredService<IMailService>());
        Assert.IsType<FakeSendGridClient>(provider.GetRequiredService<ISendGridClient>());
    }

    [Fact]
    public void AddPlatformSendGridMail_validates_configuration_at_registration()
    {
        var services = new ServiceCollection();

        Assert.Throws<ArgumentException>(() => services.AddPlatformSendGridMail(options => options.ApiKey = " "));
    }
}
