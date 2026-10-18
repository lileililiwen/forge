# project-scaffolding Specification (delta: scaffold-binds-governance-gate)

## ADDED Requirements

### Requirement: New project scaffolds validate and bind gate profiles

The project initializer SHALL reject a gate profile not present in the binder's supported profile registry before creating the target directory. For a supported profile, after writing the initial gate manifest, it SHALL invoke the binder in apply mode for that repository. Binder refusal or a dirty-manifest result SHALL make initialization fail; a completed bind that reports unresolved checks SHALL preserve and report that debt. The initializer SHALL preserve generated files for retry.

#### Scenario: Unknown profile is refused without filesystem changes

- **WHEN** initialization receives a profile name absent from the supported registry
- **THEN** it exits nonzero and identifies that the profile is unsupported
- **AND** it does not create the target directory

#### Scenario: Supported profile invokes repository-scoped binder

- **WHEN** initialization receives a supported profile and creates a new manifest
- **THEN** it invokes the binder with apply mode and a selector for that project
- **AND** the invocation occurs after the manifest exists

#### Scenario: Derivable command is recorded

- **WHEN** the new repository contains evidence from which the binder can safely derive a required command
- **THEN** the initialized manifest contains that binder-derived command
- **AND** the initializer reports success only after the binder succeeds

#### Scenario: Empty scaffold remains honestly unresolved

- **WHEN** the scaffold contains no evidence from which a required command can be derived
- **THEN** the binder leaves the command unbound
- **AND** the initializer does not invent a command or claim the gate is passing

#### Scenario: Binder refuses after scaffold creation

- **WHEN** the binder refuses the manifest update or cannot complete
- **THEN** initialization exits nonzero with actionable failure context
- **AND** the generated scaffold remains available for a safe retry

#### Scenario: Binder completes with unresolved checks

- **WHEN** the binder safely applies all commands it can derive but reports other required checks as unresolved
- **THEN** initialization completes and reports the exact unresolved checks
- **AND** it does not claim that the project gate passes
