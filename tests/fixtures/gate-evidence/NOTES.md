# Gate evidence export fixtures

Captured from driftwatchdog HEAD `221faecae64727a2f73d5d89cc5ef6a071a185fe`,
`driftwatchdog gate evidence-export --format json`.

## Fixture: forge-all-unverified.json

Produced by: `driftwatchdog gate evidence-export --format json` against this
checkout after `driftwatchdog init --no-config`.

**Real shape:**
- Top-level keys: `schema_version` (integer), `project_id`, `revision` (git
  SHA), `toolchain` (runtime@version), `fields` (array of `{field, state}`
  objects), `manifest_digest`, `rule_pack_version`, `gate_run_id`.
- Nine fields matching the WG vocabulary exactly (`revision`, `version`,
  `toolchain`, `artifacts`, `digests`, `sbom`, `provenance`, `checks`,
  `publication`).
- All nine fields `unverified` because no check command was declared.
- No `evidence_ref` field on any entry (design expected one for `verified`
  fields; absent when no field is `verified`).
- `gate_run_id` is an integer primary key from the sibling's own `gate_runs`
  table, not a UUID.
- `manifest_digest` is a sha256 prefix of the manifest the run used.
- `rule_pack_version` is `workspace-governance@1`.

**Divergences from the design's expectations:**
- No `producer`/`producer_revision` fields. The design expected attribution
  on the document itself; the sibling attributes via `toolchain` only.
  Forge will derive `producer` from the `toolchain` field's runtime name
  and `producer_revision` from `gate_run_id` + sibling version.
- No `released_at` timestamp. The sibling does not emit one; the
  `platform.release-evidence` contract (required field) is not emitted by
  the sibling's export either — it is a release-publisher concern, not
  a gate concern.
- `fields` is a flat array of `{field, state}` not `{name, state,
  evidence_ref, attribution}`. No `evidence_ref` when the field is
  `unverified`; `evidence_ref` would appear on a `verified` entry per
  the sibling spec's requirement.
- No per-field `evidence_ref` structure (the sibling's spec says "carrying
  a state and an evidence reference", but the reference is only present
  when the state is `verified`).
- No `blocked` state in this document (all are `unverified`); the sibling
  spec reserves `blocked` for checks that cannot execute, which matches
  this output.
- `gate_run_id` is an integer, not a UUID.
- `schema_version` is a bare integer `1`, not `WGvocabulary/1` or similar.
  The sibling's spec does not name the wire format of the schema_version.

**Companion change:** `driftwatchdog/openspec/changes/gate-evidence-export/`
archived `2026-09-27` at `221faeca`. This fixture is the companion evidence.
