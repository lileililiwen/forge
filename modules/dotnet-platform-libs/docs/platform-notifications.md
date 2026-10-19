# Notifications and SMS

`Platform.Notifications` provides channel-neutral `NotificationIntent` values for email and
SMS, normalized outcomes/failure categories, bounded transient retries, and stable
idempotency-key suppression. Email adapts to the existing application-owned `IMailService`;
SMS uses the replaceable `ISmsSender` contract.

```csharp
builder.Services.AddPlatformNotifications(options =>
{
    options.EnvironmentName = builder.Environment.EnvironmentName;
    options.MaxAttempts = 3;
});
builder.Services.AddSingleton<IMailService, ApplicationMailService>();
builder.Services.AddSingleton<ISmsSender, ApplicationSmsProvider>();
```

`AddPlatformNotifications` registers orchestration, mailing/job/idempotency integration, and
no delivery provider. `AddPlatformNotificationsDevelopment` is intentionally in
`Platform.Notifications.Testing`; it registers the deterministic in-memory provider and records
attempts. Production without a provider returns a configuration failure and never an accepted
result.

Applications may render a `MailTemplateId` through their own template engine before creating an
intent. Scheduled delivery can use `NotificationScheduling.EnqueueAsync` with the application’s
`IJobDispatcher`; event handlers should pass the event’s stable identifier as `IdempotencyKey`.
Provider adapters classify invalid destinations, configuration, authentication, transient, and
permanent failures without exposing secrets or provider response bodies.
