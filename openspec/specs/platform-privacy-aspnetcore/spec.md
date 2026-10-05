# platform-privacy-aspnetcore Specification

## Purpose
The platform-privacy-aspnetcore specification covers HTTP consent mechanics over the core vocabulary: consent-cookie state, request-scoped grants, and RequireConsent endpoint enforcement that refuses unconsented categories with 403. Routes, markup, and subject identity stay application-owned.
## Requirements
### Requirement: HTTP consent state

The adapter SHALL read the consent cookie into request-scoped state and write updated records back through a cookie service, with cookie name, lifetime, and policy version application-configured. It SHALL NOT define banner markup or product routes.

#### Scenario: Request with a valid cookie
- **WHEN** the consent cookie carries a current-version record
- **THEN** the scoped state grants exactly the recorded optional categories

#### Scenario: Tampered cookie
- **WHEN** the cookie value fails to parse
- **THEN** the state falls back to Necessary-only and the decision is observable

### Requirement: Category enforcement

The adapter SHALL enforce `RequireConsent` endpoint metadata: a request reaching an endpoint that requires categories it has not granted receives 403 with problem details naming the missing categories. Endpoints without the metadata are unaffected.

#### Scenario: Unconsented analytics endpoint
- **WHEN** an endpoint requiring Analytics is called without that grant
- **THEN** the response is 403 and no application handler runs

### Requirement: Explicit missing-context policy

The adapter SHALL require an explicit application policy for unresolved subject identity and SHALL NOT silently treat unrelated anonymous callers as one consenting subject.

#### Scenario: Missing subject
- **WHEN** consent is checked with no resolved subject and no explicit anonymous policy
- **THEN** the decision fails closed and is observable

