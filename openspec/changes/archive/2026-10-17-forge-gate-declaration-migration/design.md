# Design: Declaration-aware Gate evidence with one read projection

## 1. Implementation boundary

The implementation is confined to `/home/paul/code/forge`. Driftwatchdog remains the only Gate policy and execution engine. Forge first validates declarations without side effects, then invokes Driftwatchdog at most once, captures its structured result and process exit, adds context-backed attribution, persists additive evidence, and projects it for existing consumers.

Exact Forge boundaries:

| Boundary | Symbols and responsibility |
|---|---|
| `src/core/manifest.rs` | Add public `VerificationMeta`, `VerificationCheck`, `DeclaredImplementationSource`, and `ImplementationRoot`; inspect the top-level YAML mapping to preserve absent versus explicit `null`; add `Manifest::verification`; parse and validate the optional block with `parse_verification`/`validate_verification` |
| `src/gate/model.rs` | Extend `GateCheck`, `GateEvidence`, `MappedDocument`, and `GateCheckState`; add `ImplementationSource`, private-field `GateDeclarationContext`, `PackExpectation`, `PackExpectationSource`, `GateFreshnessAssessment`, `GateCheckView`, `GateEvidenceView`, and `GateStatusProjection` |
| `src/gate/declaration.rs` (new, exported by `src/gate/mod.rs`) | Own `preflight_gate_declarations`, `load_verification_floor`, `resolve_implementation_source`, `resolve_pack_expectation`, `selected_gate_manifest`, typed declaration errors, and confined root/path resolution |
| `src/gate/projection.rs` (new, exported by `src/gate/mod.rs`) | `assess_evidence_freshness`, `project_gate_evidence`, `read_gate_status`, and human summary/rendering from the shared validated declaration context |
| `src/gate/engine.rs` | Change `run_gate` to require `&GateDeclarationContext`; parse required structured runtime fields, capture exact process exit, attribute from the validated context before persistence, preserve `GateAggregate` as compatibility projection, and never use attribution to change runtime facts |
| `src/cli/functions_13.rs` | Distinguish absent from invalid `forge.yaml`; preflight before dry/real run; use `read_gate_status` for status and `GateStatusProjection` for output; the real run returns captured runtime exit while status retains read-health exit behavior |
| `src/doctor/model.rs`, `src/doctor/runner.rs`, `src/doctor/probes.rs` | Add optional `gate: GateStatusProjection` to `DoctorReport`; resolve it once and derive the `gate-evidence` finding from it |
| `src/checker/mod.rs` | Add optional top-level `gate: GateStatusProjection` to `CheckerDocument`, copied from `DoctorReport`; keep alert derivation unchanged so passing rows do not become alerts |
| `src/release/engine/prepare.rs` | Consume `read_gate_status` and its freshness assessment instead of independently loading/classifying evidence |
| `src/api/admin/gate.rs` | Delete local `evidence_envelope`; preflight before POST spawn, pass context to `run_gate`, and return the Core projection on status/run paths |
| `src/contract/mod.rs`, `src/contract/contract_tests.rs` | Make Gate row mapping total for `ReviewRequired`/`Unbound` and legacy/current states; preserve explicit blocked-aggregate precedence for `platform.gate-result/1.0.0` |
| `frontend/app.js` | Append runtime source, severity, implementation source/path, and freshness reasons as text in existing Gate facts/rows; add no control |
| `src/generate/request.rs` | Add the exact multiline empty verification block to `manifest_text` |
| `src/import/detect.rs` | No production behavior change; lock omission of verification in `build_manifest_text` with a regression test |
| Root `forge.yaml` | Add schema-1 Forge identity and the exact nine-row verification floor in section 9 |
| Tests/docs/spec deltas | Extend Gate/manifest/preflight/cross-surface/web/generation/import/release/contract tests and fixtures; update README, architecture and CHANGELOG; promote `gate-declaration-migration`, modified `gate-runtime-evidence`, and modified `platform-contract-consumption` deltas |

Do not edit sibling repositories, Driftwatchdog manifests/commands, canonical specs before archive promotion, MCP, registry/import discovery, or workbench controls. The no-`forge.yaml` fixture exercises only direct `forge gate PATH` and `forge gate status PATH` resolution.

## 2. Language and runtime

