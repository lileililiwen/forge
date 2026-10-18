# Design: standalone-project-boundary

## 1. Implementation boundary

- Repository: `workspace-governance` only.
- New file: `tests/test_standalone_project_boundary.py`.
- Changed files:
  - `schemas/project.schema.json` — additive `standalone` and
    `integrations` blocks under `properties` plus matching `$defs`.
  - `vocabulary.json` — additive `standalone_statuses` and
    `integration_required_states` arrays (the `profiles`/`kinds`/
    `placeholder_markers`/`secret_field_substrings` arrays are
    unchanged so existing callers see no diff).
  - `scripts/vocabulary.py` — surface the two new arrays via
    `vocabulary.load_vocabulary_for_governance_root(...)` and
    fail-closed on duplicates or empty lists.
  - `scripts/workspace_check.py` — new validator
    `validate_standalone(declaration, project_path, findings, project_id)`
    and finding codes `STANDALONE_BLOCKED`,
    `STANDALONE_STATUS_INVALID`, `STANDALONE_SHAPE`,
    `STANDALONE_ABSOLUTE_PATH`, `STANDALONE_PATH_ESCAPE`,
    `STANDALONE_REQUIRED_SIBLING`, `STANDALONE_REQUIRED_SERVICE`,
    `STANDALONE_OPTIONAL`, `STANDALONE_INTEGRATION_PENDING`,
    `STANDALONE_EVIDENCE_MISSING`, `STANDALONE_SECRET_VALUE`.
    The validator is invoked from `validate_project` after the
    release-evidence and quality checks so existing error codes and
    exit semantics are unchanged when the block is absent.
  - `scripts/forge_governance_adapter.py` — new fields
    `standalone_status`, `standalone_observed_revision`,
    `standalone_integrations`, `standalone_findings` on the
    observation document, populated by calling the new
    `classify_standalone(...)` helper which mirrors the checker's
    logic in-process (no new subprocess).
  - `README.md` — one short paragraph under a new
    "Standalone project boundary" subsection, with one
    copy-pasteable command.
  - `HANDOFF.md` — recorded by the second commit (this design does
    not pre-write it).
- Must not change: `projects.json`, the decisions register,
  `promote_project.py`, `jenkins_manifest.py`,
  `runtime_templates.py`, `init_project.py` (the standalone block
  is opt-in), any sibling repository, the profile JSON documents
  in `profiles/`, the capabilities or release-evidence schema,
  the governance-front-door skill, or the
  `external-postgres-runtime-profile` spec.

## 2. Language and runtime

- Python 3.11, stdlib only (`json`, `re`, `pathlib`,
  `dataclasses`), consistent with every other governance script.
- Tests: `python3 -m unittest discover -s tests`.
- Validation commands:
  - `python3 -m unittest discover -s tests -v`
  - `python3 scripts/workspace_check.py --self-check`
  - `python3 scripts/auto_gate.py --self-check --no-deployment`
  - `python3 scripts/jenkins_manifest.py --check`
  - `node scripts/check-openspec-change-names.mjs`
  - `openspec validate --changes --strict --no-interactive`
  - `git diff --check`
- Target: local execution, no network, no Docker, no child
  processes, matches the repo's no-daemon posture.

## 3. Ownership and shared code

- Governance owns the policy vocabulary, the finding codes, and
  the canonical status set (D1 in
  `openspec/specs/workspace-decision-register/spec.md` keeps
  toolchain decisions here too).
- Dependency direction:
  `workspace_check.py` and `forge_governance_adapter.py` →
  shared `classify_standalone(...)` helper in
  `workspace_check.py` → `vocabulary.json` and the project
  schema. The helper is importable so the adapter does not shell
  out.
- Release boundary: schema version stays at 1. The new fields are
  optional and additive. No existing declaration becomes
  invalid; `validate_standalone` returns early on absent
  `standalone`/`integrations` blocks.

## 4. Behavioral model

### 4.1 Declaration shape

