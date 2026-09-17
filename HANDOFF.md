current_spec: agent-runtime-workflows

# Forge handoff

## Current state

`specification-remediation` implemented, verified and archived on 2026-09-17
as `2026-09-17-specification-remediation`; canonical specs promoted to
[openspec/specs/specification-remediation/spec.md](openspec/specs/specification-remediation/spec.md).
New in this cycle: `src/spec` (versioned `SpecRequest`/`SpecDraft`/
`SpecProvenance`/`SpecGenerateOutcome`/`SpecStatus` contract v0.1.0 with
provenance covering project id, path, profile, source revision (manifest
mtime), finding ids/categories, DriftWatch policy ids and dependencies;
storage layout `.forge/specs/<project>-<short-hash>/` carrying
`proposal.md` / `design.md` / `tasks.md` / `manifest.json`; bounded
proposal size enforced at `MAX_FINDINGS_PER_SPEC = 32`; `generate_spec`
is idempotent on the same project + sorted finding set so an unchanged
finding re-run reports the existing spec without rewriting files
(boundary scenario); `SpecRoute` distinguishes `Deterministic`,
`Semantic` and `Manual` queues and `apply_routing` requires the
deterministic action's evidence (or generated spec, or recorded manual
state) before reporting completion; `route_finding` keeps the doctor
finding's `Manual` remediation class as a judgment call (no project
changes, no AI claim) and routes any finding whose id matches a
known lifecycle action to `Deterministic`; Core errors
`spec-invalid`/`spec-write-failed` with stable codes, CLI
`forge spec generate|list|inspect|route|apply` (human/JSON, structured
`error[spec-invalid]` for empty/oversized/ambiguous requests and the
existing `feature-ownership-conflict` handoff from `project-upgrade-orchestration`
now resolves through `forge spec apply semantic-<feature>` writing the
bounded proposal without changing files), the `SpecRequest` is
validated before any write so a refusal leaves the project untouched,
and existing doctor/upgrade/feature/import/generate/quality-policy
contracts still hold after the new commands run against a generated
project.
as `2026-09-17-quality-policy-integration`; canonical specs promoted to
[openspec/specs/quality-policy-integration/spec.md](openspec/specs/quality-policy-integration/spec.md).
New in this cycle: `src/policy` (versioned `DriftWatchConfig`/default
binary `driftwatch` overridable via `FORGE_DRIFTWATCH_BIN`, bounded
`Command::new` + argument-array invocation with per-run `wait_timeout`
so an unresponsive tool cannot hang the registry; `PolicyReport`/`PolicyFinding`/`PolicySeverity`
contract v0.1.0 with custom deserialization that accepts `info`/`ok` as
aliases for `pass`; project-scoped execution with `current_dir(dir)` and
`--project <dir>`; `PolicyOutcome::Reported` / `Unavailable` so a
missing binary, non-zero exit, timeout or unparseable JSON surfaces as
an `unavailable` finding instead of `pass`; `redact_credentials` and
`redact_report_in_place` covering AWS / GitHub / GitLab / Slack / JWT /
private-key / `key=value` shapes and run on every consumed report as
defense in depth; `observation_is_stale` keyed on `forge.yaml` and
configured driftwatch-file mtimes), Core `PolicyUnavailable`
(`policy-unavailable`), CLI `forge doctor` invokes the adapter and
threads the outcome through `run_doctor(..., Some(&outcome))` with
per-rule `driftwatch-<id>` findings, `driftwatch-policy` rollup and
not-applicable policies preserved with their reason and
`applicable: false`, and existing doctor contract still holds (no
registry/observation/dependency-drift regression on the unchanged
fixtures).

