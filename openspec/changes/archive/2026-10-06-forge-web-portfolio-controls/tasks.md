# Tasks: forge-web-portfolio-controls

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map catalog IDs to portfolio/catalog/fleet/inventory/governance/analytics/provider/readiness Core functions and ownership.
- [x] 1.2 Add fixtures for stale, malformed, unavailable, thresholded, disabled and not-run states.
- [x] 1.3 Verify all mutations have actor/audit coverage and exact owned fields.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add authenticated portfolio metadata and review/goal/relationship routes with accessible forms.
- [x] 2.2 Add catalog, gap, fleet and portable inventory evidence views with filters and freshness.
- [x] 2.3 Add governance selection/status and analytics/readiness/provider views; live probes remain explicitly gated.
- [ ] 2.4 Add share allowlist, preview and digest-bound approval pages; do not publish from this package.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify read models match CLI/Core, including source, timestamps, malformed and partial outcomes.
- [x] 3.2 Verify imported/source-owned data cannot be edited and all Forge-owned writes are auditable.
- [x] 3.3 Verify no provider execution on page load and privacy thresholds remain enforced.

## 4. Verification

- [x] 4.1 Run formatting, all-target checks, focused portfolio/provider/API tests and strict OpenSpec validation.
- [ ] 4.2 Run browser coverage for metadata changes, source failure, disabled providers, threshold withholding and live opt-in.
