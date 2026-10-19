# Identity sample (stage 3)

Credential verification against an application-owned store through the
provider-neutral `Platform.Identity.Contracts` boundary.

## Owned by the application

- `SampleCredentialVerifier`: user records, secret handling, and any
  hashing, lockout, or recovery rules (a dictionary stands in for a
  real store here).
- The `/sample/login` endpoint shape and whatever session or token
  issuance follows verification.

## Owned by the platform

- The `Credential`, `CurrentUser`, and `IdentityProviderResult<T>`
  contracts plus the safe `IdentityFailureReason` categories. Failures
  never carry provider internals.

## Rollback

Remove the `Platform.Identity.Contracts` reference and implement the
two-record result shape locally; the endpoint and store keep working.

## Non-goals

No JWT issuance, external providers, two-factor, recovery, or session
persistence. Those are separate opt-in capabilities with their own
application-owned stores.