Forge is Rust edition 2021 with MSRV 1.88 (`Cargo.toml`) and existing serde/serde_json/serde_yaml-based contracts. The workbench is the existing framework-free JavaScript in `frontend/app.js`; no bundler or new frontend dependency is introduced. Add the Rust `toml = "0.8"` workspace dependency only to parse the selected Driftwatchdog TOML manifest as typed data; YAML continues through `serde_yaml`, and JSON through `serde_json`. No sibling library or runtime process is added.

Implementation follows existing bounded strings (`MAX_NOTE_CHARS`), typed `ForgeError`, argument-array process invocation, atomic evidence persistence, and thin transport conventions. Verification commands are listed in section 8 and tasks phase 4; implementation work does not change their semantics.

## 3. Ownership and shared code

Driftwatchdog owns the five current row words, row result semantics, aggregate computation, blocked bit, categorized lists, and process exit. workspace-governance owns `.project.json` and the `workspace-governance` provider implementation. Forge owns schema-1 `forge.yaml`, its additive persisted evidence, and the shared read projection.

`src/gate/declaration.rs` is a Forge adapter over sibling-owned declarations, not a copy of sibling policy. It reads explicit files beneath the selected project and the already stored governance selection through `crate::governance::load_config`. `src/gate/projection.rs` is the sole owner of read-time freshness and wire-ready row projection. CLI, doctor, checker, release, API, and frontend do not implement their own pack precedence, attribution, or freshness branch.

No shared crate is extracted: Driftwatchdog already owns runtime vocabulary, workspace-governance already owns workspace declarations, and the new schema/projection is specific to Forge's manifest and evidence lifecycle. Dependency direction is read-only sibling contract → Forge adapter → Forge projection → transports. Release/version boundaries remain independent and no sibling file is written.

## 4. User experience and interface

### Existing flows

- `forge gate PATH` first calls `preflight_gate_declarations`. Only a valid context may reach Driftwatchdog; the result is projected, persisted and journaled as before, and the command exits with captured runtime exit. Invalid declarations cause no spawn or write.
- `forge gate status PATH` calls `read_gate_status`, which preflights all current declarations before loading or short-circuiting on evidence. Valid declarations plus no evidence produce `unverified`; malformed declarations return their typed error.
- `forge doctor PATH` obtains the same preflighted projection once, includes it as optional `gate`, and derives the existing `gate-evidence` finding. Row `absent`/`unrecorded` values do not mutate runtime policy.
- `forge check` copies the doctor projection into additive top-level `gate`; existing finding-to-alert behavior remains, so a passing row is never transformed into an alert.
- Release preflight calls the same declaration-first read projection. It does not run Gate and cannot cite stale or declaration-invalid evidence.
- The session-gated admin GET uses declaration-first `read_gate_status`; POST preflights then passes the context to `run_gate`. The workbench appends text and keeps its existing re-run control.

### UI states and accessibility

Initial/empty is the existing `Unverified` state. Loading, request error, unavailable runtime, stale evidence, and successful refresh keep their existing notice and recovery paths. A row displays, in order: id, state, runtime source or `unrecorded`, severity or `unrecorded`, implementation source, optional implementation path, then note. Freshness reasons appear as text under the existing facts. No new action, confirmation, permission, destructive state, or focusable element is introduced.

Rows remain semantic list items; facts remain a `dl`; no information is color-only. Added text wraps in the existing card at desktop and 390 CSS pixels with no horizontal overflow. Keyboard order and focus restoration remain unchanged because no node is interactive. No animation is added, so reduced-motion behavior is unchanged. Supported browser evidence is the repository's existing Chromium contract at desktop and 390-pixel viewport.

## 5. Behavioral model

### 5.1 Exact manifest schema

`verification` is optional on schema-1 `forge.yaml`. If present it is exactly:

```yaml
verification:
  rule_pack: workspace-governance@1 # optional
  checks:                            # required, may be empty
    - id: governance-quality
      source: bound
      root: governance-workspace     # optional as a root/path pair
      path: workspace-governance/scripts/workspace_check.py
```

Rust model:

```rust
pub struct VerificationMeta {
    pub rule_pack: Option<String>,
    pub checks: Vec<VerificationCheck>,
}

pub struct VerificationCheck {
    pub id: String,
    pub source: DeclaredImplementationSource,
    pub root: Option<ImplementationRoot>,
    pub path: Option<String>,
}

pub enum DeclaredImplementationSource { Native, Bound, PackDefault, Exempt }
pub enum ImplementationRoot { Project, GovernanceWorkspace }
```

