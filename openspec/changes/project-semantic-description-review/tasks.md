# Tasks: project-semantic-description-review

Status: planning-only package. Every task is unchecked by design.

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Read `src/spec/mod.rs`, `src/agent/`, `src/analytics/` and `src/portfolio/`; record the bounded-proposal routing, provider boundary and why semantic state is kept local rather than reusing the interest or portfolio stores.
- [ ] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, `src/semantic/*`, CLI `forge describe|classify …`, provider reached through the existing agent adapter, no new dependency.
- [ ] 1.3 Map each requirement and scenario to its proposal kind, state transition, caller, failure case and named test.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Implement the `forge-semantic-proposal/0.1.0` proposal shape, closed kind set and state machine.
- [ ] 2.2 Implement suggest/approve/reject/supersede operations with explicit operator authority and revision binding.
- [ ] 2.3 Implement conflict retention and unavailable-provider reporting without inventing a suggestion.
- [ ] 2.4 Route unresolved/conflicting interpretations to bounded OpenSpec or manual review.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Prove suggest is read-only and nothing is auto-applied to a project or provider.
- [ ] 3.2 Exercise conflicting evidence, stale revision, unavailable provider, malformed suggestion and credential-scrub boundaries.
- [ ] 3.3 Prove approved proposals hand off to the remediation and adapter packages without this package writing files or remote state.

## 4. Verification

- [ ] 4.1 Run `cargo fmt/build/clippy/test`, the two new suites, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output.
- [ ] 4.2 Record that no model/provider was contacted and no project identity was auto-published; provider evidence stays external.
