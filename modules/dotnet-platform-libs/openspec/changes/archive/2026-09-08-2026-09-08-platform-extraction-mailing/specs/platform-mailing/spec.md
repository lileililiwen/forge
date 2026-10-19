# platform-mailing Specification

## Purpose

Provide a provider-neutral mailing contract, a documented `MailMessage` value type, a template renderer abstraction, and a structured `MailSendResult` that any application in the portfolio can adopt without bringing in ASP.NET Core, EF Core, a specific mail provider, or a templating engine. The package is the fourth Phase 3 adoption proof: a real consumer (VisualFlow) contributes the type surface, the platform owns the package, and the consumer migrates to it in a follow-up change.

## ADDED Requirements

### Requirement: Mailing SHALL expose a documented contract and value types

The package SHALL expose an `IMailService` contract, a `MailMessage` value type, a `MailSendResult` value type, a `MailSendOutcome` enum, a `MailTemplateId` opaque identifier, and a `MailingOptions` configuration type.

#### Scenario: Consumer constructs a mail message

- GIVEN a consumer constructs a `MailMessage` with `From`, `To`, `Subject`, `TextBody`, and `HtmlBody`
- WHEN the construction validates
- THEN the message is accepted because at least one of `TextBody` or `HtmlBody` is supplied

#### Scenario: Consumer constructs a message with no body

- GIVEN a consumer constructs a `MailMessage` with empty `TextBody` and `HtmlBody`
- WHEN the construction validates
- THEN the construction throws the documented `ArgumentException`
- AND no message is created

#### Scenario: Provider returns a result

- GIVEN a provider implementation of `IMailService`
- WHEN the consumer calls `SendAsync`
- THEN the provider returns a `MailSendResult` with a documented `Outcome` and a documented `ProviderMessageId`
- AND a transient failure returns `Outcome = TransientFailure` with the documented `ErrorCode`

### Requirement: Template renderer SHALL be opt-in and replaceable

The package SHALL expose an `IMailTemplateRenderer` contract and SHALL ship a `RenderedMailTemplate` value type. The package SHALL NOT ship a default renderer; consumers can implement one against their preferred templating engine.

#### Scenario: Consumer registers a custom renderer

- GIVEN a consumer registers a custom `IMailTemplateRenderer`
- WHEN the consumer calls `RenderAsync` with a `MailTemplateId` and a model
- THEN the renderer is invoked
- AND the result is a `RenderedMailTemplate` with `Subject`, `TextBody`, and `HtmlBody`

#### Scenario: No default renderer

- GIVEN a consumer has not registered a custom renderer
- WHEN the host starts
- THEN the package does not register a default renderer
- AND the consumer can opt in to a renderer without changing the call sites

### Requirement: Opaque identifier SHALL be type-safe

`MailTemplateId` SHALL be an opaque identifier with implicit conversions to and from `string`; the package SHALL NOT expose a constructor that accepts a null or empty string.

#### Scenario: Implicit conversion to string

- GIVEN a consumer constructs a `MailTemplateId("welcome")`
- WHEN the consumer uses the value in a logging statement
- THEN the value is implicitly converted to the string `"welcome"`
- AND the conversion does not allocate

#### Scenario: Null or empty identifier

- GIVEN a consumer constructs a `MailTemplateId` with a null or empty string
- WHEN the construction validates
- THEN the construction throws the documented `ArgumentException`

### Requirement: Options SHALL be configurable

The package SHALL expose a `MailingOptions` configuration type with the documented fields (`DefaultFromAddress`, `DefaultFromDisplayName`, `MaxAttempts`, `InitialBackoffSeconds`, `MaxBackoffSeconds`, `TemplatesPath`) and SHALL bind to the documented `Mailing` configuration section.

#### Scenario: Default registration

- GIVEN a consumer calls `AddPlatformMailing`
- WHEN the host starts
- THEN `MailingOptions` is bound to the `Mailing` configuration section
- AND the documented defaults are applied when no section is present

#### Scenario: Consumer overrides defaults

- GIVEN a consumer sets `Mailing:DefaultFromAddress` in configuration
- WHEN the host starts
- THEN the configured value overrides the documented default
- AND the consumer can opt out without changing the call sites

### Requirement: Package SHALL be provider-neutral and framework-neutral

The package SHALL depend only on `Platform.Core`, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`. It SHALL NOT reference ASP.NET Core, EF Core, SendGrid, Mailgun, SMTP, Razor, Liquid, or a VisualFlow project.

#### Scenario: Architecture test

- GIVEN the architecture test runs
- WHEN it inspects the package's references
- THEN the test fails if any forbidden reference is present
- AND the test passes with the documented reference list