Wire words use kebab case: source is exactly `native|bound|pack-default|exempt`; root is exactly `project|governance-workspace`. `verification` permits only `rule_pack` and `checks`; each row permits only `id`, `source`, `root`, and `path`. `checks` is required and an array. `rule_pack`, when present, must be a non-empty string with no leading/trailing whitespace or control character. An id must match `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$` and be unique in the array.

Root and path are either both absent or both present. Only `native` and `bound` may carry them; `pack-default` and `exempt` with either field are invalid. `native` and `bound` without a pair are valid because in-process/command implementations may have no single filesystem file.

A path is lexically valid only if it is non-empty UTF-8 using `/` separators, does not start or end with `/`, contains no `//`, `\\`, NUL/control character, URL/drive prefix, or component equal to `.` or `..`, and `Path::is_absolute()` is false. `Manifest::parse` performs only these byte/schema checks. During declaration preflight, every existing declared candidate is canonicalized with its declared root and must remain under that canonical root; a symlink escape is `manifest-invalid`, not `absent`, and occurs before any runtime spawn.

`Manifest::parse` first parses the document as a top-level `serde_yaml::Value` mapping and records whether the exact `verification` key exists before deserializing `RawManifest`; `RawManifest` does not use `Option<serde_yaml::Value>` to represent presence. Absent key becomes `None`. A present `null`, scalar, or sequence returns `manifest-invalid` naming `verification`; a mapping without `checks` returns it naming `verification.checks`. `parse_verification(path, value)` then validates mappings and fields so every syntactic failure names the `forge.yaml` path and exact field such as `verification.checks[2].path`; duplicate ids name the later id. The attribution preflight uses that path plus row index for physical symlink-escape errors. No partial `Manifest` is returned, no process starts, and no file is changed.

### 5.1.1 Declaration preflight and side-effect boundary

`preflight_gate_declarations(project_root: &Path) -> Result<GateDeclarationContext, ForgeError>` is the only constructor of `GateDeclarationContext`; its fields are private so `run_gate` cannot receive an unchecked context.

```rust
pub struct GateDeclarationContext {
    verification: Option<VerificationMeta>,
    pack_expectation: PackExpectation,
    path_resolutions: BTreeMap<String, DeclaredPathResolution>,
}

enum DeclaredPathResolution {
    Present { stable_path: String },
    Absent { stable_path: String },
}
```

`path_resolutions` contains exactly the floor ids that declare a root/path pair; a pathless floor id is read from `verification`, and no runtime-source fallback entry is precomputed. Preflight then performs these steps in order:

1. checks whether `<project>/forge.yaml` exists; absence is a valid empty floor, while presence calls `Manifest::load_from_dir` and propagates `manifest-invalid` rather than treating failure as absence;
2. validates the presence-preserving verification shape and every lexical field rule;
3. parses every existing relevant declaration: the whole `.project.json` as JSON and the one Gate manifest selected in Driftwatchdog order as TOML/YAML, even when a higher-precedence valid pack identity will answer; malformed `.project.json` or selected Gate data returns `ForgeError::GateInvalid` (`gate-invalid`) naming its relative path and exact field;
4. for each project-root floor path, validates confinement of any existing candidate; for a governance-root floor row only, loads the stored governance config and either records missing/disabled/wrong-provider/root/file as that row's future `absent` resolution or propagates malformed config as `governance-invalid`; and
5. computes the pack expectation from the already parsed valid documents and stores the typed floor, expectation, and per-floor path resolution in the context.

CLI dry-run/real-run and admin POST call preflight before `run_gate(dir, project_id, config, dry_run, &context)`. `run_gate` performs no declaration read and cannot spawn without the validated context. CLI/admin status, doctor/checker, release and API GET call `read_gate_status`, which performs this same preflight before it loads evidence; its internal projection functions reuse the returned context and do not parse declarations twice. Therefore malformed declarations win over absent evidence. Any preflight error leaves a runtime-stub invocation marker, prior evidence, operations journal and Driftwatchdog/sibling history byte-identical.

### 5.2 Runtime capture and compatibility aggregate

Driftwatchdog's current row vocabulary maps exactly:

| Raw current row status | New `GateCheckState` | Projected word |
|---|---|---|
| `PASS` | `Pass` | `pass` |
| `FAIL` | `Fail` | `fail` |
| `REVIEW_REQUIRED` | `ReviewRequired` | `review_required` |
| `NOT_APPLICABLE` | `NotApplicable` | `not_applicable` |
| `UNBOUND` | `Unbound` | `unbound` |
| any other current-runtime string, including `SKIP` | `Unresolved` | `unresolved`, with the raw string in the bounded note |

