# Platform mailing providers

Optional SMTP and SendGrid adapters over the provider-neutral `Platform.Mailing`
contracts. The base `Platform.Mailing` package stays dependency-light; each
provider adapter is an independent opt-in package that references only
`Platform.Mailing` and its provider dependency.

## Packages

| Package | Provider dependency | Purpose |
| --- | --- | --- |
| `Platform.Mailing.Smtp` | MailKit | SMTP send with explicit TLS/authentication modes, MIME construction, and attachments. |
| `Platform.Mailing.SendGrid` | SendGrid | SendGrid send over an application-owned `ISendGridClient` with safe response classification. |

Both adapters implement `IMailService` and return the normalized
`MailSendResult`; neither throws for ordinary provider rejections, and neither
owns templates, retry policies, delivery records, or provider credentials
beyond its own options.

## Normalized outcomes

| Outcome | SMTP | SendGrid |
| --- | --- | --- |
| `Sent` | Server accepted the message. | 2xx response. `X-Message-Id` is surfaced as `ProviderMessageId` when present. |
| `TransientFailure` | SMTP 4xx replies, connection, TLS, and timeout failures (`mail.smtp.*`). | 429 (`mail.sendgrid.rate_limited`), 5xx (`mail.sendgrid.server_error`), transport and timeout failures (`mail.sendgrid.unavailable`, `mail.sendgrid.timeout`). |
| `PermanentFailure` | SMTP 5xx replies and authentication failures (`mail.smtp.recipient_rejected`, `mail.smtp.authentication_failed`, …). | Other 4xx responses (`mail.sendgrid.rejected`). |
| `Bounced` | Not produced; bounce detection is application-owned (webhooks or IMAP feedback loops). | Not produced; bounce detection is application-owned (event webhook). |

`TransientFailure` is the retry signal; `PermanentFailure` is not. Retry
ownership stays with the application (Hangfire, a workflow, or a custom loop);
the adapters never retry.

## Adoption

### SMTP

```csharp
services.AddPlatformSmtpMail(options =>
{
    options.Host = "smtp.example.com";
    options.Port = 587;
    options.SecureMode = SmtpSecureMode.StartTls; // explicit; no auto-negotiation
    options.UserName = "…";
    options.Password = "…";
});
```

`AddPlatformSmtpMail` validates the configuration at registration and adds
`IMailService` → `SmtpMailService` with a `TryAdd`, so an application-owned
`IMailService` registration always wins. A `Func<SmtpClient>` `ClientFactory`
seam is available for custom client lifecycle.

### SendGrid

```csharp
services.AddPlatformSendGridMail(options => options.ApiKey = "…");
```

`AddPlatformSendGridMail` validates the configuration at registration and
`TryAdd`s both `ISendGridClient` (a default `SendGridClient` with
`HttpErrorAsException = false` so failures are classified from the response)
and `IMailService` → `SendGridMailService`. Applications can register their own
`ISendGridClient` before the call to own the client lifecycle.

## Safety contract

- Invalid sender, recipient, subject, body, or attachment input is rejected as
  a configuration failure (`mail.configuration.*`) before the provider is
  contacted.
- Provider responses are never surfaced: diagnostics carry stable error codes
  and fixed safe messages only. SendGrid response bodies can echo message
  content or PII and are never included.
- Options validation messages never carry secrets.
- Caller cancellation is preserved: the adapters rethrow
  `OperationCanceledException` instead of converting cancellation into a
  provider failure.
- Each adapter exposes a `Status` snapshot (`SmtpMailProviderStatus` /
  `SendGridMailProviderStatus`) with the provider name, health state, and last
  stable error code.

## Delivery semantics

Provider acceptance is not delivery. `Sent` means the provider accepted the
message; bounces, deferrals, and suppression events are application-owned
concerns (SendGrid event webhooks, SMTP feedback loops). The adapters report
provider acceptance only.

## Migration from the starter kit

The starter `MailRequest` is mutable and provider-coupled. Wrap existing call
sites with a mapper from `MailRequest` to the immutable `MailMessage`
(`From`, `To`, `Subject`, `TextBody`/`HtmlBody`, attachments), then replace the
concrete `SmtpMailService`/`SendGridMailService` registrations with the
platform adapters. The starter's throw-on-transient behavior is replaced by
the normalized `TransientFailure` outcome; move the retry decision to the
caller. Rollback restores the starter implementations; no data migration is
needed.
