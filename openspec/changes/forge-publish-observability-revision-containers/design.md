# Design: revision-identifiable publish status

## Implementation boundary

Forge changes:

- `src/publish/providers.rs`: phase progress contract and revision validation.
- `src/main.rs`: status projection with build/run fields.
- `src/registry/mod.rs`: phase evidence persistence or normalized operation
  detail compatible with the existing SQLite journal.
- `tests/provider_contract.rs` and status CLI tests.

Jenkins provider changes:

- `jenkins-local/adapters/forge-publish-provider.py`: emit build/run events,
  use revision-qualified Compose project names, and verify runtime identity.
- `jenkins-local/tests/test_forge_publish_provider.py`: protocol and naming
  fixtures.

## Language and runtime

Forge uses Rust/Cargo/SQLite. The provider uses Python 3 and executes Docker
Compose only through SSH on Mac Docker Desktop. Required checks are the
existing Forge Cargo tests/build, Jenkins unittest suite, and strict OpenSpec
validation.

## Ownership and shared code

Forge owns the semantic phase model and status output. Jenkins owns the
provider-specific Compose invocation. The Mac stores runtime data, images,
containers, logs, and cache only; no source or script ownership changes.

## Behavioral model

| Publish phase | Started by | Success evidence | Failure meaning |
|---|---|---|---|
| `build` | provider has staged the exact revision and starts Compose build | image build exits 0 and image is tagged/project-labeled with revision | image cannot be built; run is not attempted |
| `run` | build succeeded and Compose starts the stack | containers exist with revision identity, health/Compose observation succeeds, routes refresh | stack startup, health, or route refresh failed |
| `complete` | run succeeded | terminal provider response `done/healthy` | not a separate build/run result |

Runtime identity is `forge-<project>-<sha12>` as the Compose project name.
Generated container names therefore include the SHA when the Compose file does
not override `container_name`. If a source Compose file explicitly sets a
container name that omits the SHA, the provider SHALL fail before declaring
run success and SHALL report the offending service. It SHALL not silently
claim revision identity from an image tag alone.

The full SHA remains in Forge request/journal data; the 12-character suffix is
the human/container identity. Hash validation remains strict hexadecimal.

## Contract and compatibility

Progress events extend the existing event with:

```json
{
  "event": "publish.progress",
  "project_id": "alethefy",
  "revision": "<full-40-char-sha>",
  "phase": "build",
  "status": "started|succeeded|failed",
  "detail": "bounded secret-free text"
}
```

The same shape is used for `phase=run`. Terminal response evidence includes
`build_status`, `run_status`, `revision`, and bounded evidence lines. Existing
providers that return only a terminal response remain accepted for single
publish compatibility but status marks missing phase evidence as `unknown`,
never as success.

`forge deploy status` JSON adds:

```json
{
  "project": "alethefy",
  "revision": "<sha>",
  "build": {"state": "succeeded", "detail": "..."},
  "run": {"state": "succeeded", "detail": "..."},
  "container_identity": "forge-alethefy-<sha12>"
}
```

## Failure and boundary policy

- Missing/invalid SHA: reject before staging.
- Build failure: persist `build=failed`, `run=not_started`; do not run the
  old or partially built container as the new revision.
- Run failure: persist `build=succeeded`, `run=failed`; report the exact
  bounded Compose/health evidence.
- Missing revision-qualified container: run is `failed`, even if a container
  is otherwise healthy.
- Explicit conflicting `container_name`: refuse before run success.
- Old revision container: remains visible for rollback/cleanup, but status
  identifies it as stale rather than current.
- Secret-like Docker output: redact before persistence/display.

## Verification oracle

- Provider unit tests prove build failure does not emit run success, run failure
  preserves build success, and generated Compose project/container identity
  includes the SHA.
- Forge contract tests prove ordered phase events, missing phase evidence,
  invalid revision, and status JSON/human parity.
- A Compose fixture with an explicit non-qualified `container_name` is refused.
- A fake provider fixture proves a success response without build/run evidence
  is not shown as fully verified.
- Real Mac evidence includes `docker ps --format`, Compose labels/names, the
  requested SHA, and successful route/health observation.

## Decision ledger

- Resolved: use the first 12 SHA characters for Docker naming and retain the
  full SHA in Forge state.
- Resolved: build and run are separate terminal evidence fields, not inferred
  from one aggregate health value.
- Resolved: explicit conflicting `container_name` is a hard failure because
  silently renaming an application can break its runtime contract.
- Deferred: automatic cleanup policy for stale revision containers.
- Deferred: image registry tagging/pushing; the Mac local BuildKit cache remains
  the build reuse mechanism.
