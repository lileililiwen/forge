# Design

## Dependencies

This change depends on `platform-core-contracts` for `IClock` and the documented `Error` shape. It does not depend on ASP.NET Core, EF Core, a mail provider, or a templating engine.

## Components

Add a `src/Platform.Mailing/Platform.Mailing.csproj` project targeting `net8.0`. The project declares `<PackageReference Include="Microsoft.Extensions.Options" />` and `<PackageReference Include="Microsoft.Extensions.DependencyInjection.Abstractions" />` as transitive dependencies only.

Move the following types from `src/VisualFlow.BuildingBlocks.Mailing/` into `src/Platform.Mailing/` under the `Platform.Mailing` namespace, preserving the XML doc comments and the public surface:

- `MailAddress` — the documented value type with `Address` and `DisplayName`.
- `MailAttachment` — the documented value type with `FileName`, `ContentType`, and `Content`.
- `MailMessage` — the documented message shape with `From`, `To`, `Subject`, `TextBody`, `HtmlBody`, `Attachments`, and `CorrelationId`. Construction validates that at least one of `TextBody` or `HtmlBody` is supplied.
- `MailSendOutcome` — the documented enum (`Sent`, `TransientFailure`, `PermanentFailure`, `Bounced`).
- `MailSendResult` — the documented result with `Outcome`, `ProviderMessageId`, `ErrorCode`, and `ErrorMessage`.
- `IMailService` — the documented contract with `SendAsync(MailMessage, CancellationToken)`.
- `MailTemplateId` — the opaque identifier with implicit conversions to and from `string`.
- `IMailTemplateRenderer` — the documented renderer contract.
- `RenderedMailTemplate` — the documented rendered shape with `Subject`, `TextBody`, and `HtmlBody`.
- `MailingOptions` — the configuration type with `DefaultFromAddress`, `DefaultFromDisplayName`, `MaxAttempts`, `InitialBackoffSeconds`, `MaxBackoffSeconds`, and `TemplatesPath`.
- `MailingServiceCollectionExtensions` — the opt-in registration, renamed to `AddPlatformMailing`.

## Compatibility

The `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions` references are the same versions already used in `Platform.Core`. The package is added to `Directory.Build.props` packaging metadata with the documented version `0.1.0`.

## Verification

Unit tests cover `MailMessage` validation, `MailSendResult` construction, `MailTemplateId` implicit conversions, and the configuration validation. The architecture test `Platform.Architecture.Tests` is extended to enforce that `Platform.Mailing` does not reference ASP.NET Core, EF Core, SendGrid, Mailgun, SMTP, Razor, Liquid, or a VisualFlow project. A TestServer integration test proves the registration and the documented default options.

## Out of scope

- A SendGrid, Mailgun, Postmark, or SMTP adapter (`platform-mailing-smtp` or `platform-mailing-sendgrid`).
- A Razor or Liquid renderer.
- A delivery worker; the existing consumer worker continues to own delivery.
- Migration of the VisualFlow consumer (separate change).
- Migration of any other consumer in the portfolio.
