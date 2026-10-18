## MODIFIED Requirements

### Requirement: Declare reusable capabilities explicitly

Projects MAY declare identity, admin, tenancy, billing, jobs, storage, and
release capabilities, and the cross-cutting shared capabilities telemetry,
rate-limiting, idempotency, and feature-flags. Governance MUST treat those
declarations as intent and must not imply that the capabilities are implemented
or production-ready.

A cross-cutting shared capability is one that applies to a provider, a product,
a library, and a tool alike, and whose id is bound by the capability registry.
Every project profile that declares a `supported_capabilities` list MUST
include the cross-cutting shared capability ids, so a consumer can record a
lawful `consumes[]` adoption of a registered shared capability without a
spurious `CAPABILITY_UNSUPPORTED` warning. A capability id that the profile's
list does not contain MUST still be reported as WARNING
`CAPABILITY_UNSUPPORTED`, so the supported set stays intentional and a
role-shaped shared capability is admitted only by a profile that decides to.

#### Scenario: Declared capability

- **WHEN** a project declares a capability with an owner and evidence state
- **THEN** governance reports the declaration without fabricating verification

#### Scenario: Missing evidence

- **WHEN** a declared capability has no required native evidence
- **THEN** governance reports it as unverified or review-required

#### Scenario: A cross-cutting shared capability is declarable in every profile

- **WHEN** a project declares a cross-cutting shared capability such as
  `telemetry` under a profile whose `supported_capabilities` list contains it
- **THEN** governance reports no `CAPABILITY_UNSUPPORTED` warning for it

#### Scenario: A capability outside the profile's supported set still warns

- **WHEN** a project declares a capability id that its profile's
  `supported_capabilities` list does not contain
- **THEN** governance reports WARNING `CAPABILITY_UNSUPPORTED` naming the
  capability and the profile
