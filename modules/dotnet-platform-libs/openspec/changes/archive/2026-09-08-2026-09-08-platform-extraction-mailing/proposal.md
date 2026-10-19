# Proposal: Adopt VisualFlow's mailing contracts into the platform

## Why

Every consumer in the portfolio that sends transactional email needs a documented `IMailService` contract, a `MailMessage` value type, a template renderer abstraction, a structured `MailSendResult`, and a configurable `MailingOptions`. VisualFlow has implemented exactly this in `src/BuildingBlocks/Mailing/`, including the `MailTemplateId` opaque identifier, the `IMailTemplateRenderer` contract, the `RenderedMailTemplate` shape, and the documented `MailSendOutcome` enum. The type surface is framework-neutral and provider-neutral. Promoting it gives the portfolio a shared, audited mailing contract and proves that the platform can host a domain-flavored abstraction (transactional email) without leaking provider details.

## What Changes

- Add a new production package `Platform.Mailing` that owns the mailing contracts, the value types, the template renderer abstraction, the options, and the opt-in registration.
- The package depends on `Platform.Core` for `IClock` and the documented `Error` shape; it does not depend on ASP.NET Core, EF Core, a specific mail provider, or a templating engine.
- VisualFlow's `src/BuildingBlocks/Mailing/` is removed in a follow-up VisualFlow change that adopts the new package; this change is the platform side only.

## Capabilities

### New Capabilities

- `platform-mailing`: Documented `IMailService` contract, `MailMessage` value type, `IMailTemplateRenderer` contract, `RenderedMailTemplate` shape, `MailSendResult`, `MailSendOutcome` enum, `MailTemplateId` opaque identifier, and `MailingOptions`.

### Modified Capabilities

- (none)

## Impact

- Adds one new production NuGet package, `Platform.Mailing`, versioned in `Directory.Packages.props`.
- Expands the architecture test to forbid `Platform.Mailing` from referencing ASP.NET Core, EF Core, SendGrid, Mailgun, SMTP, Razor, Liquid, or a VisualFlow project.
- VisualFlow adopts the package in a separate change after this lands; no VisualFlow code is modified in this change.

## Context

The platform roadmap's Phase 3 adoption proof needs at least one domain-flavored contract (rate limiting is infrastructure, idempotency is infrastructure, eventing is infrastructure; mailing is the first domain-flavored candidate). The VisualFlow mailing block is the right size and shape: a small surface, framework-neutral, and provider-neutral.

## Goals / Non-Goals

**Goals:**

- Ship a self-contained, provider-neutral mailing package.
- Preserve the documented `MailSendOutcome` enum so a consumer can switch providers without changing the call sites.
- Provide a documented `IMailTemplateRenderer` contract that consumers can implement with their preferred templating engine.

**Non-Goals:**

- Ship a SendGrid, Mailgun, Postmark, or SMTP adapter in this change; a follow-up `platform-mailing-smtp` or `platform-mailing-sendgrid` change can adopt one if a second consumer needs it.
- Ship a Razor or Liquid renderer in this change; consumers can implement `IMailTemplateRenderer` against their preferred engine.
- Replace the existing mailing implementation in any consumer.

## Decisions

- Reuse the existing `Platform.Core.IClock` for the documented `MailSendResult.OccurredAt` if a consumer chooses to stamp the result.
- Use the `MailingOptions` shape verbatim from VisualFlow; the documented section name (`Mailing`) is preserved.
- Keep the `MailTemplateId` as an opaque identifier; consumers map templates to provider-specific IDs in their own adapter.

## Risks / Trade-offs

- The `MailingServiceCollectionExtensions` is intentionally minimal (it only registers `MailingOptions`); consumers must register an `IMailService` implementation against their provider. This is documented in the consumer guide.
- The `IMailTemplateRenderer` contract is minimal; consumers that need a model-binding step can implement it against their preferred engine without changing the bus or the call sites.