`extended-profile-catalog` implemented, verified and archived on 2026-09-17
as `2026-09-17-extended-profile-catalog`; canonical specs promoted to
[openspec/specs/extended-profile-catalog/spec.md](openspec/specs/extended-profile-catalog/spec.md).
New in this cycle: `ProfileSupportStatus` (`Supported` / `Planned`) on
`ProfileDescriptor`, the sixth supported profile `react-web`
(`adapter-react`, typescript, `npm@20` 0.1.0) with a tested native-buildable
template (`forge new --profile react-web` renders
`forge.yaml`/`README.md`/`index.html`/`src/main.js`/`src/app.test.mjs`/
`package.json`/`scripts/build.mjs`/`vite.config.js`/`Dockerfile`/
`.gitignore`; build `npm run build`, test `npm test`), and seven reserved
specialist candidates (`aspnet-saas`, `flutter-client`, `nextjs-content`,
`python-ai`, `python-data`, `rust-cli`, `rust-worker`) discoverable through
`inspect_profile` with `support_status: planned` but refused by
`resolve_profile` / `preflight_profile` / `generate` with a new
`ForgeError::UnsupportedProfile` (code `unsupported-profile`) before any
file change; the original five MVP ids and their semantics stay intact,
react-web refuses server-side capabilities with the existing backend
boundary hint, `flutter-client` description names the backend boundary
('rust-web'/'python-service') it needs, and CLI `forge profile inspect`
shows the support status plus description in human and JSON output.

`project-upgrade-orchestration` implemented, verified and archived on 2026-09-17
as `2026-09-17-project-upgrade-orchestration`; canonical specs promoted to
[openspec/specs/project-upgrade-orchestration/spec.md](openspec/specs/project-upgrade-orchestration/spec.md).
New in this cycle: `src/upgrade` (pinned
[`UpgradePlan`](src/upgrade/mod.rs) with old/new versions, kind-ordered
steps, asset list, validators, migration strategy and recovery
implications; `apply_upgrade` precondition sweep journals a `blocked`
`upgrade` row and emits a structured `SemanticConflict` naming the
owned file and the suggested `forge spec generate` follow-up;
already-satisfied upgrades are a no-op with no file, manifest or
registry write; missing requested features install at the tested
version; `run_fleet` snapshots the explicit registry selection,
journals each project with `done`/`failed`/`blocked`/`skipped` states,
isolates per-project failures and reports a healthy verdict only when
nothing failed or blocked; retry re-plans from the current manifest so
completed steps are not blindly repeated; `postgres` steps are marked
irreversible with declared strategy
`manifest-repin+manual-schema-review`), Core `record_operation`
append-only journal, CLI `forge upgrade [TARGET] [--feature FEATURE]
[--all] [--dry-run]` (human/JSON, structured `error[unknown-feature]`
and `error[feature-ownership-conflict]`, fleet owns its exit code
without the generic error path), and existing feature-lifecycle
contracts still hold after fleet upgrades (admin depends on auth, both
reach 0.1.0 in dependency order with all manifest sections preserved).

`feature-lifecycle` implemented, verified and archived on 2026-09-16
as `2026-09-16-feature-lifecycle`; canonical specs promoted to
[openspec/specs/feature-lifecycle/spec.md](openspec/specs/feature-lifecycle/spec.md).
New in this cycle: `src/feature` (18-descriptor versioned catalog at
tested `0.1.0` with compatibility derived from the MVP profile
descriptors, dependencies, conflicts, install/upgrade strategies,
validation policies, docs and tests; dependency-ordered deterministic
plans with exact versions; add/remove/upgrade through manifest-only
edits preserving all other sections plus deterministic `.forge/features`
ownership receipts; reverse-dependency and user-edit preflight blocks
with full preservation; atomic writes with restore-on-validation-failure;
registry refresh so source/manifest/registry agree), Core errors
`unknown-feature`/`incompatible-feature`/`feature-ownership-conflict`,
CLI `forge feature list|inspect|resolve|add|remove|upgrade`
(human/JSON, stable `error[code]` diagnostics), and `forge new`
dependency-closure selection (`--feature admin` records `auth`+`admin`)
while preserving the existing `incompatible-profile` contract.

`doctor-maturity-assessment` implemented, verified and archived on 2026-09-16
as `2026-09-16-doctor-maturity-assessment`; canonical specs promoted to
[openspec/specs/doctor-maturity-assessment/spec.md](openspec/specs/doctor-maturity-assessment/spec.md).
New in this cycle: `src/doctor` (read-only finding inventory with stable
rule IDs, PASS/WARN/FAIL/UNAVAILABLE statuses, evidence, applicability and
automatic/AI/manual remediation classes; versioned L0-L4 maturity policy
descriptors with target-gated applicability so L0 prototypes are respected;
registry-observation staleness via manifest-mtime comparison; unknown profile
reported as unavailable findings, never a hard refusal), Core
`--target L0..L4` parsing (`parse_target_level`), CLI `forge doctor [<path>]
[--target]` (human/JSON, exit 0 with `healthy:false` on FAIL/UNAVAILABLE/
unmet/stale; hard errors only for path/manifest IO via existing codes), and
`healthy` defined as no FAIL/UNAVAILABLE, no unmet applicable controls and
no stale observation.

