current_spec: profile-registry

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

The machine-readable line above is the single current OpenSpec pointer. It
selects the next eligible future implementation package; it does not claim
work has started.

## Next change

Implement [profile-registry](openspec/changes/profile-registry/proposal.md)
only when implementation is requested. Its prerequisite
(`core-manifest-registry`) now has implementation evidence, not merely a
proposal. Then follow the roadmap prerequisites. Later changes remain
planning-only with zero implementation tasks completed.

## Verification evidence

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
3. Run `openspec status --change profile-registry` and `openspec instructions apply --change profile-registry`; read all selected artifacts and applicable local rules.
4. Follow BFS analysis, structural pass, DFS requirement implementation, then BFS regression/completeness. Check tasks only against evidence.
5. Run the actual local build/test/integration commands and applicable Gate before archive; record exact failures and next actions. Gate FAIL or unresolved REVIEW_REQUIRED blocks completion when a Gate is configured.
6. Run the name checker and `openspec validate --all --strict --no-interactive`; review diffs and original impact surfaces.
7. Archive verified work without `--skip-specs`, inspect promoted canonical specs, and commit only related implementation/tests/archive/specs.
8. Advance `current_spec` to the next active eligible change, or remove the line when no active changes remain; update this evidence, commit HANDOFF separately and stop without push.

Planning-only documentation does not implement, archive or commit the queued changes. Future blockers must identify the exact failed command and next action; they must not be recorded as completion.
