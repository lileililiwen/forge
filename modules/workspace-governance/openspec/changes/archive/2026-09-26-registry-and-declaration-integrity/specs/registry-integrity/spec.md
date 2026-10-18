# registry-integrity Specification

## Purpose

Make the registry and every project declaration the single, schema-valid,
mutually consistent source of governance identity, so the authoritative audit
and the maturity report can no longer disagree.

## ADDED Requirements

### Requirement: Registry completeness is enforced

Every discovered project declaration MUST have exactly one registry entry, and
an unregistered declaration MUST be an ERROR rather than a warning; a directory
with neither a declaration nor a registry entry MUST NOT be reported.

#### Scenario: Discovered and registered

- **WHEN** a directory contains a `.project.json` whose `id` matches a registry
  entry that names the same `path`
- **THEN** the audit reports no finding for that project

#### Scenario: Discovered but unregistered

- **WHEN** a directory contains a `.project.json` and no registry entry names it
- **THEN** the audit reports ERROR `DISCOVERED_UNREGISTERED` naming the project

#### Scenario: Duplicate registry identity

- **WHEN** two registry entries share an `id` or a `path`
- **THEN** the audit reports ERROR and does not treat either as valid

#### Scenario: Not a discovered project

- **WHEN** a directory has neither a declaration nor a registry entry
- **THEN** the audit reports no finding for it

### Requirement: Declaration identity is single-sourced

A project declaration MUST be schema-valid (`schema_version: 1`), MUST carry a
`verification.command`, a `verification.gate_runtime` and a
`verification.evidence_status`, MUST carry the required `deployment` block, and
its `id` MUST equal the `id` of its registry entry. An adopted registry entry
MUST have a declaration.

#### Scenario: Valid declaration

- **WHEN** an adopted project's declaration conforms to schema v1 and its `id`
  equals the registry `id`
- **THEN** the audit reports no identity or format finding

#### Scenario: Identity mismatch

- **WHEN** a declaration's `id` differs from its registry `id`
- **THEN** the audit reports ERROR `DECLARATION_ID` naming both values

#### Scenario: Non-conformant declaration

- **WHEN** a declaration omits `schema_version: 1` or the required verification
  or deployment fields
- **THEN** the audit reports ERROR `DECLARATION_SCHEMA`

#### Scenario: Adopted without declaration

- **WHEN** a registry entry is `adoption: adopted` and its directory has no
  `.project.json`
- **THEN** the audit reports ERROR `DECLARATION_MISSING`

#### Scenario: Legacy verification key

- **WHEN** a declaration provides a real command under a non-schema key such as
  `verification_command`
- **THEN** the audit reports ERROR `VERIFICATION_COMMAND`

### Requirement: Canonical id is derived at authoring time

`init_project.py` MUST derive the project `id` from the target directory name
using the kebab rule, MUST refuse an explicit `--id` that differs from the
derived id, and MUST allow an explicitly recorded alias only when the directory
name is not a valid identifier.

#### Scenario: Derived id

- **WHEN** `init_project.py` targets a directory named `hestiaLab`
- **THEN** the written declaration's `id` is `hestia-lab`

#### Scenario: Conflicting explicit id refused

- **WHEN** `init_project.py` is given an `--id` that differs from the derived id
  for a non-dotted directory
- **THEN** it refuses and writes nothing

#### Scenario: Dotted directory alias

- **WHEN** the directory name is not a valid identifier, such as
  `lileililiwen.github.io`
- **THEN** an explicit alias is accepted and recorded with the real `path`

### Requirement: One authoritative audit result

`maturity_audit.py` MUST report the error and warning counts produced by
`workspace_check.py` and MUST NOT compute a second, independent result; when the
audit cannot be read it MUST exit non-zero with the reason.

#### Scenario: Counts agree

- **WHEN** both commands run against the same workspace root
- **THEN** the maturity report's `errors` and `warnings` equal the audit's

#### Scenario: Audit unreadable

- **WHEN** the underlying audit cannot be executed or parsed
- **THEN** `maturity_audit.py` exits non-zero and names the failure instead of
  reporting a clean workspace

#### Scenario: Empty workspace

- **WHEN** no project is registered
- **THEN** both report 0 errors and 0 warnings

### Requirement: Promotion freshness is surfaced

A project whose promotion-ledger revision differs from its current `HEAD` MUST
be reported as WARNING `DEPLOYMENT_PROMOTION_STALE` naming both revisions, and
MUST NOT be silently treated as current or as deployable.

#### Scenario: Current promotion

- **WHEN** the promoted revision equals the project's current `HEAD`
- **THEN** no staleness warning is reported

#### Scenario: Stale promotion

- **WHEN** the promoted revision differs from the current `HEAD`
- **THEN** the audit reports WARNING `DEPLOYMENT_PROMOTION_STALE` naming both
  revisions and the remedy (re-promote at `HEAD`)