`core-manifest-registry` implemented, verified and archived on 2026-09-16
as `2026-09-16-core-manifest-registry`; canonical specs promoted to
[openspec/specs/core-manifest-registry/spec.md](openspec/specs/core-manifest-registry/spec.md).
Rust workspace `forge` 0.1.0: `src/core` (schema-v1 `forge.yaml`
validation, legacy `platform.yaml` only via explicit `--manifest`),
`src/registry` (SQLite via bundled rusqlite, unique id/canonical-path,
nullable observations, pending→done/failed operation journal with
open-time reconciliation), `src/main.rs` (thin CLI: `list`, `inspect`,
`register`, human/JSON output, stable `error[code]` diagnostics, exits
0/1/2). Foundation decisions recorded in
[ADR 0001](docs/adr/0001-foundation-toolchain.md); build commands recorded
in [README.md](README.md).

`profile-registry` implemented, verified and archived on 2026-09-16 as
`2026-09-16-profile-registry`; canonical specs promoted to
[openspec/specs/profile-registry/spec.md](openspec/specs/profile-registry/spec.md).
New in this cycle: `src/profile` (five versioned MVP descriptors with
capabilities, packages, layout, conventions, build/test commands,
deployment defaults and quality policies; `list`/`inspect`/`resolve`/`preflight`;
descriptor validation naming the missing field), Core errors
`unknown-profile`/`invalid-profile`/`incompatible-profile`/`toolchain-missing`,
CLI `forge profile list|inspect|resolve|preflight` (human/JSON, stable
`error[code]` diagnostics), and `register` gating on profile resolution
plus feature compatibility before any row mutation.

`project-import` implemented, verified and archived on 2026-09-16 as
`2026-09-16-project-import`; canonical specs promoted to
[openspec/specs/project-import/spec.md](openspec/specs/project-import/spec.md).
New in this cycle: `src/import` (read-only detection of language,
framework, package manager, database, Docker, CI, auth, features,
DriftWatch, Git remote and deployment with unknown-vs-missing evidence
and suggested profile/maturity; `ambiguous-import` on profile or
monorepo-root disagreement until `--profile` selects; minimal validated
manifest written only on `--accept` after a registry identity
pre-check, with rollback on registration failure and no legacy
doubling), Core errors `ambiguous-import`/`import-conflict`, CLI
`forge import [<path>] [--profile] [--accept] [--id]` (human/JSON,
stable `error[code]` diagnostics), and `Registry::check_identity_available`
for mutation-free collision checks.

The machine-readable line above is the single current OpenSpec pointer. It
selects the next eligible future implementation package; it does not claim
work has started.

## Next change

Implement [agent-runtime-workflows](openspec/changes/agent-runtime-workflows/proposal.md)
only when implementation is requested. Its prerequisite
(`specification-remediation`) now has implementation evidence. Then
follow the roadmap prerequisites. Later changes remain planning-only
with zero implementation tasks completed.

