# Tasks: project metadata bundle

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map each requirement/scenario to the Core builder, ownership receipt, registry reader, CLI/MCP call path, failure boundary and exact regression test.
- [x] 1.2 Confirm current `ProjectCommands`, local `Registry` read-only APIs, atomic file-write conventions, redaction helpers, and MCP tool naming/dispatch; record the exact modules and no-sibling behavior in review evidence.
- [x] 1.3 Add behavioral tests for deterministic metadata projections, receipt state transitions, read-only search, matching/ranking, and per-project failure reporting before implementation.
- [x] 1.4 Confirm no change to `CatalogRecord`'s 15-key contract and no CodeGraph process/configuration or sibling write.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement the versioned bundle and receipt models; parse bounded `.project.json`, optional `forge.yaml`, README and canonical spec labels beneath the explicit project root; normalize/redact and produce deterministic bytes.
- [x] 2.2 Implement safe `build`/refresh ownership transitions with path confinement, exact digest checks, staging and refusal without partial writes.
- [x] 2.3 Implement local-registry read-only metadata search, token validation, deterministic ranking, source revision checks and explicit per-project issues.
- [x] 2.4 Add `forge project metadata build` and `forge project metadata search` handlers that delegate to Core and support human/JSON output.
- [x] 2.5 Add mature MCP `search_project_metadata` descriptor and read-only dispatch using the same Core search result; document its argument and result contracts.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify generation/search behavior for absent, malformed, foreign, edited, stale and valid artifacts; exact bytes remain unchanged on every refusal and every search.
- [x] 3.2 Verify CLI/MCP parity, MCP read-only classification and one added tool only; the existing catalog and all existing tool behavior remain unchanged.
- [x] 3.3 Review path bounds, symlink escape and replacement races, redaction, query limits, bounded results/issues, deterministic tie ordering, and confirm no sibling or CodeGraph dependency.
- [x] 3.4 Update README, architecture and usage docs; reconcile the proposal, design, capability spec and code paths.

## 4. Verification

- [x] 4.1 Run focused Core, CLI, and MCP behavioral suites; run a temporary project smoke flow that builds metadata, searches it through CLI and MCP, edits a receipt-owned file, and observes refusal with unchanged bytes.
- [x] 4.2 Run `cargo fmt --check`, `cargo build`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets`; record actual counts and known skips. — Format/build/clippy passed; test suite: 2,762 passed, 0 failed, 3 ignored (153 suites).
- [x] 4.3 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, `git diff --check`, `forge gate --dry-run`, and full `forge gate --timeout-secs 1800`; record each verdict and pre-existing failures in HANDOFF. — Names PASS; strict validation 105/0; diff clean; dry-run rendered 9 required checks. Full Gate: 8 checks pass, `source-file-size` fails only on the pre-existing, unchanged `src/api/admin/assurance.rs` (1,003 > 1,000 lines); no changed file exceeds the cap.
- [x] 4.4 Archive without `--skip-specs` and verify canonical metadata spec promotion with no stale Purpose marker. — `openspec archive project-metadata-bundle -y` promoted three requirements; the archiver's generated Purpose placeholder was replaced, and post-archive strict validation passed 105/0.
- [x] 4.5 Commit implementation/specs/tests; do not push. — `3b59ea4` contains the implementation, updated callers/tests/docs, canonical spec and archived package.
- [x] 4.6 Update HANDOFF/remove `current_spec`, then commit HANDOFF separately; do not push. — `HANDOFF.md` records the archived change and verification; the active pointer is removed, and this update is committed separately.
