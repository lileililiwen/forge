# standalone-project-boundary Specification

## Purpose

Classify a discovered project as `standalone`,
`standalone-with-optional-integrations`, `library-dependent`,
`workspace-dependent`, `service-dependent`, or `blocked` from declared
metadata and bounded static inspection, without ever executing project
commands. The classification is additive: existing declarations without a
`standalone` block remain valid and are reported with status `blocked` and
a `STANDALONE_BLOCKED` warning so the audit never fabricates a pass.

The Forge governance-provider adapter MUST emit a normalized
`standalone_status` and a list of integration entries alongside the
existing governance status, evidence, and detail fields. Adapter output is
additive; existing governance observation keys, status mappings, and exit
semantics are unchanged.

## Requirements

### Requirement: Classify project independence

Workspace Governance SHALL classify a discovered project as one of the
following six statuses without executing project commands:

- `standalone` — local build/test/run baseline has no declared external
  prerequisite.
- `standalone-with-optional-integrations` — local baseline exists; one or
  more external systems are declared as optional adapters.
- `library-dependent` — the baseline depends only on versioned/published
  libraries identified by name and version, never on a sibling
  repository checkout.
- `workspace-dependent` — a sibling project checkout is required (declared
  in `required_sibling_projects` or detected from a bounded static
  scan of the project root) or directly imported.
- `service-dependent` — a private external service or credential is
  required for baseline operation (declared in
  `required_external_services`).
- `blocked` — the audit cannot derive a status from the available
  evidence.

#### Scenario: Local standalone project

- **WHEN** a declaration has a `standalone` block whose `status` is
  `standalone`, `local_defaults` is a non-empty array of strings, and
  `required_external_services` and `required_sibling_projects` are
  present and empty
- **THEN** the audit records the classification as `standalone` and
  emits no `STANDALONE_*` finding

#### Scenario: Standalone with optional integrations

- **WHEN** a declaration has a `standalone` block with status
  `standalone-with-optional-integrations` and a non-empty `integrations`
  array whose entries all have `required: false`
- **THEN** the audit records `standalone-with-optional-integrations`
  and emits no finding for the optional integrations

#### Scenario: Library-only dependency

- **WHEN** a declaration has a `standalone` block with status
  `library-dependent` and every `local_defaults` entry names a
  versioned package coordinate (string, non-empty)
- **THEN** the audit records `library-dependent` and emits no
  `STANDALONE_*` finding

#### Scenario: Required sibling project declared

- **WHEN** a declaration's `standalone.required_sibling_projects` array
  is non-empty
- **THEN** the audit records `workspace-dependent`, emits a
  `STANDALONE_REQUIRED_SIBLING` error naming each project, and never
  rewrites the declared sibling identifiers

#### Scenario: Required external service declared

- **WHEN** a declaration's `standalone.required_external_services` array
  is non-empty
- **THEN** the audit records `service-dependent` and emits a
  `STANDALONE_REQUIRED_SERVICE` warning naming each service (or
  `error` if a sibling project with the same id is also declared,
  because the donation boundary is then workspace-dependent)

#### Scenario: Insufficient evidence

- **WHEN** a declaration has no `standalone` block, or the block is
  missing required keys, or `evidence_ref` does not resolve to a file
  in the project root
- **THEN** the audit records `blocked`, emits a `STANDALONE_BLOCKED`
  warning naming the missing field or evidence, and never reports a
  standalone pass

### Requirement: Preserve portable path boundaries

The audit SHALL reject absolute machine-specific paths and SHALL report
the exact field or file containing the offending value. The audit SHALL
never rewrite a value into a portable form; malformed values are
errors, not auto-fixes.

#### Scenario: Absolute path in declaration field

- **WHEN** any value in `standalone.local_defaults`,
  `standalone.required_sibling_projects`,
  `standalone.required_external_services`, `integrations[].id`, or
  `integrations[].evidence_ref` is an absolute filesystem path
  (starts with `/`, contains a Windows drive letter, or matches
  `^[A-Za-z]:[\\/]`)
- **THEN** the audit emits `STANDALONE_ABSOLUTE_PATH` naming the
  field and project, and the value is not normalized

#### Scenario: Relative path resolves against the project root

- **WHEN** a project-local evidence reference is repository-relative
  (does not start with `/`, contains no `..` segment) and points to an
  existing file
- **THEN** the audit resolves the reference against the project root
  and treats the block as `configured` (or `verified` if the
  declaration marks the evidence as verified)

#### Scenario: Evidence ref escapes the project root

- **WHEN** an `evidence_ref` contains a `..` segment or resolves
  outside the project root
- **THEN** the audit emits `STANDALONE_PATH_ESCAPE` as an error and
  does not read the path

### Requirement: Optional integrations are bounded and non-fatal