## Verification evidence (specification-remediation, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 199 passed, 0 failed (113 lib incl. 7 new spec
  contract tests for spec-id stability, finding-set hash,
  validate-request refusal, build-draft provenance, idempotent
  generation, list, manual/deterministic/semantic routing and
  apply_routing outcomes, 9 spec contract incl. traceable proposal
  with provenance, idempotent re-run with mtime preservation,
  empty/oversized refusal, route classification, deterministic
  apply with no files, manual apply with no AI claim, semantic apply
  producing a bounded proposal, list/inspect roundtrip, and
  doctor-after-spec regression, 4 cross-surface incl. 1 new
  upgrade × spec semantic-conflict handoff, 5 CLI contract, 8 doctor
  contract, 10 feature contract, 12 generate contract, 10 import
  contract, 9 profile contract, 7 quality_policy_contract, 12 upgrade
  contract; rust_scaffold and react-web scaffold skipped in the
  regular run; the slow `cargo build+test` evidence path is exercised
  through the existing scaffold tests that finish in ~210s when the
  host toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge spec generate` on a rust-web project writes
  `proposal.md` / `design.md` / `tasks.md` / `manifest.json` under
  `.forge/specs/<project>-<hash>/`; `forge spec list` reports the
  entry; `forge spec inspect <id>` shows the traceable provenance
  (project, profile, path, source revision, contract, generated_at,
  findings, dependencies, acceptance scenarios); a second
  `forge spec generate` with the same finding set reports
  `spec existing: ...` and writes nothing (boundary scenario);
  `forge spec generate` with no findings exits 1 with
  `error[spec-invalid]: spec invalid: spec generate requires at
  least one finding id`; `forge spec route dependency-drift` returns
  `route: deterministic` with `action: forge upgrade`; `forge spec
  route manifest-valid` returns `route: manual` with no action and
  no suggested spec; `forge spec apply driftwatch-DEPLOY-002` returns
  `route: semantic`, `status: spec-generated`, the bounded proposal
  is written, and the doctor verdict is unchanged after the spec
  operations; `forge spec apply manifest-valid` records the manual
  status with `note: manual boundary: no project changes and no AI
  fix claimed`; `forge upgrade` on a project with a drifted receipt
  still exits 1 with `error[feature-ownership-conflict]` and the
  stderr names `forge spec generate`; the cross-surface
  `upgrade_semantic_conflict_handoff_resolves_through_spec_apply`
  test confirms the receipt and manifest are preserved while the
  bounded proposal is written under `.forge/specs/`.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate specification-remediation --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/specification-remediation`).
- `git diff --check`: PASS; staged set reviewed (8 files modified,
  3 files added: `src/spec/mod.rs`, `tests/spec_contract.rs`, the
  promoted spec — 11 files; archive under
  `openspec/changes/archive/2026-09-17-specification-remediation/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real AI/agent implementation stays untested: the bounded spec is
  the handoff and the agent runtime is the next change; this cycle
  proves the spec storage, provenance, idempotency and routing
  contracts end to end, not the agent that consumes the spec. The
  spec's `tasks.md` enumerates the agent-side follow-up and the
  finding ids, dependencies and source revision are preserved for
  the agent adapter.

## Verification evidence (quality-policy-integration, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 180 passed, 0 failed (104 unit incl. 18 new policy
  contract tests for redaction shapes, JSON parsing, missing binary,
  invalid output, non-zero exit, stale observation, plus 5 new doctor
  policy integration tests for unavailable rollup, rule-id/severity
  preservation, not-applicable applicability, redaction-on-consume and
  stale-source demotion, 5 CLI contract, 3 cross-surface regression,
  8 doctor contract incl. the new `driftwatch-policy` finding, 10
  feature contract, 12 generate contract, 10 import contract, 9
  profile contract, 7 quality_policy_contract incl. missing-binary,
  parseable-report normalization with not-applicable preservation,
  non-zero exit, invalid output, credential redaction, per-project
  isolation and human-output redaction, 12 upgrade contract;
  rust_scaffold skipped in the regular run; the slow
  `cargo build+test` evidence path is exercised through the
  `rust_scaffold_builds_and_tests_with_native_toolchain` test that
  finishes in ~210s when the host toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web` then `forge doctor` with
  no `FORGE_DRIFTWATCH_BIN` reports a `driftwatch-policy` finding
  with status `unavailable` and evidence naming the missing binary
  (`driftwatch invocation failed: binary not found on PATH`) so the
  project is not labeled healthy; `FORGE_DRIFTWATCH_BIN=…fake.sh forge
  doctor` against the same project reports `driftwatch-AUTH-001`
  (warn), `driftwatch-DEPLOY-002` (fail) and `driftwatch-FLUTTER-AUTH-001`
  (applicable:false, reason preserved) plus a `driftwatch-policy`
  rollup at `fail`; the same fake script reporting a credential-laden
  payload surfaces every secret as `[REDACTED]` in both JSON and
  human output; two projects running `forge doctor` in sequence each
  reference only their own evidence strings; aging `forge.yaml` after
  a successful run flips `driftwatch-AUTH-001` from pass to warn with
  a stale observation line.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate quality-policy-integration --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/quality-policy-integration`).
- `git diff --check`: PASS; staged set reviewed (5 implementation +
  test files modified, 3 files added: `src/policy/mod.rs`,
  `tests/quality_policy_contract.rs`, the promoted spec — 8 files;
  archive under `openspec/changes/archive/2026-09-17-quality-policy-integration/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real DriftWatch adapter execution stays untested: a real
  `driftwatch` binary is not present in the local sandbox, so the
  contract is validated through `FORGE_DRIFTWATCH_BIN` fixture
  scripts that stand in for the real tool. The brief's "real
  DriftWatch integration evidence" claim is deferred until a real
  binary is available, matching the design decision that contract
  fixtures supplement but do not replace a real integration run.

## Verification evidence (extended-profile-catalog, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 148 passed, 0 failed (80 unit incl. 11 new profile
  support_status/react-web/planned tests after dropping the stale
  `react-web` unknown-profile probe, 5 CLI contract, 3 cross-surface
  regression, 8 doctor contract incl. 1 new planned-profile
  doctor finding, 10 feature contract, 12 generate contract incl.
  react-web render + planned-profile generation refusal, 9 profile
  contract incl. 3 new react-web and planned-profile contract
  cases, 10 import contract, 11 upgrade contract; rust_scaffold
  skipped in the regular run; the slow `cargo build+test` evidence
  path is exercised through the `rust_scaffold_builds_and_tests_
  with_native_toolchain` test that finishes in ~218s when the host
  toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge profile list` shows all six supported ids
  (aspnet-web, flutter-app, nextjs-web, python-service, react-web,
  rust-web) at 0.1.0 and omits the planned candidates; `forge
  profile inspect react-web` (human + JSON) reports
  `support_status: supported` with `adapter-react`, toolchain
  `npm@20`, build `npm run build`, test `npm test`, and the
  client-only capability set; `forge profile inspect flutter-client`
  reports `support_status: planned` with a description that names
  the backend boundary; `forge profile resolve react-web --feature
  i18n` resolves `react-web@0.1.0 via adapter-react`; `forge
  profile resolve react-web --feature postgres` exits 1 with
  `error[incompatible-profile]` and the backend hint; `forge
  profile resolve aspnet-saas` exits 1 with
  `error[unsupported-profile]`; `forge new --profile react-web`
  creates a registered project with the expected 10 files;
  `forge new --profile rust-cli` exits 1 with
  `error[unsupported-profile]` and writes nothing.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate extended-profile-catalog --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/extended-profile-catalog`).
