using Microsoft.Extensions.DependencyInjection;
using Platform.Identity.AspNetCore;
using Platform.Identity.Contracts;
using Platform.Starter;
using Platform.Starter.DependencyInjection;
using Platform.Web;

namespace Platform.Starter.Tests;

public sealed class StarterOptionsTests
{
    [Fact]
    public void Web_only_registration_does_not_add_optional_capabilities()
    {
        var services = new ServiceCollection();

        services.AddPlatformApplication();

        using var provider = services.BuildServiceProvider();
        Assert.NotNull(provider.GetService<IPlatformApplicationStatus>());
        Assert.NotNull(provider.GetService<IPlatformConfigurationValidator>());
        Assert.Null(provider.GetService<ICurrentUserAccessor>());
        Assert.DoesNotContain(services, descriptor => descriptor.ServiceType.FullName?.Contains("Billing", StringComparison.Ordinal) == true);
        Assert.DoesNotContain(services, descriptor => descriptor.ServiceType.FullName?.Contains("Mail", StringComparison.Ordinal) == true);
    }

    [Fact]
    public void Explicit_capabilities_register_only_their_platform_composition()
    {
        var services = new ServiceCollection();

        services.AddPlatformApplication(options =>
        {
            options.EnableIdentity = true;
            options.EnableAdmin = true;
            options.EnableNotifications = true;
            options.NotificationProviderName = "fake";
        });

        using var provider = services.BuildServiceProvider();
        Assert.NotNull(provider.GetService<ICurrentUserAccessor>());
        Assert.NotNull(provider.GetService<Platform.Admin.Contracts.IAdminTenantScope>());
        Assert.Contains(services, descriptor => descriptor.ServiceType == typeof(Microsoft.Extensions.Options.IConfigureOptions<Platform.Mailing.MailingOptions>));
        Assert.Equal(new[] { "web", "identity", "admin", "notifications" }, provider.GetRequiredService<IPlatformApplicationStatus>().EnabledCapabilities);
    }

    [Fact]
    public void Production_enabled_provider_without_name_fails_validation()
    {
        var services = new ServiceCollection();
        services.AddPlatformApplication(options =>
        {
            options.EnvironmentName = "Production";
            options.EnableBilling = true;
        });

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<Microsoft.Extensions.Options.IOptions<PlatformApplicationOptions>>();

        Assert.Throws<Microsoft.Extensions.Options.OptionsValidationException>(() => _ = options.Value);
    }
}
