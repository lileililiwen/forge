## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map delivery CLI, project API, admin API, workbench, registry journal and Gate evidence paths to success/refusal scenarios.
- [x] 1.2 Add test fixtures for current-revision passing Gate evidence and absent/stale/non-passing evidence; verify package design matches the shared freshness classifier and user error surface.
- [x] 1.3 Confirm implementation handoff: Rust Core boundary, no sibling dependency, no new persistence, existing Delivery workbench journey and exact verification commands.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Enforce valid Git HEAD equality with the registered revision before provider/config/journal operations; test mismatch and unavailable Git.
- [x] 2.2 Require fresh `passed` Gate evidence before provider configuration resolution and operation reservation; test absent, stale, blocked, failed and unknown evidence.
- [x] 2.3 Preserve actionable typed refusal across CLI/API/workbench and preserve successful provider invocation/idempotency behavior.
- [x] 2.4 Add/adjust browser delivery coverage for the announced refusal/recovery message and successful preflight journey.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run every successful delivery preflight path with exact Gate evidence and confirm provider call count, journal shape, stage and promotion compatibility.
- [x] 3.2 Verify all refusal paths leave provider and journal untouched and errors retain safe typed details.
- [x] 3.3 Update the canonical delivery requirement and CI/CD model gap ordering; review no unrelated lifecycle or provider contract changed.

## 4. Verification

- [x] 4.1 Run changed-file Rust formatting checks, Node syntax check, and focused delivery/API/browser contract tests.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, and `git diff --check`.
- [x] 4.3 Run `forge gate --dry-run` then bounded full `forge gate`; record the blocked verdict and confirmed unrelated failure/remediation in HANDOFF.
- [x] 4.4 Archive with canonical spec promotion; reviewed and staged only related paths, create the implementation/archive commit, update HANDOFF, create the HANDOFF-only commit, then stop.