```json
{
  "standalone": {
    "status": "standalone-with-optional-integrations",
    "local_defaults": ["sqlite", "local-fixtures"],
    "required_external_services": [],
    "required_sibling_projects": [],
    "evidence_ref": "docs/standalone.md",
    "evidence_state": "configured"
  },
  "integrations": [
    {
      "id": "workspace-governance",
      "required": false,
      "evidence_ref": "HANDOFF.md"
    }
  ]
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `standalone.status` | enum (6) | yes | One of `standalone`, `standalone-with-optional-integrations`, `library-dependent`, `workspace-dependent`, `service-dependent`, `blocked`. |
| `standalone.local_defaults` | array of strings | yes | May be empty only when `status` is `library-dependent` or `workspace-dependent`. |
| `standalone.required_external_services` | array of strings | yes | Always present; empty for `standalone`/`standalone-with-optional-integrations`/`library-dependent`. |
| `standalone.required_sibling_projects` | array of strings | yes | Always present; empty for `standalone`/`standalone-with-optional-integrations`/`library-dependent`/`service-dependent`. |
| `standalone.evidence_ref` | string | no | Repository-relative path to the native evidence file; required when `evidence_state` is `configured` or `verified`. |
| `standalone.evidence_state` | enum | no | `declared` / `configured` / `verified` / `unverified`; defaults to `declared` when absent. |
| `integrations[].id` | string (kebab-case) | yes | The provider id (e.g. `workspace-governance`). |
| `integrations[].required` | boolean | yes | `true` ⇒ must be live for the baseline; `false` ⇒ optional. |
| `integrations[].evidence_ref` | string | no | Repository-relative path; required when `required` is `true` and `evidence_state` is `configured` or `verified`. |

### 4.2 Status transition table

| Declared `standalone.status` | `required_sibling_projects` | `required_external_services` | Effective audit status | Findings |
|---|---|---|---|---|
| `standalone` | empty | empty | `standalone` | none |
| `standalone` | non-empty | * | `workspace-dependent` | `STANDALONE_REQUIRED_SIBLING` (error) |
| `standalone` | empty | non-empty | `service-dependent` | `STANDALONE_REQUIRED_SERVICE` (warning) |
| `standalone-with-optional-integrations` | empty | empty | `standalone-with-optional-integrations` | none (only integration warnings) |
| `library-dependent` | empty | empty | `library-dependent` | none |
| `workspace-dependent` | empty | empty | `workspace-dependent` (with `STANDALONE_SHAPE` warning because the field is empty when it should list siblings) | warning |
| `service-dependent` | empty | non-empty | `service-dependent` | none |
| `blocked` | * | * | `blocked` | `STANDALONE_BLOCKED` warning |
| absent | — | — | `blocked` | `STANDALONE_BLOCKED` warning |
| malformed | — | — | `blocked` | `STANDALONE_SHAPE` or `STANDALONE_STATUS_INVALID` error |

### 4.3 Integration resolution

For each `integrations` entry:

| `required` | `evidence_ref` resolves to file | `evidence_state` | Result |
|---|---|---|---|
| `false` | * | * | Optional; no finding unless evidence is malformed |
| `true` | yes | `configured` / `verified` | passes silently |
| `true` | yes | `declared` / `unverified` | `STANDALONE_INTEGRATION_PENDING` warning |
| `true` | no | * | `STANDALONE_INTEGRATION_PENDING` warning (missing evidence file) |
| `true` | absolute path | * | `STANDALONE_ABSOLUTE_PATH` error |

### 4.4 Bounded static scan

`validate_standalone` MAY inspect the following files only (no
transversal, no execution, no globbing outside the project root):

- `Cargo.toml` — `dependencies` and `dev-dependencies` tables,
  matched by `path = "../<sibling>"` (the `workspace-dependent`
  signal).
- `package.json` — `workspaces` field, matched by string
  containing `../<sibling>`.
- `pyproject.toml` — `tool.poetry.dependencies` and
  `tool.uv.sources` matched for `path = "../<sibling>"`.
- `go.mod` — `replace <module> => ../<sibling>` directives.
- `pubspec.yaml` — `path:` dependencies pointing to `../<sibling>`.
- `pom.xml` — `<systemPath>` values starting with `../`.

A detected sibling source path is recorded in the findings list as
`STANDALONE_DETECTED_SIBLING` (warning) but never overrides the
declared `standalone.status` — the declaration remains the source
of truth. The scan is a clue, not a verdict.

### 4.5 Secret scanning

The audit calls the existing `workspace_check.scan_for_secrets` on
the `standalone` and `integrations` blocks before further
processing. Any secret-shaped value is replaced with `[redacted]`
in the finding message and the audit still emits
`STANDALONE_SECRET_VALUE` so the operator can locate it.

## 5. Contract and compatibility

- Schema: `schemas/project.schema.json` gains an optional
  `standalone` property and an optional `integrations` array;
  both are absent from the required list so existing
  declarations remain valid. `$defs` gains
  `standalone_status` and `integration` refs.
- Vocabulary: `vocabulary.json` gains `standalone_statuses`
  (length 6) and `integration_required_states` (length 2). The
  loader fails closed on a missing/empty/non-array value, exactly
  like the existing `profiles`/`kinds` lists. A duplicate value
  raises.
- Audit: `validate_project` calls `validate_standalone(...)`
  after the existing release-evidence and quality checks. The
  function returns early when neither `standalone` nor
  `integrations` is present so a missing block is just a
  `STANDALONE_BLOCKED` warning, not an error.
- Adapter: `evaluate(...)` calls `classify_standalone(...)`
  (imported from `workspace_check`) and adds four new keys to the
  observation document. The order is:
  `provider`, `protocol_version`, `project_id`, `status`,
  `evidence`, `detail`, `metadata`, plus
  `source_revision` (when present), plus
  `standalone_status`, `standalone_observed_revision`,
  `standalone_integrations`, `standalone_findings`. The status
  field, the protocol_version field, the exit code, and the
  redaction behaviour are unchanged.
- `--decisions` and the Jenkins manifest behaviour are
  untouched: this change does not promote `deployable` and does
  not require a sibling to be live for deployment.

## 6. Failure and boundary policy

| Case | Behaviour |
|---|---|
| No `standalone` block | warning `STANDALONE_BLOCKED`; status `blocked` |
| `standalone.status` not in canonical 6 | error `STANDALONE_STATUS_INVALID`; status `blocked` |
| `standalone` not an object / array fields not arrays of strings | error `STANDALONE_SHAPE`; status `blocked` |
| Absolute path in any field | error `STANDALONE_ABSOLUTE_PATH`; status unchanged |
| `..` in `evidence_ref` | error `STANDALONE_PATH_ESCAPE`; status `blocked` |
| `required_sibling_projects` non-empty | error `STANDALONE_REQUIRED_SIBLING` per id; status `workspace-dependent` |
| `required_external_services` non-empty | warning `STANDALONE_REQUIRED_SERVICE` per service; status `service-dependent` |
| Optional integration offline | warning `STANDALONE_OPTIONAL` |
| Required integration with missing/unresolvable evidence | warning `STANDALONE_INTEGRATION_PENDING` |
| Secret-shaped value in any field | error `STANDALONE_SECRET_VALUE` (redacted in evidence) |
| Audit root missing / read failure | adapter exits 3; no standalone fields emitted |
| `evidence_ref` points outside the project root | error `STANDALONE_PATH_ESCAPE` |

## 7. Verification oracle

- `tests/test_standalone_project_boundary.py` (unittest, tmp
  fixtures) covers:
  - absent `standalone` block ⇒ `blocked` + `STANDALONE_BLOCKED`
  - well-formed `standalone` ⇒ matching status, no findings
  - `standalone-with-optional-integrations` with non-empty
    `integrations` ⇒ no findings, integration entries captured
  - `library-dependent` with non-empty `local_defaults` ⇒
    `library-dependent`
  - `workspace-dependent` (siblings declared) ⇒ error per sibling
  - `service-dependent` (services declared) ⇒ warning per service
  - `blocked` declared ⇒ `STANDALONE_BLOCKED` warning
  - unknown status ⇒ `STANDALONE_STATUS_INVALID` error
  - `standalone` not an object ⇒ `STANDALONE_SHAPE` error
  - `local_defaults` not an array ⇒ `STANDALONE_SHAPE` error
  - absolute path in `evidence_ref` ⇒ `STANDALONE_ABSOLUTE_PATH`
    error
  - `..` in `evidence_ref` ⇒ `STANDALONE_PATH_ESCAPE` error
  - `evidence_ref` missing on disk ⇒ `STANDALONE_EVIDENCE_MISSING`
    warning
  - secret-shaped value ⇒ `STANDALONE_SECRET_VALUE` error,
    redacted in evidence
  - bounded static scan finds `path = "../sibling"` in
    `Cargo.toml` ⇒ `STANDALONE_DETECTED_SIBLING` warning
  - adapter: project with no `standalone` block ⇒
    `standalone_status: "blocked"`, four new fields present
  - adapter: project with `standalone-with-optional-integrations`
    ⇒ `standalone_status: "standalone-with-optional-integrations"`
    and `standalone_integrations` list with one entry
  - adapter: audit crash ⇒ no `standalone_*` fields
  - adapter: secret-scrubbing applies to `standalone_findings`
  - adapter: read-only guarantee (hash before/after identical)
  - vocabulary loader: missing/empty `standalone_statuses` ⇒
    fail; duplicate status ⇒ fail
- `python3 scripts/workspace_check.py --self-check` — 0 errors,
  0 warnings for the governance repo (it carries no
  `standalone` block).
- Portfolio audit (`--root .. --audit --json`) is unchanged in
  shape: the new finding codes only fire on declarations that
  include a `standalone` block or an `integrations` array, and
  the governance repo does not opt in.
- `python3 scripts/auto_gate.py --self-check --no-deployment` —
  PASS.
- `python3 scripts/jenkins_manifest.py --check` — unchanged
  (no deployable effect).
- `python3 scripts/forge_governance_adapter.py` against a
  fixture with a `standalone-with-optional-integrations`
  declaration — produces the four new fields and the existing
  status remains `pass`/`blocked`/`unknown`/etc.
- `node scripts/check-openspec-change-names.mjs` — the active
  change name `standalone-project-boundary` is valid.
- `openspec validate --changes --strict --no-interactive` —
  passes.
- `git diff --check` — clean before the HANDOFF-only commit.

## 8. Decision ledger

| # | Decision | Choice | Alternatives rejected |
|---|---|---|---|
| L1 | Status vocabulary | 6 canonical values in `vocabulary.json` | ad-hoc strings in `.project.json` (drift) |
| L2 | Default for missing block | `blocked` + warning | `standalone` (would silently bless unverified projects) |
| L3 | Required siblings | errors, status `workspace-dependent` | warnings (would hide donation blockers) |
| L4 | Required services | warnings, status `service-dependent` | errors (the project may still be open-source donation-ready) |
| L5 | Bounded static scan | read-only, warning only, never overrides declared status | full grep (false positives, scope creep) |
| L6 | Optional integrations | warnings, never block the declared baseline | errors (would block donation when a provider is offline) |
| L7 | Secret redaction | reuse `scan_for_secrets` + `[redacted]` | a new secret vocabulary (drift) |
| L8 | Forge adapter | additive fields, no breaking change | new endpoint (violates v0.1.0 contract) |
| L9 | Promotion / deployment | untouched | coupling `standalone_status` to `deployable` (mixes concerns) |
| L10 | Initializer | unchanged | auto-inject a `standalone` block (would silently change declared status) |
| — | Blockers | none | — |

## Traceability

| Req | Proposal outcome | Design section | Boundary | Scenarios | Oracle |
|---|---|---|---|---|---|
| STB-001 classify | What Changes #1 | §4.1–4.4 | project schema, vocabulary, workspace_check.validate_standalone | local / optional / library / sibling / service / blocked | §7 unit tests |
| STB-002 portable paths | What Changes #1 | §4.4, §6 | workspace_check path validators | absolute / relative / escape | §7 path tests |
| STB-003 optional integrations | What Changes #1 | §4.3, §6 | workspace_check.validate_standalone | optional / required | §7 integration tests |
| STB-004 canonical statuses | What Changes #1 | §4.1, §6 | vocabulary + schema | unknown status / shape | §7 status tests |
| STB-005 adapter | What Changes #1 | §5 | forge_governance_adapter | optional / crash / secret | §7 adapter tests |
| STB-006 secret redaction | What Changes #1 | §4.5, §6 | workspace_check.scan_for_secrets | secret in field | §7 secret test |
| STB-007 no execution | What Changes #1 | §1, §2 | audit is read-only | read-only hash | §7 read-only test |
