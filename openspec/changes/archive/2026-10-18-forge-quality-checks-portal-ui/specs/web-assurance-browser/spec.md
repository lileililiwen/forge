# web-assurance-browser Specification (delta: quality-checks-portal-ui)

## ADDED Requirements

### Requirement: Advanced assurance diagnostics stay collapsed until requested

The Projects view SHALL keep its read-only Assurance tools in a native,
keyboard-operable disclosure named **Advanced diagnostics and records**,
collapsed by default. Existing registries and explicit reads SHALL remain
available inside the disclosure, including optional provider observations.
Opening or loading the Projects view SHALL NOT select a provider, probe it,
read provider status automatically, or run a provider. Provider names and
provider-specific diagnostic fields SHALL be shown only after the operator
opens the disclosure and explicitly requests the relevant read. The disclosure
SHALL preserve existing typed routes, text-only rendering, honest
unavailable-with-reason states, status announcements and CLI-only guidance.

#### Scenario: Assurance tools start collapsed

- **WHEN** a signed-in operator opens the Projects view
- **THEN** `Advanced diagnostics and records` is collapsed, no assurance
  registry read occurs, and the operator can expand it with the keyboard

#### Scenario: Provider observation is an explicit advanced read

- **WHEN** the operator expands the disclosure and explicitly requests a
  governance list/status/inspect read
- **THEN** Forge renders the typed read within the disclosure and runs no
  provider, adapter, write or journal operation

#### Scenario: Other assurance reads remain available

- **WHEN** the operator expands the disclosure and selects another supported
  registry
- **THEN** the same typed read, search, inspect, error and CLI-only behavior
  remains available with accessible results
