## Why

`Platform.Mailing` already defines provider-neutral messages and outcomes, but the starter kit's SMTP and SendGrid implementations are not reusable through those contracts. Consumers must choose between duplicating provider code or depending on the starter's concrete request model.

## What Changes

- Add independent SMTP and SendGrid adapters over `IMailService`.
- Normalize provider IDs, transient/permanent failures, cancellation, attachments, and sender configuration.
- Add safe configuration validation and provider status.
- Keep templates, retry ownership, delivery records, and product notifications application-owned.

## Capabilities

### New Capabilities

- `platform-mailing-providers`: SMTP and SendGrid adapters for the platform mailing contracts.

### Modified Capabilities

- None.

## Impact

Adds optional provider packages and public adapter options. The existing provider-neutral mailing package remains dependency-light. No provider API key, template catalog, or delivery database is owned by the platform.