The serialized old evidence state `"skip"` continues to deserialize to the compatibility-only `GateCheckState::Skip` and project as `skip`. `GateCheckState::from_raw` has no `SKIP` arm, so no newly parsed runtime result can create it.

A current status document must have string `status`, boolean `blocked`, arrays of strings `failures`, `pending_reviews`, `not_applicable`, and `unbound`, and array `results`. Every result requires non-empty string `gate_id`, string `source`, string `status`, and string `severity`. Malformed required structured fields make the runtime answer `GateOutcome::Unavailable`; prior evidence remains byte-identical. Unknown but well-formed status/source/severity strings are retained; only status becomes `Unresolved`.

`MappedDocument` carries all raw fields. A normal process exit must provide an integer code in `0..=255`; a signal/no-code result is unavailable because Forge cannot mirror an exact code. New evidence stores all fields below; old records use serde defaults:

```rust
pub struct GateCheck {
    pub id: String,
    pub state: GateCheckState,
    pub runtime_source: Option<String>,
    pub severity: Option<String>,
    pub implementation_source: Option<ImplementationSource>,
    pub implementation_path: Option<String>,
    pub note: Option<String>,
}

pub enum ImplementationSource {
    Native, Bound, PackDefault, Exempt, Absent, Unbound, Undeclared,
}

pub struct GateEvidence {
    // existing fields unchanged
    pub runtime_status: Option<String>,
    pub runtime_blocked: Option<bool>,
    pub runtime_exit_code: Option<i32>,
    pub failures: Vec<String>,
    pub pending_reviews: Vec<String>,
    pub not_applicable: Vec<String>,
    pub unbound: Vec<String>,
}
```

New records always set the three runtime options. The arrays serialize when non-empty and default empty. Existing `contract` remains `0.1.0`. `GateAggregate` keeps the existing `classify_aggregate(status, blocked, exit_success)` mapping solely for compatible old callers: blocked → `Blocked`; unblocked `PASS` plus zero exit → `Passed`; unblocked `FAIL` → `Failed`; every other/contradictory combination → `Unknown`. It never drives the real run command exit. `forge gate` converts the captured `0..=255` code directly to `ExitCode`; persistence, journal summary, doctor, release, and UI may classify their own Forge observation but cannot alter the captured runtime facts.

#### Platform Gate projection

`src/contract/mod.rs::map_gate_check_state` remains total over every enum variant: `Pass → passed`, `Fail → failed`, `ReviewRequired → review_required`, `Unresolved → review_required`, `Unbound → unbound`, `NotApplicable → skipped`, and compatibility-only `Skip → skipped`. `map_gate_result` maps `Passed → passed`, `Failed → failed`, `Unknown → review_required`, and `Blocked → failed` only when at least one row is `Fail`, otherwise `review_required`. An `Unbound` row remains `unbound` at row level and never by itself selects the root result. Contract fixtures assert every row variant and all blocked/non-blocked aggregate branches.

### 5.3 Deterministic implementation attribution

For every runtime row, `resolve_implementation_source(row, &GateDeclarationContext)` applies this ordered algorithm without reading declarations or the filesystem after spawn:

1. Find an exact, case-sensitive `verification.checks[].id` match in the context. No fuzzy/prefix match.
2. If found:
   - `exempt` → `exempt`, no path.
   - `pack-default` → `pack-default`, no path.
   - `native` or `bound` without root/path → the declared word, no path.
   - `native` or `bound` with a preflighted existing confined regular file → the declared word and stable placeholder path.
   - `native` or `bound` whose preflighted project/provider root or candidate is missing, disabled, wrong-provider, or non-file → `absent` and exactly one stable placeholder path.
3. With no floor entry: row state `Unbound` → `unbound`; else row state `NotApplicable` → `exempt`; else exact runtime source `adapter:project-runtime` → `bound`; else exact runtime source `driftwatchdog` → `native`; else → `undeclared`.

Stable path labels are `<project>/<relative-path>` and `<governance-workspace>/<relative-path>`, with one separator and the validated relative path. Absolute host paths are never persisted or rendered. Forge never inspects diagnostics, remediation, findings, evidence, missing-evidence, commands, or notes. No runtime-source value maps to `pack-default`; only an exact floor entry may produce it. An `absent` row changes only implementation attribution/path; row state, runtime status, blocked bit, category lists, aggregate projection, and exit code are byte-for-byte the captured values.

