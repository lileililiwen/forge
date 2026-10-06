# forge-web-command-catalog Specification

## Purpose

Make every Forge CLI command and nested action discoverable in the authenticated browser, with honest availability and safe-use guidance.

## ADDED Requirements

### Requirement: Exhaustive CLI command coverage

Forge SHALL represent every top-level and nested CLI command in an authenticated, versioned web catalog with a stable ID, user-facing purpose, category, scope, risk, and web route or explicit CLI-only disposition.

#### Scenario: New CLI command added

- **WHEN** a top-level or nested Clap command is added without catalog metadata
- **THEN** the command coverage check fails and identifies the missing command path

#### Scenario: Catalog is complete

- **WHEN** an authenticated operator opens command search
- **THEN** every command path from the Rust Clap tree appears once with a valid category and disposition

### Requirement: Truthful availability and safe navigation

Forge SHALL enable only implemented typed web routes and SHALL explain provider, project capability, disabled, not-yet-web and CLI-only states with a safe next step.

#### Scenario: Command requires unavailable provider

- **WHEN** a command requires a disabled or unavailable provider
- **THEN** the catalog identifies the provider prerequisite and does not present the action as runnable

#### Scenario: Command is terminal-only

- **WHEN** a command requires an interactive terminal, serves a transport, or is a developer/build operation
- **THEN** the catalog links to its CLI help and explains why there is no browser execution control

#### Scenario: Unknown search result

- **WHEN** a search has no matching commands
- **THEN** the UI presents an accessible empty result without calling an arbitrary shell command

### Requirement: Catalog authorization and invocation isolation

Forge SHALL require a valid global admin session to read the catalog and SHALL NOT provide a generic endpoint that executes arbitrary CLI text.

#### Scenario: Anonymous catalog request

- **WHEN** an unauthenticated client requests the command catalog
- **THEN** Forge returns 401 and no catalog data

#### Scenario: Search text contains shell syntax

- **WHEN** search text contains shell metacharacters
- **THEN** it is treated only as literal catalog text and is never executed
