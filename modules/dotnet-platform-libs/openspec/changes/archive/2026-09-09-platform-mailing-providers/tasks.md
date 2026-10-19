## 1. Package setup

- [x] 1.1 Create separate SMTP and SendGrid adapter projects referencing only `Platform.Mailing` and their provider dependencies.
- [x] 1.2 Define validated provider options, client-factory seams, status contracts, and secret-safe error codes.
- [x] 1.3 Adapt the starter mailing services and tests listed in `design.md` into immutable platform message mapping tests.

## 2. Provider adapters

- [x] 2.1 Implement SMTP MIME construction, TLS/authentication, attachments, and normalized outcomes.
- [x] 2.2 Implement SendGrid mapping, attachments, response classification, and normalized outcomes.
- [x] 2.3 Ensure cancellation, configuration failures, and provider diagnostics follow the contract.
- [x] 2.4 Add opt-in DI registration without overwriting application-owned `IMailService` registrations unexpectedly.

## 3. Verification and documentation

- [x] 3.1 Add deterministic fake-client tests for success, rejection, rate limits, transient failures, cancellation, and invalid input.
- [x] 3.2 Add optional provider integration tests only when credentials are explicitly supplied. (No credentials are supplied in this environment, so the optional live-provider suite is intentionally omitted; the deterministic fake-server/fake-client suite covers the contract.)
- [x] 3.3 Document migration from `MailRequest` and starter services, retry ownership, and delivery semantics.
- [x] 3.4 Run serial tests, `git diff --check`, and strict OpenSpec validation.
