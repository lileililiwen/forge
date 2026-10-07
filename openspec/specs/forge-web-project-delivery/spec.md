# forge-web-project-delivery Specification

## Purpose
TBD - created by archiving change forge-web-project-delivery. Update Purpose after archive.
## Requirements
### Requirement: Read-only browser delivery status

Forge SHALL expose, on the session-gated global admin surface, a read-only delivery-status route for exactly one managed project id. It SHALL reuse the existing delivery projection without invoking a provider, adapter, build or write. Its path-free response SHALL identify the current phase, bound revision and latest preflight, stage, promote and Hermora evidence, plus the next eligible staged confirmation.

#### Scenario: Signed-in operator reads delivery status

- **WHEN** an authenticated admin requests delivery status for a managed project
- **THEN** Forge returns the current phase, revision and per-verb evidence without invoking any provider or writing any row

#### Scenario: Unsigned client requests delivery status

- **WHEN** a request without a valid admin session calls the status route
- **THEN** Forge refuses it as unauthorized and runs no Core operation

#### Scenario: Project id is unmanaged or path-bearing

- **WHEN** the status route receives an unknown id or an id containing a filesystem path
- **THEN** Forge returns typed 404 or 400 respectively, echoes no path and runs no Core operation

### Requirement: Confirmed digest-bound staged mutations

Forge SHALL require `confirm: true` and a digest matching the exact project, registered revision and staged confirmation before invoking preflight, stage, promote or Hermora retry from the browser. An unconfirmed request SHALL return a preview and digest without invoking a provider or writing a row. A mismatched digest SHALL be refused with a refreshed preview and no provider call.

#### Scenario: Apply without confirmation previews only

- **WHEN** a signed-in operator submits a delivery mutation without `confirm: true`
- **THEN** Forge returns the path-free preview and digest, invokes no provider and records no operation

#### Scenario: Apply with a mismatched digest is refused

- **WHEN** the supplied digest does not match the reviewed project, revision and confirmation
- **THEN** Forge refuses with a typed mismatch, returns a fresh preview and performs no provider call or journal write

#### Scenario: Confirmed staged mutation uses Core unchanged

- **WHEN** the operator confirms with the matching digest and valid staged confirmation
- **THEN** Forge delegates to the same delivery Core function as the CLI and records the resulting journal row

### Requirement: Health-gated promotion and optional Hermora enrollment

Forge SHALL enforce the existing staged prerequisites in browser delivery: stage requires a healthy preflight row for the registered revision; promotion requires a healthy stage row and matching revision; Hermora retry requires a healthy production row and valid deployment URL plus environment-variable secret reference. A failed or unhealthy provider outcome SHALL be recorded honestly and never presented as success. Hermora retry SHALL NOT republish.

#### Scenario: Stage without healthy preflight is refused

- **WHEN** stage is requested without a healthy same-revision preflight
- **THEN** Forge refuses with a typed conflict and invokes no provider

#### Scenario: Promotion without healthy stage is refused

- **WHEN** promotion is requested without a healthy same-revision stage
- **THEN** Forge refuses with a typed conflict and invokes no provider

#### Scenario: Hermora retry without healthy deployment is refused

- **WHEN** Hermora retry is requested without a healthy production row
- **THEN** Forge refuses with a typed conflict and invokes no adapter

#### Scenario: Unhealthy staged result is not success

- **WHEN** a provider returns a non-healthy terminal delivery outcome
- **THEN** Forge journals that real state and reports it honestly, never as success

### Requirement: Workbench delivery presentation and safe navigation

Forge SHALL present delivery status, latest staged evidence and the next required confirmation in the workbench for the open project. Mutation controls SHALL be generated from catalog execution blocks using typed fields only. The interface SHALL expose loading, empty, unavailable, permission-denied, error, confirmation, success and recovery states without raw HTML, shell text, filesystem paths or credential values.

#### Scenario: Operator follows the staged workbench flow

- **WHEN** a signed-in operator opens a managed project and runs the staged delivery controls in order
- **THEN** the browser shows phase transitions, requires each preview and confirmation, and ends in the Core-reported terminal state

#### Scenario: Delivery evidence is unavailable

- **WHEN** delivery status cannot be read or a staged prerequisite is absent
- **THEN** the browser shows an honest unavailable or blocked state and does not invent healthy progress
