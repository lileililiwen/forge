# Tasks: fleet-liveness-status

## 1. BFS — Baseline and impact coverage
- [ ] 1.1 Map requirements to layers/callers: `src/main.rs` fleet CLI wiring, `src/fleet/` module home, `src/publish/` transport + redaction reuse, `forge-project-inventory/0.1.0` roster input, `forge-publish-queue-status` journal join (read-only), served `platform/Caddyfile` + target `docker ps` + HTTPS probes; record language (Rust, same crate), test layers (unit, CLI contract, live Mac), and the implementation-handoff gate.
- [ ] 1.2 Confirm proposal/design/spec agreement on verdicts, read-only guarantee, contract shape, and non-goals (no remediation, no history, no MCP/API).

## 2. DFS — Requirement-by-requirement implementation
- [ ] 2.1 Add `src/fleet/online.rs` verdict classifier + served-Caddyfile host parser with unit tests (full verdict matrix, nav/fallback exclusion, redaction).
- [ ] 2.2 Add the bounded probe runners (target `docker ps` / `ssh cat` via the shared transport vocabulary, HTTPS via bounded `curl`) with timeout shaping and unavailable-on-missing-binary behavior.
- [ ] 2.3 Wire `forge fleet online` (roster flags mirroring `publish fleet`, `--timeout-secs`, `--format`) emitting `forge-fleet-liveness/0.1.0` JSON + human table with the specified exit contract.
- [ ] 2.4 Add `tests/fleet_online_contract.rs` CLI contract tests (verdicts, skipped-never-probed, exit codes, empty-roster refusal).

## 3. BFS — Cross-surface regression and completeness
- [ ] 3.1 Prove read-only: journal/registry/target bytes identical before and after; no secret values in any output.
- [ ] 3.2 Prove no existing surface changed: `fleet list|status|inspect`, `publish fleet`, `deploy status`, MCP `tools/list`, portal views byte-identical.

## 4. Verification
- [ ] 4.1 Run `cargo fmt --check` (touched files), `cargo build`, `cargo clippy --all-targets -- -D warnings` (touched files), `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain` (full suite green + new suites); `git diff --check` pass.
- [ ] 4.2 Run `node scripts/check-openspec-change-names.mjs` and `openspec validate --all --strict --no-interactive` (0 failed); re-render ensures proposal/design/specs/tasks still satisfy workflow.md BFS→DFS→BFS, capability, and non-goals sections.
- [ ] 4.3 Live Mac evidence: real `forge fleet online` post-fleet with per-host verdicts recorded.
