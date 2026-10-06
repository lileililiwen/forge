# forge-web-delivery-controls Specification

## Purpose

Let operators review, explicitly authorize and track supported repository, provider and delivery operations in the web command center.

## ADDED Requirements

### Requirement: Reviewable typed operation plans

Forge SHALL provide a typed plan before every browser-triggered repository or external delivery mutation and SHALL display target, affected resources, provider, external effects, reversibility and preconditions.

#### Scenario: Plan is requested

- **WHEN** an operator prepares a supported delivery operation
- **THEN** Forge returns a side-effect-free plan bound to the current project and target state

#### Scenario: Plan cannot be produced

- **WHEN** a precondition or provider is unavailable
- **THEN** Forge reports the typed blocker and creates no operation or external effect

### Requirement: Exact confirmation and operation tracking

Forge SHALL require explicit confirmation bound to the displayed plan digest and idempotency key before dispatching any browser-triggered write and SHALL return a trackable operation identity.

#### Scenario: Confirmed operation

- **WHEN** a valid operator confirms the current plan and named effects
- **THEN** Forge dispatches the existing typed Core operation and exposes its journal-backed status by operation ID

#### Scenario: Stale or altered confirmation

- **WHEN** the plan expired, changed or confirmation does not match its digest
- **THEN** Forge rejects the request and dispatches no operation

### Requirement: External outcome reconciliation

Forge SHALL distinguish successful, failed, partial and unknown external outcomes and SHALL require reconciliation before retrying an ambiguous dispatch.

#### Scenario: Provider timeout after dispatch

- **WHEN** a provider times out after an external operation may have started
- **THEN** Forge records `unknown` or `reconciliation_required`, displays the provider and operation identity, and does not blindly retry

#### Scenario: Partial write

- **WHEN** some operation stages succeed and a later stage fails
- **THEN** Forge displays completed and failed stages, rollback status and recovery guidance

### Requirement: Credential and execution isolation

Forge SHALL use configured provider credentials without returning them to the browser and SHALL NOT execute arbitrary shell text or accept credentials from global login fields as provider credentials.

#### Scenario: User submits shell syntax

- **WHEN** an operation form includes arbitrary command text or shell metacharacters
- **THEN** Forge rejects the unsupported field and executes no process

#### Scenario: Provider credential unavailable

- **WHEN** the configured provider lacks a required credential
- **THEN** Forge reports the provider prerequisite without exposing secret values or dispatching the operation
