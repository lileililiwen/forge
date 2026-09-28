# Tasks: portfolio-activation-readiness

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Map the interest read model, freshness rule, privacy-mode and coverage vocabularies, threshold handling, CLI and API dispatch, and the admin authorization boundary.
- [ ] 1.2 Confirm the package adds no stored state, no migration and no billing surface; document the Forge/product ownership split and the `paid_interest_events` boundary.
- [ ] 1.3 Record that the readiness verdict reads only what an existing provider already imported, and that no provider is contacted.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Add the closed readiness vocabulary (`ready|not-ready`) and the closed reason vocabulary, with a reason per withheld verdict rather than a score.
- [ ] 2.2 Add the read-only readiness projection over current interest snapshots, carrying window, source, source revision, privacy mode, coverage and freshness into every verdict.
- [ ] 2.3 Add the operator-declared threshold: no default, bounded, refused out of range rather than clamped, and `threshold-not-declared` when absent.
- [ ] 2.4 Add `forge portfolio activation readiness [PROJECT]` as a read-only CLI command, admin-gated, and refuse an unknown project or unlisted metric with the existing typed errors.
- [ ] 2.5 Add unit and CLI contract tests for `ready`, every `not-ready` reason, the threshold refusal, the unknown project and metric, and the read-only guarantee.
- [ ] 2.6 Add cross-surface tests proving no readiness or billing field reaches the share manifest, the portfolio projection or the fleet list.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Prove a registry written by `portfolio-interest-snapshots` is read as-is with no schema or migration change.
- [ ] 3.2 Exercise the boundary cases: superseded-only evidence, all-stale windows, `lower-bound` and `undeclared` privacy modes, `partial` coverage, no evidence, and a project exactly at the threshold.
- [ ] 3.3 Verify existing interest, share, portfolio, generation and publish behaviour remains compatible, and that absence of evidence never reads as readiness.

## 4. Verification

- [ ] 4.1 Run formatting, build, clippy, focused and full tests, strict OpenSpec validation, the name preflight and `git diff --check`.
- [ ] 4.2 Record that no analytics provider was contacted and no product was activated; product conversion evidence stays external and this package claims no revenue outcome.
- [ ] 4.3 Do not begin product billing, entitlement or checkout work in Forge in this package or as a follow-up to it without a separate product-owned proposal.
