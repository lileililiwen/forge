# sibling-cwd-publish Specification

## Purpose
A sibling checkout publishes itself through Forge via cwd discovery (`forge publish` from the sibling directory) without central inventory registration, using the existing `forge-publish-provider/0.1.0` contract and journal; `jenkins-local` remains a Forge-side plugin.
## Requirements
### Requirement: CWD-discovered single publish

Forge SHALL allow `forge publish` with no `--project` and no `--folder` to discover the target project from the current working directory and publish it through the selected external provider without requiring central inventory registration.

#### Scenario: Discover identity from .project.json

- **WHEN** `forge publish --dry-run` is run in a directory containing `.project.json` with `id: "alethefy"` and no `--project`/`--folder` is supplied
- **THEN** Forge discovers `project_id="alethefy"` and `project_dir` as the canonicalized cwd and validates the request without contacting workspace-governance

#### Scenario: Fall back to forge.yaml when .project.json is absent

- **WHEN** cwd has no `.project.json` but contains `forge.yaml` with `project.id: "demo-app"`
- **THEN** Forge discovers `project_id="demo-app"` from `forge.yaml`

#### Scenario: Fall back to directory basename

- **WHEN** cwd contains neither `.project.json` nor `forge.yaml` and the directory basename is a valid kebab/snake id
- **THEN** Forge discovers `project_id` from the canonicalized directory basename

#### Scenario: Explicit --cwd overrides the process cwd

- **WHEN** `forge publish --cwd /tmp/other --dry-run` is invoked
- **THEN** Forge discovers identity from `/tmp/other` rather than the process cwd

### Requirement: Bare publish shares the provider engine

A bare `forge publish` SHALL invoke the same `forge-publish-provider/0.1.0` external provider path as `forge publish --folder <path>`, with the same bounded invocation, secret redaction, phase evidence, and journal semantics.

#### Scenario: Dry-run prints the typed request without spawning

- **WHEN** `forge publish --dry-run` is run from a valid sibling checkout
- **THEN** Forge prints the `PublishProviderRequest` JSON (`contract`, `provider`, `project_id`, `revision`, `operation_id`) and exits successfully without spawning the provider

#### Scenario: Apply invokes the provider and records phase evidence

- **WHEN** `forge publish` is run from a valid sibling checkout with an enabled provider configured
- **THEN** Forge spawns the provider with a 40-hex `revision`, validates `revision`/`build_status`/`run_status`/`container_identity`, and persists the additive phase fields on the journal row

#### Scenario: Status is observable through the existing projection

- **WHEN** a bare `forge publish` completes
- **THEN** `forge deploy status --project <discovered-id>` returns the same `revision`/`build_status`/`run_status`/`container_identity` fields as an explicit `--folder` publish

### Requirement: Explicit flag precedence

Explicit `--project` and `--folder` SHALL take precedence over cwd discovery; supplying either flag SHALL bypass cwd discovery entirely.

#### Scenario: --folder beats cwd

- **WHEN** `forge publish --folder /tmp/other --dry-run` is run
- **THEN** Forge publishes `/tmp/other` and ignores cwd-discovered identity

#### Scenario: --project beats cwd

- **WHEN** `forge publish --project myapp --dry-run` is run
- **THEN** Forge resolves `myapp` via the registry lookup and ignores cwd

#### Scenario: --project and --folder remain mutually exclusive

- **WHEN** `forge publish --project a --folder b` is invoked
- **THEN** Forge refuses the invocation with a typed argument conflict before discovery or provider invocation

### Requirement: Revision discipline for bare publish

Forge SHALL capture the revision from the discovered directory's `HEAD` and enforce the same 40-hex revision contract as the explicit `--folder` path.

#### Scenario: Valid HEAD is accepted

- **WHEN** the discovered directory is a git checkout whose `HEAD` is a 40-hex SHA
- **THEN** the provider request carries that `revision` and `operation_id` `publish-<id>-<sha12>`

#### Scenario: Non-hex revision is refused before provider invocation

- **WHEN** `--revision short` or a non-hex `HEAD` is supplied for a bare publish
- **THEN** Forge returns `error[publish-invalid]` naming the revision shape and does not spawn the provider

#### Scenario: Missing git repository is refused before provider invocation

- **WHEN** the discovered directory has no `HEAD` and no `--revision` is supplied
- **THEN** Forge returns `error[publish-invalid]` about the missing revision and does not spawn the provider

### Requirement: Typed failures without silent fallback

Forge SHALL report discovery and provider-selection failures with typed `publish-invalid` errors and SHALL NOT silently scan sibling directories, invent an inventory, or fall back to another provider.

#### Scenario: Malformed .project.json is a typed discovery failure

- **WHEN** cwd `.project.json` exists but contains invalid JSON or a blank `id`
- **THEN** Forge returns `error[publish-invalid]` naming the file and does not fall back to the directory basename

#### Scenario: Invalid directory basename is a typed discovery failure

- **WHEN** cwd has neither identity file and its basename is not a valid project id
- **THEN** Forge returns `error[publish-invalid]` naming the cwd and suggesting `--folder`

#### Scenario: Unknown or disabled provider is refused before spawn

- **WHEN** a bare `forge publish` selects a provider id that is unknown or disabled in `<cwd>/.forge/providers.yaml` (or `FORGE_PUBLISH_PROVIDER_CONFIG`)
- **THEN** Forge returns `error[publish-invalid]` naming the provider and does not spawn it

#### Scenario: No sibling files are read outside the explicit --cwd / cwd

- **WHEN** bare publish is invoked in a checkout with no identity files
- **THEN** Forge does not search parent directories or sibling checkouts for a registry

### Requirement: Plugin boundary preservation

Forge SHALL remain the only user-facing publish entry point; `jenkins-local` remains a Forge-side provider configured via `FORGE_PUBLISH_PROVIDER` / `.forge/providers.yaml` and SHALL NOT be invoked directly by the sibling.

#### Scenario: Sibling publishes through Forge provider contract

- **WHEN** a sibling runs `forge publish` from its checkout and the configured provider is `jenkins`
- **THEN** Forge invokes the `jenkins` provider executable via the `forge-publish-provider/0.1.0` stdin/stdout envelope; the sibling never spawns `project-action.sh` itself

#### Scenario: Fleet mode is unchanged

- **WHEN** `forge publish fleet --inventory <path>` is invoked
- **THEN** Forge classifies every inventory entry (`compose_ready`/`compose_missing`/`invalid`/`source_unavailable`) exactly as before; bare single publish does not alter fleet reporting

