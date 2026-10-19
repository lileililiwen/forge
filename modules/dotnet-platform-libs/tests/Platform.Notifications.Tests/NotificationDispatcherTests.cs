using Microsoft.Extensions.DependencyInjection;
using Platform.Mailing;
using Platform.Notifications;
using Platform.Notifications.DependencyInjection;
using Platform.Notifications.Testing;

namespace Platform.Notifications.Tests;

public sealed class NotificationDispatcherTests
{
    [Fact]
    public async Task Development_fake_delivers_email_and_sms()
    {
        var services = new ServiceCollection();
        services.AddPlatformNotificationsDevelopment();
        using var provider = services.BuildServiceProvider();
        var dispatcher = provider.GetRequiredService<INotificationDispatcher>();

        var email = await dispatcher.SendAsync(NotificationIntent.Email("welcome", "person@example.test", "Welcome", "Hello"));
        var sms = await dispatcher.SendAsync(NotificationIntent.Sms("otp", "+8613800138000", "123456"));

        Assert.Equal(NotificationDeliveryOutcome.Accepted, email.Outcome);
        Assert.Equal(NotificationDeliveryOutcome.Accepted, sms.Outcome);
        Assert.Equal(2, provider.GetRequiredService<InMemoryNotificationProvider>().Attempts.Count);
    }

    [Fact]
    public async Task Same_idempotency_key_is_delivered_once()
    {
        var fake = new InMemoryNotificationProvider();
        var services = new ServiceCollection();
        services.AddSingleton(fake);
        services.AddSingleton<IMailService>(fake);
        services.AddPlatformNotifications();
        using var provider = services.BuildServiceProvider();
        var dispatcher = provider.GetRequiredService<INotificationDispatcher>();
        var intent = NotificationIntent.Email("reminder", "person@example.test", "Reminder", "Hello") with { IdempotencyKey = "reminder-1" };

        var first = await dispatcher.SendAsync(intent);
        var second = await dispatcher.SendAsync(intent);

        Assert.Equal(NotificationDeliveryOutcome.Accepted, first.Outcome);
        Assert.Equal(NotificationDeliveryOutcome.Duplicate, second.Outcome);
        Assert.Single(fake.Attempts);
    }

    [Fact]
    public async Task Transient_failures_are_retried_within_the_configured_bound()
    {
        var fake = new InMemoryNotificationProvider { TransientFailuresRemaining = 2 };
        var services = new ServiceCollection();
        services.AddSingleton(fake);
        services.AddSingleton<IMailService>(fake);
        services.AddPlatformNotifications(options => options.MaxAttempts = 3);
        using var provider = services.BuildServiceProvider();

        var result = await provider.GetRequiredService<INotificationDispatcher>().SendAsync(
            NotificationIntent.Email("retry", "person@example.test", "Retry", "Hello"));

        Assert.Equal(NotificationDeliveryOutcome.Accepted, result.Outcome);
        Assert.Equal(3, result.Attempts);
        Assert.Equal(3, fake.Attempts.Count);
    }

    [Fact]
    public async Task Production_without_provider_returns_configuration_failure()
    {
        var services = new ServiceCollection();
        services.AddPlatformNotifications(options => options.EnvironmentName = "Production");
        using var provider = services.BuildServiceProvider();

        var result = await provider.GetRequiredService<INotificationDispatcher>().SendAsync(
            NotificationIntent.Sms("alert", "+8613800138000", "Alert"));

        Assert.Equal(NotificationFailureCategory.Configuration, result.Failure?.Category);
        Assert.NotEqual(NotificationDeliveryOutcome.Accepted, result.Outcome);
    }

    [Fact]
    public void Secret_like_values_are_not_in_failure_details()
    {
        var failure = NotificationDeliveryResult.Failed(NotificationFailureCategory.Authentication, "provider_auth_failed", "provider-secret");

        Assert.DoesNotContain("provider-secret", failure.Failure?.Message, StringComparison.Ordinal);
    }
}
