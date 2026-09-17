current_spec: extended-profile-catalog

# Forge handoff

## Current state

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

Implement [extended-profile-catalog](openspec/changes/extended-profile-catalog/proposal.md)
only when implementation is requested. Its prerequisite
(`feature-lifecycle`) now has implementation evidence, and
`project-upgrade-orchestration` is also archived. Then follow the
roadmap prerequisites. Later changes remain planning-only with zero
implementation tasks completed.

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
