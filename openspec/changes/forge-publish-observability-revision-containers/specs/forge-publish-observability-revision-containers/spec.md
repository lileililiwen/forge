# Capability: revision-identifiable publish status

## ADDED Requirements

### Requirement: Separate build and run status

Forge SHALL expose build and run as separate publish phases with independent
state, evidence, timestamps, and bounded failure detail.

#### Scenario: Build succeeds and run succeeds

- **WHEN** the requested revision builds and its Compose services become
  healthy
- **THEN** status SHALL report `build=succeeded`, `run=succeeded`, and the
  terminal publish result as healthy

#### Scenario: Build fails

- **WHEN** the Mac Docker build exits non-zero
- **THEN** status SHALL report `build=failed`, `run=not_started`, and SHALL
  include the bounded build failure evidence

#### Scenario: Run fails after build

- **WHEN** the image builds but Compose startup or health verification fails
- **THEN** status SHALL report `build=succeeded`, `run=failed`, and SHALL not
  report the deployment as healthy

### Requirement: Revision-qualified container identity

The provider SHALL include the requested committed revision's first 12
hexadecimal characters in the Compose project/container identity and Forge
status SHALL expose that identity.

#### Scenario: Current container identifies its revision

- **WHEN** revision `0123456789abcdef0123456789abcdef01234567` is published
- **THEN** the runtime identity SHALL include `0123456789ab` and status SHALL
  expose the full revision and qualified identity

#### Scenario: Explicit container name hides revision

- **WHEN** a Compose service defines a container name without the requested
  revision suffix
- **THEN** the provider SHALL fail the run verification and SHALL not report
  the service as a current healthy deployment

### Requirement: Status is evidence-backed

Forge SHALL not derive build or run success solely from an aggregate provider
health string when phase evidence is absent.

#### Scenario: Legacy provider response lacks phases

- **WHEN** a provider returns a healthy-looking terminal response without
  build and run evidence
- **THEN** Forge SHALL mark the missing phase evidence as `unknown` and SHALL
  show the evidence gap in `forge deploy status`

## Traceability

| Requirement | Design boundary | Implementation/tests | Evidence |
|---|---|---|---|
| Separate build and run status | Forge provider transport, registry, status CLI; Jenkins provider phases | phase fixture and status tests | independent phase states/evidence |
| Revision-qualified identity | Jenkins Compose project naming and runtime verification | naming/Compose fixture tests | Docker names/labels contain SHA suffix |
| Evidence-backed status | Forge response validation and status projection | legacy response negative test | missing evidence is unknown |
