current_spec: deterministic-project-generation

# Forge handoff

## Current state

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

Implement [deterministic-project-generation](openspec/changes/deterministic-project-generation/proposal.md)
only when implementation is requested. Its prerequisites
(`profile-registry`, `project-import`) now have implementation
evidence, not merely proposals. Then follow the roadmap prerequisites.
Later changes remain planning-only with zero implementation tasks completed.

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
3. Run `openspec status --change deterministic-project-generation` and `openspec instructions apply --change deterministic-project-generation`; read all selected artifacts and applicable local rules.
4. Follow BFS analysis, structural pass, DFS requirement implementation, then BFS regression/completeness. Check tasks only against evidence.
5. Run the actual local build/test/integration commands and applicable Gate before archive; record exact failures and next actions. Gate FAIL or unresolved REVIEW_REQUIRED blocks completion when a Gate is configured.
6. Run the name checker and `openspec validate --all --strict --no-interactive`; review diffs and original impact surfaces.
7. Archive verified work without `--skip-specs`, inspect promoted canonical specs, and commit only related implementation/tests/archive/specs.
8. Advance `current_spec` to the next active eligible change, or remove the line when no active changes remain; update this evidence, commit HANDOFF separately and stop without push.

Planning-only documentation does not implement, archive or commit the queued changes. Future blockers must identify the exact failed command and next action; they must not be recorded as completion.
