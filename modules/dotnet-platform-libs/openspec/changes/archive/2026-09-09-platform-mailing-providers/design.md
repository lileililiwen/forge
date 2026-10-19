## Context

The starter has `SmtpMailService`, `SendGridMailService`, `MailRequest`, and tests covering provider behavior. The platform has validated `MailMessage`, `MailSendResult`, `MailSendOutcome`, attachments, and template seams.

Starter-kit references:

- `dotnet-starter-kit/src/BuildingBlocks/Mailing/MailRequest.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Mailing/Services/SmtpMailService.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Mailing/Services/SendGridMailService.cs`
- `dotnet-starter-kit/src/Tests/Framework.Tests/Mailing/`

Use these for MIME construction, recipient handling, attachments, and response classification, then map to `Platform.Mailing` records. Do not copy the mutable starter request type or silently preserve its failure behavior.

## Goals / Non-Goals

**Goals:**

- Provide opt-in SMTP and SendGrid adapters with normalized results and safe diagnostics.
- Validate sender/recipient/body constraints before provider calls.
- Preserve cancellation and make retry classification explicit.

**Non-Goals:**

- Owning templates, notification workflows, outbox delivery, retry scheduling, suppression lists, or provider account policy.
- Adding MailKit, MimeKit, or SendGrid dependencies to `Platform.Mailing`.

## Decisions

1. Create `Platform.Mailing.Smtp` and `Platform.Mailing.SendGrid` as separate packages.
2. Return `MailSendResult` for accepted/permanent/transient/bounced outcomes; do not throw for ordinary provider rejection unless the contract explicitly classifies it as transient/configuration.
3. Use options validation with secret-free error messages and allow application-owned client factories where provider SDK lifecycle requires it.
4. Keep retries outside the adapter by default so Hangfire or application workflows own retry count and backoff.

Alternative rejected: a single provider package would force all consumers to install both SDK stacks.

## Risks / Trade-offs

- [Risk] Provider SDK response bodies may contain secrets or PII → redact response details and expose only stable error codes.
- [Risk] SMTP providers differ in TLS/auth modes → make security mode explicit and reject ambiguous configuration.
- [Risk] Accepted does not mean delivered → document that adapters report provider acceptance only.

## Migration Plan

Wrap existing starter mail calls with a mapper from `MailRequest` to `MailMessage`, run provider contract tests, then replace the concrete service. Rollback restores the starter implementation; no data migration is needed.

## Open Questions

- Whether a future `Platform.Mailing.Testing` package should add recording service and failure scripts; it is not required for this change.