### 5.4 Pack expectation and freshness

`resolve_pack_expectation` runs inside declaration preflight over the already parsed valid documents and returns identity plus source and relative provenance:

```rust
pub struct PackExpectation {
    pub identity: Option<String>,
    pub source: PackExpectationSource,
    pub provenance: Option<String>,
}

pub enum PackExpectationSource {
    ForgeManifest,
    WorkspaceDeclaration,
    GateManifest,
    Undeclared,
}
```

Wire words are `forge-manifest|workspace-declaration|gate-manifest|undeclared`. Resolution stops at the first non-empty declaration:

1. Existing valid `forge.yaml` `verification.rule_pack`; provenance `forge.yaml`.
2. Existing valid `.project.json` string `verification.rule_pack`; provenance `.project.json`.
3. First existing regular file in Driftwatchdog order: `gate.toml`, `.driftwatch/gate.toml`, `.ai-gate/gate.yaml`. TOML reads root string `rule_pack_version`; YAML reads root string `rule_pack`. Provenance is the selected relative file.
4. `identity: None`, source `undeclared`, no provenance.

A higher-precedence valid document with no rule-pack field falls through. Preflight nevertheless validates each existing relevant declaration before precedence: malformed `.project.json` or selected Gate JSON/TOML/YAML returns `gate-invalid` naming relative path and exact field, even if a valid higher tier has an identity; malformed Forge verification is `manifest-invalid`. Selection never searches a parent, environment variable, registry, or sibling.

`assess_evidence_freshness` first preserves the current revision/aggregate rule:

- exact non-empty evidence/current revisions and aggregate `Passed|Blocked|Failed` → `fresh`;
- missing evidence revision → `stale`, reason `evidence revision unrecorded`;
- unavailable current revision → `stale`, reason `current revision unavailable`;
- unequal revisions → `stale`, reason `revision mismatch: evidence <E>, current <C>`;
- aggregate `Unknown` → `stale`, reason `aggregate unknown`.

Then apply pack expectation:

- Declared expected identity equals `evidence.rule_pack_version` → no added reason.
- Declared expected identity differs or observed identity is missing → force `stale` and append exactly `rule pack mismatch: expected <EXPECTED> from <SOURCE> (<PROVENANCE>), observed <OBSERVED>`, using `<unrecorded>` for missing observed identity.
- Undeclared expectation → do not change the revision/aggregate result and append exactly `pack expectation undeclared`.

Reasons remain in that deterministic order with no duplicate. `GateFreshnessAssessment` carries `state`, `reasons`, and `pack_expectation`. An undeclared expectation never proves which pack was intended and never makes otherwise fresh evidence stale.

### 5.5 One shared read projection

`project_gate_evidence(evidence, current_revision, &GateDeclarationContext)` produces `GateEvidenceView` without further declaration I/O. Public `read_gate_status(project_root, project_id, current_revision) -> Result<GateStatusProjection, ForgeError>` first calls `preflight_gate_declarations`, then loads optional evidence, and finally projects with that context. Declaration failure therefore precedes both evidence loading and the absent-evidence branch.

`GateCheckView` preserves `id`, `state`, optional runtime source/severity/path/note, and has a required string `implementation_source`; missing stored source becomes `unrecorded` without changing the stored record. `GateEvidenceView` contains every existing evidence field, all additive runtime fields/category lists, projected checks, and `freshness`, `freshness_reasons`, and `pack_expectation`. Only valid declarations plus absent evidence produce `GateStatusProjection { gate: None, freshness: absent, detail: \"the gate has never run; unverified is not a pass\" }`.

CLI JSON/human, `DoctorReport.gate`, `CheckerDocument.gate`, release preflight, API status/run envelopes, and frontend consume this projection. Doctor derives its one Gate finding from the projection; checker copies it and omits passing findings from alerts. Release uses projected freshness/aggregate; API has no local envelope builder. A caller that already holds a run-path context passes it to the internal projector, so no operation parses declarations twice.

### 5.6 No-manifest direct Gate boundary

`resolve_gate_target` still accepts an explicit directory and may derive identity from valid `forge.yaml`, `.project.json`, or directory name. It MUST NOT treat a present invalid Forge manifest as authorization to run: the mandatory preflight catches it before status evidence load or runtime spawn. The cargo-shaped fixture contains valid `.project.json`, `.ai-gate/gate.yaml`, and a runtime stub but no `forge.yaml`; absence is a valid empty floor. Only direct Gate run/status is asserted.