- `git diff --check`: PASS; staged set reviewed (8 implementation +
  test files, 5 archive files, 1 promoted spec — 14 files; 1033
  insertions, 36 deletions).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Per-service native react-web validation was not exercised in this
  cycle (no node toolchain integration in the local sandbox);
  rendering alone is verified and the
  `react_web_scaffold_builds_and_tests_with_native_toolchain` test
  remains available as the `npm run build` / `npm test` evidence
  path when the host toolchain is on PATH. Planned candidates stay
  discoverable through `inspect_profile` and `planned_profiles` but
  refuse generation before any file change; their promotion to
  `Supported` remains future work, not an implementation claim here.

## Verification evidence (project-upgrade-orchestration, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 138 passed, 0 failed (71 unit incl. 11 new upgrade
  plan/apply/fleet tests after the drifted-receipt test fix, 5 CLI
  contract, 3 cross-surface regression incl. 1 new
  upgrade × feature-lifecycle interaction, 8 doctor contract,
  10 feature contract, 12 generate contract, 10 import contract,
  7 profile contract, 12 new upgrade contract incl. dry-run plan with
  old/new versions/assets/recovery, apply advancing versions and
  changing files, semantic-conflict handoff on drifted receipt with
  preserved files, already-satisfied no-op, unknown feature failure
  before edits, missing requested feature install, postgres schema
  irreversible with declared strategy, fleet completion with
  per-project journals, fleet isolation of a blocked project without
  wholesale success, fleet retry skipping satisfied projects and
  re-planning on changed preconditions, fleet dry-run skipping
  writes).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id up-app` registers
  L1; `forge upgrade --feature auth` installs auth and writes receipt
  + manifest; `forge upgrade --all` reports `0 success, 0 failure,
  0 blocked, 1 skipped` for the now-satisfied project; `forge upgrade
  --all --dry-run` reports `plan only` without changes; `forge upgrade
  --feature nosuch` exits 1 with `error[unknown-feature]`;
  `forge doctor` still reports the same PASS manifest/profile/
  features/drift/build/deployment after fleet upgrades; postgres
  --dry-run after aging `postgres: 0.0.9` shows `upgrade postgres:
  0.0.9 -> 0.1.0 [package+configuration+codemod+schema]` with the
  irreversible `manifest-repin+manual-schema-review` recovery note.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate project-upgrade-orchestration --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive, includes the promoted
  `spec/project-upgrade-orchestration`).
- `git diff --check`: PASS; staged set reviewed (4 files modified,
  2 files added: implementation, tests, archive and promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Per-service native generation validation stays out of scope here;
  upgrade only repins manifest versions, receipts and runs declared
  policy validators; DriftWatch execution evidence stays deferred to
  v0.3 (`quality-policy-integration` and later).

## Verification evidence (feature-lifecycle, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 115 passed, 0 failed (61 unit incl. 12 new feature
  catalog/resolver/add/remove/upgrade/ownership/section-preservation
  tests, 5 CLI contract, 2 cross-surface regression, 8 doctor contract,
  10 new feature contract incl. catalog discovery, dep-ordered plans,
  conflict/missing/unsupported failures before edits, add-then-upgrade
  agreement, reverse-dep and ownership blocks with preservation,
  exact-reinstall no-op, closure selection in `new`, repeatability,
  12 generate contract, 10 import contract, 7 profile contract).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --feature admin` records
  auth+admin 0.1.0; `feature resolve rust-web --feature billing` plans
  auth,billing in order; `feature add billing` updates manifest+receipt+
  registry; `feature remove auth` exits 1 with
  `error[incompatible-feature]` naming admin,billing dependents and
  preserving files; `forge doctor` still PASSes manifest/profile/
  features-compatible on the feature-modified project.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate feature-lifecycle --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Package installation stays per-service native resolution (scaffolds are
  dependency-free by design); declared policy validators are reported
  per plan while DriftWatch execution evidence stays deferred to v0.3
  (`quality-policy-integration` and later).

