# extension-point-contract Specification

## Purpose
Every extension boundary in Forge invented its own contract. There were nine
such surfaces — plugins, publish providers, governance providers, identity
providers, analytics providers, evidence providers, standard packs, identity
packs, and components and UI patterns — and they shared no descriptor, no
discovery rule, no version rule, no state rule and no shared test. Every
version check in the repository was exact string equality, so a newer
extension was refused exactly like an unrelated one, and the one boundary that
already behaved like a port — the deploy executor — was the only place a
future extension could plug into. This capability gives the repository one
extension contract: one way to read a declared contract version, one rule for
deciding whether it satisfies a requirement, one state vocabulary, one owner
for the deploy target kinds, and one contract test every kind runs.

## Requirements
### Requirement: One contract-version rule

Forge SHALL judge a declared contract version against a required one with one
rule: the families must match and the majors must be equal. A different
family, a different major, or a version that cannot be read is incompatible.
The minor and the patch are reported but do not decide compatibility.

#### Scenario: A newer minor is accepted

- **WHEN** an extension declares the required family at the same major and a
  newer minor
- **THEN** the declaration satisfies the requirement

#### Scenario: A major bump is refused

- **WHEN** an extension declares the required family at a different major
- **THEN** the declaration is refused, and the refusal names both majors

#### Scenario: Another family is refused

- **WHEN** an extension declares a different family
- **THEN** the declaration is refused, and the refusal names both families

#### Scenario: An unreadable version is refused

- **WHEN** an extension declares a version that cannot be read
- **THEN** the declaration is refused, and the refusal names the value

### Requirement: One state vocabulary for an extension

Forge SHALL report an extension's state from one vocabulary: `ready`,
`disabled`, `unavailable` or `invalid`. Every refusal that is not `ready`
SHALL name the extension and the reason.

#### Scenario: A usable extension is ready

- **WHEN** an extension is enabled, well-formed, compatible and executable
- **THEN** its state is `ready`

#### Scenario: A switched-off extension is disabled

- **WHEN** an extension is switched off by the operator
- **THEN** its state is `disabled`, and the reason names the extension

#### Scenario: An extension that cannot run here is unavailable

- **WHEN** an extension's command is not executable on this host
- **THEN** its state is `unavailable`, and the reason names the extension

#### Scenario: A malformed or incompatible extension is invalid

- **WHEN** an extension's declaration is malformed, or names a version that
  does not satisfy the requirement
- **THEN** its state is `invalid`, and the reason names the extension and the
  mismatch

### Requirement: One owner for the deploy target kinds

Forge SHALL keep the deploy target kinds in one owner, and every reader — the
manifest validator, the configuration validator and the deployment plan —
SHALL read that one list. A kind that is known but not yet wired SHALL be
reported as such, in the same words, wherever it is refused.

#### Scenario: An unknown kind is refused

- **WHEN** a manifest names a deploy target kind that is not known
- **THEN** the kind is refused, and the refusal names the kinds Forge supports

#### Scenario: A known but unwired kind is refused once

- **WHEN** a manifest names a deploy target kind that is known but not yet
  wired
- **THEN** the kind is refused as not yet wired, in the same words in the
  manifest, the configuration and the plan

### Requirement: The deploy executor boundary uses the shared contract

Forge SHALL judge a deploy adapter's declared contract with the shared
version rule rather than with string equality, and SHALL read the adapter's
self-identification as a family and a version.

#### Scenario: An adapter at a newer minor is accepted

- **WHEN** a deploy adapter declares the executor family at the same major and
  a newer minor
- **THEN** the adapter is accepted

#### Scenario: An adapter at a different major is refused

- **WHEN** a deploy adapter declares the executor family at a different major
- **THEN** the adapter is refused, and the refusal names both majors

#### Scenario: The adapter's self-identification is read

- **WHEN** a deploy adapter declares its own family and version
- **THEN** Forge records the family and the version with the stage evidence

#### Scenario: A malformed self-identification is refused

- **WHEN** a deploy adapter declares a self-identification that cannot be read
- **THEN** the adapter is refused, and the refusal names the value

#### Scenario: An absent self-identification is allowed

- **WHEN** a deploy adapter declares no self-identification
- **THEN** the adapter is accepted, and the evidence attributes to the configured
  binary with an unknown revision

### Requirement: One contract test every kind runs

Forge SHALL verify an extension kind against one shared contract, so a new
kind inherits the rules instead of restating them. The shared verification
SHALL cover the state vocabulary and the version rule.

#### Scenario: A kind is verified against the shared contract

- **WHEN** an extension kind is added
- **THEN** it runs the shared contract verification and is held to the shared
  state vocabulary and the shared version rule