## 6. Contract, migration, and compatibility

- Gate evidence remains `0.1.0`, manifest schema remains `1`, checker remains `forge-checker/0.1.0`, and platform Gate result remains `platform.gate-result/1.0.0`. Owning surfaces add optional fields while preserving every pre-existing field/value meaning; unaffected documents remain byte-identical and old tolerant consumers remain valid.
- Old evidence missing runtime source, severity, implementation source/path, raw runtime facts, and categorized arrays deserializes through defaults. Read projection emits implementation source `unrecorded`; it does not infer from old state/note and does not rewrite evidence.
- Old `state: "skip"` deserializes and maps to platform `skipped`; current `SKIP` input is unknown/unresolved, maps to platform `review_required`, and no newly persisted record serializes `skip` from runtime parsing.
- Manifests with no `verification` key parse as before. A present `verification: null` is intentionally not equivalent to absence and is invalid.
- `forge new` adds exactly:

  ```yaml
  verification:
    checks: []
  ```

  with the generator's deterministic trailing newline. Accepted `forge import` continues to create its normal minimal `forge.yaml`; `build_manifest_text` remains free of `verification:`.
- Existing evidence persistence path, atomic write behavior, journal operation name, runtime resolution, dry-run/rehearsal behavior, and read authorization remain unchanged after successful preflight. A failed preflight touches none of them.
- `GateAggregate` and serialized words remain compatible, but command-run exit uses `runtime_exit_code`. Platform projection follows the explicit total row/aggregate mapping in section 5.2; status/read/release use shared freshness without overwriting raw fields.

## 7. Failure and boundary policy

| Case | Required result |
|---|---|
| Absent `forge.yaml` or absent `verification` key | Valid empty floor; continue declaration validation and pack precedence |
| Present `verification: null`, scalar, sequence, or mapping without `checks` | `manifest-invalid` naming `verification` or `verification.checks`; no read short-circuit, spawn, evidence or journal write |
| Unknown/duplicate verification field/id; invalid source/root/path; symlink escape | `manifest-invalid` naming `forge.yaml` and exact field; invocation marker, prior evidence, journal and sibling history unchanged |
| Malformed `.project.json` or selected Gate JSON/TOML/YAML | `gate-invalid` naming relative path and exact field, even with no evidence or a higher valid pack identity; no fallback/spawn/write |
| Missing project/provider root or declared file | Preflight records only the matching row's future `absent` source and one stable path; runtime facts/other rows unchanged after a run |
| Missing/disabled/wrong governance provider or no stored `workspace_root` | Governance-root row resolves `absent`; no environment, parent, adapter-path or sibling fallback |
| Malformed governance selection while a governance-root row exists | `governance-invalid` before read/spawn; without such a row governance config is not loaded |
| Valid declarations and absent evidence | Shared unverified/absent projection |
| Malformed runtime structured document/list/row after valid preflight | `GateOutcome::Unavailable`; prior evidence byte-identical |
| Unknown current row status | `unresolved`, raw word in bounded note; raw top-level facts/exit retained |
| Unknown structured runtime source | `undeclared` unless an exact floor entry wins |
| Runtime diagnostic/note contains source-like words | No effect on attribution |
| Process has no numeric exit code | Unavailable because exact exit cannot be mirrored; prior evidence unchanged |
| Pack declaration mismatch/missing observed pack | Stale with exact expected/source/provenance/observed reason |
| No pack declaration | Existing revision/aggregate freshness unchanged plus `pack expectation undeclared` |
| Missing old evidence fields | Readable; implementation source `unrecorded`, raw options absent, category lists empty; no rewrite |
| Passing row in checker | Present under top-level `gate`; no row alert synthesized |
| API/frontend unavailable | Existing error recovery; no fabricated pass and prior cached evidence remains |

There is no retry, background work, remote write, parent search, rollback across repositories, or partial declaration migration. The only new filesystem write in product behavior is the explicitly requested root `forge.yaml`; normal Gate evidence writes retain the existing atomic path.

## 8. Verification oracle

Red tests are written before implementation and must fail for the intended reason:

- `src/gate/engine_tests.rs`: `runtime_row_words_map_one_to_one`, `skip_is_legacy_decode_only`, `runtime_source_severity_and_raw_outcome_are_preserved`, `unknown_row_word_is_unresolved_with_raw_word`, `runtime_exit_is_not_changed_by_absent_attribution`, `old_evidence_projects_unrecorded`.
- Manifest tests: `verification_presence_distinguishes_absent_from_null`, `verification_null_scalar_sequence_and_missing_checks_are_invalid`, `verification_schema_accepts_closed_words`, `verification_schema_rejects_unknown_duplicate_and_invalid_fields`, `verification_paths_are_confined`, `manifest_without_verification_remains_valid`.
- Preflight/declaration tests: `invalid_declaration_prevents_spawn_and_all_writes`, `read_preflight_precedes_absent_evidence`, `declaration_error_codes_name_relative_path_and_field`, `floor_entry_wins_over_runtime_source`, `attribution_fallback_is_exact_and_never_reads_notes`, `governance_root_uses_only_enabled_stored_selection`, `missing_declared_file_marks_only_one_row_absent`, `pack_expectation_precedence_matches_driftwatchdog`, `undeclared_pack_does_not_stale_revision_fresh_evidence`, `pack_mismatch_names_expected_observed_and_source`.
- Contract tests: `gate_check_state_mapping_is_total`, `blocked_gate_result_requires_failed_row_for_failed_root`, `unbound_row_stays_unbound_without_inventing_root_verdict`, and an old-consumer fixture that accepts additive Gate/checker fields while all previous values remain unchanged.
- `tests/gate_cross_surface.rs`: `all_gate_readers_use_the_same_projection`, `malformed_declaration_wins_over_absent_evidence_everywhere`, `checker_carries_rows_without_passing_alerts`, `release_uses_shared_pack_freshness`, `legacy_projection_is_unrecorded_everywhere`, `cargo_shape_direct_gate_needs_no_forge_manifest`.
- Generation/import tests: `new_manifest_contains_empty_verification_floor`, `accepted_import_manifest_omits_verification`.
- `tests/web_gate_controls_contract.rs`: `gate_rows_render_attribution_as_text_without_new_controls`, including desktop and 390-pixel no-overflow/static accessibility assertions.
- Root-floor contract test: parse root `forge.yaml` and assert the exact nine ids/sources/root/path values in section 9.

Focused commands after implementation:

```text
cargo fmt --check
cargo test --lib gate
cargo test --test gate_cross_surface
cargo test --test web_gate_controls_contract
cargo test --workspace --all-targets
cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
node scripts/check-openspec-change-names.mjs
openspec validate forge-gate-declaration-migration --strict --no-interactive
openspec validate --all --strict --no-interactive
git diff --check
forge gate --dry-run
forge gate --timeout-secs 1800
```

The success smoke uses a stub returning all five statuses plus unknown, categorized lists and exit 1; stored JSON, platform projection and every read surface retain the specified facts/mappings and command exit 1. The failure smoke gives the stub an invocation-marker path, snapshots prior evidence/journal/sibling-history bytes, and supplies each invalid declaration class; preflight returns the exact typed error, the marker is absent and all snapshots are unchanged. Read-path matrices repeat malformed declarations with no evidence and prove the error wins over unverified. The pack matrix proves Forge > workspace > selected Gate manifest > undeclared after all current documents validate.

## 9. Forge root floor

The new root `forge.yaml` is schema 1 and uses project id `forge`, name `Forge`, profile `rust-product`, and this exact verification block:

```yaml
verification:
  rule_pack: workspace-governance@1
  checks:
    - id: repository
      source: bound
    - id: build
      source: bound
    - id: tests
      source: bound
      root: project
      path: scripts/check-tests.sh
    - id: security
      source: bound
    - id: declared-verification
      source: bound
      root: project
      path: .project.json
    - id: governance-quality
      source: bound
      root: governance-workspace
      path: workspace-governance/scripts/workspace_check.py
    - id: placeholder-threshold
      source: bound
      root: project
      path: scripts/governance/product_code_quality.py
    - id: product-code-boundary
      source: bound
      root: project
      path: scripts/governance/product_code_quality.py
    - id: source-file-size
      source: native
```

These are the nine effective checks. Only filesystem-backed rows carry root/path. `governance-workspace` is the stored portfolio root, so the governance implementation path includes the sibling directory segment. The declaration does not create or select a provider; absent stored selection/root/file projects only that row as absent.

## 10. Decision ledger