## Verification evidence (doctor-maturity-assessment, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 93 passed, 0 failed (49 unit incl. 9 new doctor
  inventory/maturity/stale/unavailable tests, 5 CLI contract, 2
  cross-surface regression, 8 new doctor contract incl. stable
  finding IDs with evidence/remediation classes, unavailable-inspector
  reporting, repeatability without file changes, L2 missing-control
  reporting, L4 recovery denial, L0 nonapplicability, stale-observation
  reporting and generated-project registry freshness, 12 generate
  contract, 10 import contract, 7 profile contract).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web` then `forge doctor`
  reports PASS manifest/profile/features/drift/build/deployment with
  `[UNAVAILABLE] repository` outside a git repo and verdict `not
  healthy`; `--target L2` JSON reports unmet
  L2-auth/admin/ci/driftwatch (deployment met via Dockerfile) and
  L1-structure met after the forge.yaml-presence fix.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate doctor-maturity-assessment --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (10 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Doctor is local inspection in v0.1; DriftWatch execution evidence stays
  deferred to v0.3 (`quality-policy-integration` and later).

## Verification evidence (deterministic-project-generation, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 76 passed, 0 failed (40 unit incl. 11 new generate
  normalization/rendering/failure/cancel/collision/preflight tests, 5 CLI
  contract, 2 cross-surface regression, 10 import contract, 7 profile
  contract, 12 new generate contract incl. explicit-vs-interactive
  equivalence, nonempty/cancel/unknown/incompatible/id-collision failures,
  missing-toolchain unverified reporting, native rust/node/dotnet/flutter
  builds and python compile check).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new` for all five MVP profiles renders, registers
  and lists with L1 maturity; repeat `new` on a nonempty destination exits
  1 with `error[generation-conflict]`; piped interactive answers create the
  same request as flags; `--verify-native` on rust-web reports native
  `cargo build` + `cargo test` success; empty-PATH `--verify-native`
  exits 1 with `error[toolchain-missing]` containing "not tested".
