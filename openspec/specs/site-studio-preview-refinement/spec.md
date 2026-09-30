# site-studio-preview-refinement Specification

## Purpose
TBD - created by archiving change site-studio-preview-refinement. Update Purpose after archive.
## Requirements
### Requirement: Versioned site specification

Forge SHALL represent a supported site request as a validated, versioned
`forge.app.yaml` artifact separate from the infrastructure `forge.yaml`
manifest.

#### Scenario: Review before save

- **WHEN** a prompt produces a valid AppSpec proposal
- **THEN** Forge shows the proposal and writes it only after explicit
  owner confirmation that includes the current artifact revision

#### Scenario: Invalid or unsupported AppSpec

- **WHEN** a proposal contains an unsupported profile, duplicate route,
  unknown schema major, command, secret, or path escape
- **THEN** Forge returns a typed validation error
  (`studio-invalid-spec`, `studio-unsupported-profile`, or
  `studio-project-scope`) naming the exact field and does not write
  project files

#### Scenario: Project scope

- **WHEN** the spec's `project_id` does not match the registered
  project the route was scoped to
- **THEN** Forge refuses with `studio-project-scope` and the path
  the request resolved against

### Requirement: Bounded live preview

Forge SHALL run a preview for an authorized project revision through a
bounded, project-scoped process session whose lifecycle is journaled
through the existing `operations` table. The long-lived API process SHALL
own the live preview session for a project; the short-lived CLI SHALL
start the profile runner and report readiness through the same bounded
lifecycle without leaving a detached process behind.

#### Scenario: Preview becomes ready

- **WHEN** the configured profile process starts and responds within
  the startup window over the API
- **THEN** Forge persists the `ready` preview state, returns a
  `forge-studio-preview/0.1.0` envelope with `state: ready`, the
  reserved port, and a same-origin preview URL bound to that project
  and revision, and keeps that session alive until an explicit stop

#### Scenario: CLI readiness probe

- **WHEN** the operator starts a preview over the short-lived CLI
- **THEN** Forge runs the same bounded start, returns the `ready`
  envelope captured at readiness, and tears the preview down so no
  detached process survives the CLI invocation

#### Scenario: Preview startup failure

- **WHEN** the process times out, exits, or its requested port is
  occupied
- **THEN** Forge reports `state: failed` with the typed
  `studio-start-timeout` or `studio-port-unavailable` reason, cleans
  up only its own process, and does not claim the preview is available

#### Scenario: Project boundary

- **WHEN** a preview file or process request resolves outside the
  registered project root
- **THEN** Forge refuses it before reading or mutating the external
  path with `studio-project-scope`

#### Scenario: Idempotent stop

- **WHEN** the operator stops an already-stopped preview
- **THEN** Forge returns the same envelope with `state: stopped` and
  records exactly one `studio.preview.stop` journal row per request

#### Scenario: Bounded log capture

- **WHEN** the preview process writes more than the bounded log cap
  to stdout or stderr
- **THEN** Forge truncates with a marker, redacts credential-shaped
  strings, and never echoes the cap marker without the truncation
  suffix

### Requirement: Scoped refinement

Forge SHALL validate and journal a refinement request against the
current `app_revision` and the project's registered file scope.

#### Scenario: Refinement accepted

- **WHEN** the request matches the expected revision, every
  `selected_files` entry resolves inside the registered project root,
  and no shell metacharacter or secret-shaped string is present
- **THEN** Forge records a `studio.refine` journal row carrying the
  new `app_revision`, the bounded and redacted detail, and the
  bounded `selected_files` list

#### Scenario: Stale refinement

- **WHEN** the request's `expected_revision` does not match the
  current `spec_revision` or `app_revision`
- **THEN** Forge refuses with `studio-revision-conflict` and writes
  nothing

#### Scenario: Refinement path escape

- **WHEN** any `selected_files` entry resolves outside the
  registered project root or contains a secret-shaped string
- **THEN** Forge refuses with `studio-project-scope` or
  `studio-invalid-spec` and writes nothing