The audit SHALL accept an optional `integrations` array and SHALL
classify each entry as `required` or `optional`. Optional entries MAY
be absent (provider offline, no credential) without changing the
declared baseline status. Required entries that are absent SHALL be
recorded as warnings, not errors, and SHALL NOT block the donation
contract: the declaration remains `standalone-with-optional-integrations`
so long as the local baseline still works.

#### Scenario: Optional provider offline

- **WHEN** an integration is declared with `required: false` and the
  provider is unavailable at audit time
- **THEN** the audit records the provider as optional, does not change
  the declared baseline status, and surfaces a `STANDALONE_OPTIONAL`
  warning with the integration id

#### Scenario: Required integration declared without evidence

- **WHEN** an integration is declared with `required: true` but has no
  `evidence_ref` or the `evidence_ref` does not resolve to a project
  file
- **THEN** the audit emits `STANDALONE_INTEGRATION_PENDING` as a
  warning naming the integration

### Requirement: Forbidden declared statuses are errors

The audit SHALL accept only the six canonical statuses
(`standalone`, `standalone-with-optional-integrations`,
`library-dependent`, `workspace-dependent`, `service-dependent`,
`blocked`). Any other value in the `standalone.status` field is a
schema error and the audit SHALL NOT guess a substitution.

#### Scenario: Unknown status rejected

- **WHEN** a declaration's `standalone.status` is any string other
  than the six canonical values
- **THEN** the audit emits `STANDALONE_STATUS_INVALID` naming the
  declared value and treats the project as `blocked` for the
  purposes of the audit summary

#### Scenario: Malformed standalone block

- **WHEN** the `standalone` field is not an object, or is missing
  required keys (`status`, `local_defaults`,
  `required_external_services`, `required_sibling_projects`), or any
  of the array fields is not an array of strings
- **THEN** the audit emits `STANDALONE_SHAPE` naming the offending
  field, does not derive a status from partial data, and reports
  `blocked`

### Requirement: Export normalized adapter evidence

The Forge governance-provider adapter SHALL emit `standalone_status`,
`standalone_observed_revision`, `standalone_integrations`, and a
`standalone_findings` list alongside the existing governance observation
keys. The adapter MUST NOT remove or rename any existing field and
MUST continue to honour the existing exit-code contract (0 for
answerable evaluations including `unknown`, 2 for malformed request,
3 for audit crash).

#### Scenario: Optional Workspace Governance integration

- **WHEN** a project's `integrations` array names
  `workspace-governance` with `required: false`
- **THEN** the adapter reports the integration as optional, records
  `standalone_status: "standalone-with-optional-integrations"`, and
  the consumer does not need the provider to be live for the project
  to pass

#### Scenario: Adapter audit failure

- **WHEN** the audit root is missing or the audit cannot be parsed
- **THEN** the adapter exits non-zero with the existing codes (2 for
  malformed request, 3 for audit crash), `standalone_status` is
  `blocked`, and `standalone_findings` carries a single
  `STANDALONE_SHAPE` finding

#### Scenario: Secret redaction

- **WHEN** an integration `evidence_ref` or a finding message
  contains a substring matching the shared secret vocabulary
  (loaded from `vocabulary.json`)
- **THEN** the matching substring is replaced with `[redacted]`
  before the observation is printed, matching the existing
  `scrub_line` behaviour

### Requirement: Redact credential-shaped values in evidence

The audit SHALL scan every `standalone` and `integrations` value for
credentials. Values matching the shared secret vocabulary (any
substring of `password`, `passwd`, `secret`, `token`, `apikey`,
`api_key`, `api-key`, `credential`, `private_key`, `privatekey`,
`private-key`, `bearer`, `session_ke`, `session_key`) followed by a
value (`=value`, `: value`, or whitespace + value) are redacted in
evidence lines. The audit SHALL still surface a
`STANDALONE_SECRET_VALUE` error so the operator can fix the
declaration, but the value itself is never written to the evidence
line or to a log.

#### Scenario: Secret value in declaration

- **WHEN** a declaration field such as
  `standalone.required_external_services[0]` contains a secret-shaped
  value
- **THEN** the audit emits `STANDALONE_SECRET_VALUE` with the value
  replaced by `[redacted]` and does not write the raw value to any
  log, error line, or evidence line

### Requirement: No command execution

The audit SHALL be read-only with respect to the project repository
and the host filesystem beyond reading the declaration, the project
markers, and the bounded static-inspection targets listed in
"Preserve portable path boundaries". The audit MUST NOT execute the
project verification command, run package managers, invoke sibling
checkouts, make network requests, or run Docker / Compose. Audit mode
does not modify the project.

#### Scenario: Audit is read-only

- **WHEN** the audit runs against a fixture project
- **THEN** a recursive hash of the project tree is identical before
  and after the audit, and no child process is launched by the
  audit