- Native evidence (Forge unavailable): generated rust `cargo build` +
  `cargo test` PASS; `npm run build` + `npm test` PASS; `dotnet build`
  PASS (0 warnings, 0 errors); `flutter test` PASS (1 test); python
  `compileall` PASS while `pytest`/`build` modules are absent, so only
  rendering (not a pytest run) is claimed for python-service.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate deterministic-project-generation --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Generated scaffolds are dependency-free by design (offline-portable);
  per-service framework packages remain per-service resolution, not part
  of the scaffold claim.

## Verification evidence (project-import, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 53 passed, 0 failed (29 unit incl. 8 new import
  detection/adoption tests, 5 CLI contract, 2 cross-surface regression,
  7 profile contract, 10 new import contract incl. read-only proposal,
  ambiguity-before-writes, missing-remote inspection, accept-writes-only-
  manifest, id-collision/unwritable/legacy failures, repeatability,
  unknown-profile refusal, incompatible-manifest gating).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge import` on a Rust fixture reports Language rust,
  Framework axum, suggested rust-web/L1 with high confidence;
  `--accept` writes only `forge.yaml` and `inspect` returns profile
  rust-web maturity L1; mixed rust+flutter exits 1 with
  `error[ambiguous-import]` in human and JSON.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate project-import --strict --no-interactive`: valid
  pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Detection fixtures are synthetic; no new native-toolchain profile
  support is advertised. Native generation validation stays deferred to
  `deterministic-project-generation`.

## Verification evidence (profile-registry, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 35 passed, 0 failed (21 unit incl. 8 new profile
  descriptor/resolver/preflight tests, 5 CLI contract, 2 cross-surface
  regression, 7 new profile contract incl. list/inspect/resolve/preflight,
  flutter+postgres rejection, register gating and repeatability).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge profile list` shows all five MVP IDs at 0.1.0;
  `profile inspect rust-web` JSON carries adapter/build/test metadata;
  `profile resolve flutter-app --feature postgres` exits 1 with
  `error[incompatible-profile]` suggesting a backend boundary; empty-PATH
  `profile preflight rust-web` exits 1 with `error[toolchain-missing]`
  without claiming the profile was tested.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate profile-registry --strict --no-interactive`: valid
  pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- Committed as `9a3b263`; no push performed.
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Profile metadata alone does not establish working templates; native
  toolchain generation validation is deferred to
  `deterministic-project-generation`.

## Prior verification evidence (core-manifest-registry)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 20 passed, 0 failed (13 unit, 5 CLI contract, 2
  cross-surface regression incl. register→restart→inspect roundtrip,
  id/path collisions, unavailable/unknown reporting, dual-manifest and
  unsupported-schema failures leaving files unchanged, stale-pending
  journal reconciliation).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge --version`/`list`/`register`/`inspect` roundtrip
  against `tests/fixtures/valid-full`; unknown id exits 1 with
  `error[unknown-project]`; unknown subcommand exits 2 without creating a
  registry; empty registry lists an empty collection.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 24 passed, 0 failed
  (post-archive); `openspec validate core-manifest-registry --strict`:
  valid pre-archive.
- `git diff --check`: PASS; staged set reviewed (23 files, implementation
  + tests + fixtures + ADR + README + archive + promoted specs only).
- Committed as `15c225f`; no push performed.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## Implementation cycle

1. Run `node scripts/check-openspec-change-names.mjs` before selection; failure blocks status/instructions and implementation.
2. Run `openspec list`, reconcile roadmap dependencies, and update the single pointer before work.
3. Run `openspec status --change extended-profile-catalog` and `openspec instructions apply --change extended-profile-catalog`; read all selected artifacts and applicable local rules.
4. Follow BFS analysis, structural pass, DFS requirement implementation, then BFS regression/completeness. Check tasks only against evidence.
5. Run the actual local build/test/integration commands and applicable Gate before archive; record exact failures and next actions. Gate FAIL or unresolved REVIEW_REQUIRED blocks completion when a Gate is configured.
6. Run the name checker and `openspec validate --all --strict --no-interactive`; review diffs and original impact surfaces.
7. Archive verified work without `--skip-specs`, inspect promoted canonical specs, and commit only related implementation/tests/archive/specs.
8. Advance `current_spec` to the next active eligible change, or remove the line when no active changes remain; update this evidence, commit HANDOFF separately and stop without push.

Planning-only documentation does not implement, archive or commit the queued changes. Future blockers must identify the exact failed command and next action; they must not be recorded as completion.
