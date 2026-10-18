# web-project-delivery-timeline Specification

## Purpose
TBD - created by archiving change web-project-delivery-timeline. Update Purpose after archive.
## Requirements
### Requirement: Read-only project delivery timeline

Forge SHALL expose an authenticated, read-only, bounded project timeline that joins existing hosted CI observations, latest local Gate evidence, persisted releases, deploys, and relevant delivery/publish/Hermora journal operations while preserving each source's identity, status vocabulary, timestamp, and owner.

#### Scenario: Timeline contains source-labelled events

- **WHEN** an authenticated operator requests the timeline for a registered project with records in multiple sources
- **THEN** Forge returns events with source, owner, stage, source-specific status, timestamp, immutable source identity, revision when available, and safe source URL when available
- **AND** sorts them newest first with deterministic tie ordering
- **AND** does not collapse distinct records that share a revision
- **AND** does not synthesize a combined green or healthy status

#### Scenario: Hosted and local evidence scope is explicit

- **WHEN** a project has hosted CI observations or local Gate evidence
- **THEN** the timeline labels hosted records as observations received by Forge and exposes only the latest persisted local Gate evidence
- **AND** it does not imply Forge has queried every GitHub run or holds a complete local Gate history

#### Scenario: One source is unavailable

- **WHEN** one source cannot be read but other sources are readable
- **THEN** Forge returns the readable source events and marks the failed source unavailable with a safe note
- **AND** the timeline coverage is partial
- **AND** no unavailable source is represented as passing

#### Scenario: No events exist

- **WHEN** all queried sources are readable and contain no applicable records
- **THEN** Forge returns an empty event list with complete local-source coverage
- **AND** the client explains that no records were found without implying success

#### Scenario: Access and project boundaries are enforced

- **WHEN** the request has no valid administrator session or names an unknown project
- **THEN** Forge returns the existing unauthorized or typed not-found response
- **AND** does not accept a filesystem path from the browser

#### Scenario: Unsafe metadata is excluded

- **WHEN** source records contain local paths, malformed URLs, or provider details
- **THEN** the response omits absolute paths and secret-bearing details
- **AND** only validated GitHub Actions HTTPS run URLs are rendered as links

### Requirement: Delivery timeline operator experience

The Delivery page SHALL let an operator select a registered project and refresh its timeline, showing initial, loading, empty, populated, partial, expired-session/error, and retry states accessibly.

#### Scenario: Operator inspects a project timeline

- **WHEN** the operator selects a project and activates refresh
- **THEN** the page loads the read-only timeline and presents newest events first with source, owner, stage, status, time, revision and identity
- **AND** external links use safe targets and text is inserted without interpreting source data as HTML

#### Scenario: Timeline works on narrow screens and by keyboard

- **WHEN** the Delivery page is used at desktop width or a 390px viewport
- **THEN** event fields remain readable without horizontal page overflow
- **AND** refresh and external links are keyboard reachable in logical order with visible focus
- **AND** loading/completion changes are announced through a polite live region without stealing focus
