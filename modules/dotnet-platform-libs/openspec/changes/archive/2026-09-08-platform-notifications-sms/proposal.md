# Proposal: Notifications and SMS capability

## Why

The projects repeat email senders, console fakes, SMS seams, notification scheduling, retry
behavior, and provider failure handling. `Platform.Mailing` provides the email contract but not
the broader notification intent or SMS boundary.

## Scope

Add notification intent and delivery-result contracts, SMS provider abstractions, a console/fake
adapter, and integration hooks for jobs, eventing, and existing mailing contracts.

## Non-goals

- no product-specific templates or notification wording;
- no mandatory Twilio, SMTP, SendGrid, Firebase, or email provider;
- no push implementation in the first slice;
- no silent sending in production when a provider is unconfigured.

## API impact

Adds public notification, SMS, provider-status, and testing contracts.
