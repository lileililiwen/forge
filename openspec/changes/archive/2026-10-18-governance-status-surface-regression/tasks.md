# Tasks: Governance and Gate status projection separation

## 1. BFS — Baseline and impact coverage

- [x] Map the provider fixture lifecycle, governance status CLI, checker alert, and Gate-only portal settings row to their current source contracts. Evidence: source inspection of `src/governance/engine.rs`, `src/checker/mod.rs`, and `src/portal/sections.rs`, plus the focused failure reproduction.
- [x] Confirm the test covers missing adapter, provider-reported failure, and absent Gate evidence without mutating shared state. Evidence: `cargo test --test governance_cross_surface -- --test-threads=1` passes all six tests.
- [x] Confirm proposal, design, scenarios, task mapping, and unchanged UI scope agree.

## 2. DFS — Requirement-by-requirement implementation

- [x] GSR-001: update the cross-surface integration test to assert unavailable/fail through governance status and checker, while independently asserting absent Gate evidence stays unknown in portal settings. Evidence: `cargo test --test governance_cross_surface -- --test-threads=1` passes all six tests.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-run the complete cross-surface file and provider preset contract; confirm healthy baseline behavior and local default remain covered.
- [x] Review the status outputs for provider attribution, Gate freshness, no fabricated governance attribute, and no healthy status masking. Evidence: missing adapter maps to CLI unavailable/checker warning; provider fail maps to CLI fail/checker error; portal remains Gate unknown/absent in both cases.

## 4. Verification

- [x] Run the full workspace test binding, relevant formatting, name preflight, strict OpenSpec validation, and `git diff --check`. Evidence: both Gate workspace test bindings passed; `rustfmt --edition 2021 --check tests/governance_cross_surface.rs`, name preflight, strict OpenSpec validation (116/116 before archive), and `git diff --check` passed.
- [x] Run `forge gate --dry-run` and full Forge Gate with a bounded timeout; record any unrelated failure and its existing remediation path truthfully. Evidence: dry-run resolved 8/8 checks; fresh full run `timeout 1260 ./target/debug/forge gate --timeout-secs 1200` passed all 8 checks at revision `a8a63595ef96`, observed `2026-10-18T13:53:16Z`.
- [x] Archive without `--skip-specs`, promote the canonical requirement, commit implementation/archive, then commit HANDOFF only. Evidence: archived as `2026-10-18-governance-status-surface-regression`; canonical requirement promoted to `openspec/specs/governance-status-surface-regression/spec.md`; implementation/archive and HANDOFF commits recorded below.