- **Preflight:** `preflight_gate_declarations` is the only `GateDeclarationContext` constructor; all runs and reads validate declarations before spawn/evidence short-circuit.
- **Manifest presence:** omitted verification is valid; explicit null/scalar/sequence or missing `checks` is `manifest-invalid`.
- **Runtime authority:** Driftwatchdog's current five statuses and top-level fields win. `SKIP` is old-evidence decode only.
- **Aggregate:** retain normalized `GateAggregate` for compatibility; never use it for real run exit.
- **Platform mapping:** row mapping is total; blocked root is failed only with an actual failed row, otherwise review-required; unbound remains row-level unbound.
- **Attribution:** exact floor id first, then exact state/source allowlist. No free-text or inferred `pack-default`.
- **Governance root:** only stored enabled workspace-governance `workspace_root`; malformed config is consulted only for a governance-root row.
- **Pack/read order:** validate all current relevant declarations first; then precedence/freshness; malformed declarations beat absent evidence.
- **Projection:** one Core projection is embedded by every reader; checker adds no per-row alerts.
- **Compatibility:** three deltas promote together; old fields/meanings remain, additive owning fields are allowed, unaffected surfaces remain byte-identical.
- **No-manifest scope:** direct Gate only; absence is valid, presence-invalid is refused.
- **Generation/import:** new emits empty floor; accepted import keeps normal minimal manifest without it.
- **Versions:** Gate evidence `0.1.0`, manifest schema `1`, platform Gate `1.0.0`, checker/API unchanged.
- **Dependencies:** typed TOML parsing is Forge-local/read-only; no sibling dependency.
- **No unresolved implementation decision or blocker.**

## 11. Traceability

| Requirement/scenarios | Proposal outcome / UX | Design decisions | Implementation symbols | Tasks | Exact oracle |
|---|---|---|---|---|---|
| GDM-001; S1-S4 + gate-runtime-evidence MODIFIED current-vocabulary/run scenarios | Preserve runtime authority and exact row/raw facts | 5.2, 6, 7 | `GateCheckState::from_raw`, `MappedDocument`, `GateEvidence`, `run_gate`, CLI/admin run exit | 1.2, 2.1, 2.2, 3.1 | five-word/legacy tests; raw-outcome stub; modified canonical scenario post-archive |
| GDM-002; S1-S7 | Closed presence-preserving verification and zero-side-effect preflight | 5.1, 5.1.1 | top-level YAML mapping parse, `parse_verification`, `preflight_gate_declarations` | 1.3, 1.4, 2.3, 2.4, 3.2 | absent/null/scalar/sequence/missing-checks errors; invocation marker absent; snapshots unchanged |
| GDM-003; S1-S7 | Deterministic implementation source/path | 5.3, 7 | `GateDeclarationContext`, `resolve_implementation_source`, `ImplementationSource` | 1.5, 2.5, 3.3 | attribution matrix; one-row absent; note-poison; no post-spawn declaration I/O |
| GDM-004; S1-S6 | Declaration-first expectation/freshness | 5.1.1, 5.4 | `preflight_gate_declarations`, `resolve_pack_expectation`, `assess_evidence_freshness` | 1.6, 2.6, 3.4 | error-code/path matrix with absent evidence; precedence/mismatch/undeclared tests |
| GDM-005; S1-S5 + gate-runtime-evidence MODIFIED read scenarios | All readers show one validated projection | 4, 5.5 | `GateStatusProjection`, `project_gate_evidence`, `read_gate_status`, doctor/checker/API consumers | 1.7, 2.7-2.10, 3.5 | cross-surface equality; malformed-before-absent; no passing alerts; browser oracle |
| GDM-006; S1-S4 + additive-compatibility MODIFIED scenarios | Additive migration and stable creation/import | 5.2, 6 | serde defaults, `GateCheckView`, generator/import functions | 1.8, 2.11, 3.6 | old fixture unchanged; old tolerant consumer accepts additions; unaffected bytes equal |
| GDM-007; S1-S3 | Direct Gate supports absent manifest only | 5.1.1, 5.6 | `resolve_gate_target`, `preflight_gate_declarations` | 1.9, 2.12, 3.7 | no-manifest run/status; present-invalid prevents marker/write |
| GDM-008; S1-S3 | Forge declares all nine checks | 9 | root `forge.yaml`, preflight/parser | 1.10, 2.12, 3.8 | exact floor/schema/version test |
| platform-contract-consumption MODIFIED mapping scenarios | Total row/aggregate platform mapping | 5.2 platform subsection, 6 | `map_gate_check_state`, `map_gate_result`, `src/contract/contract_tests.rs` | 1.11, 2.13, 3.1, 3.9 | every state; blocked with/without Fail; Unbound row; post-archive contradiction check |
