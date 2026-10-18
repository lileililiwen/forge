# Tasks: CI/CD system model documentation

## 1. BFS — Baseline and impact coverage

- [x] Map generated project hooks/CI, Forge's own CI, release checks, delivery operations, evidence files, journals, API/UI readers, and external owners to current sources.
- [x] Reconcile every finding against the relevant canonical specs and latest HANDOFF evidence; label inferred or externally unverified claims.
- [x] Review document navigation, diagrams, and traceability before drafting the final guide.

## 2. DFS — Requirement-by-requirement implementation

- [x] Write the current state model, data dictionary, command flows, evidence boundaries, and ranked gaps in `docs/ci-cd-system-model.md`.
- [x] Link the guide from README, correct the stale Gate API description, and keep proposed follow-up work separate from implemented behavior.

## 3. BFS — Cross-surface regression and completeness

- [x] Recheck Gate, release, delivery, generated template, Forge CI, API/workbench, registry and roadmap references for drift or contradictory descriptions.
- [x] Verify diagram/prose consistency and all internal links; ensure no sibling, provider, branch-protection, publication, or deployment state is claimed without evidence.

## 4. Verification

- [x] Run name preflight, strict OpenSpec validation, Markdown link/reference checks, and `git diff --check`.
- [x] Rehearse with `forge gate --dry-run`, run full local Forge Gate with a bounded timeout, and record the actual result in HANDOFF. The 2026-10-18 run was blocked: two project-runtime build checks timed out; `tests` failed at `governance_cross_surface::broken_preset_adapter_never_renders_healthy_in_checker_or_portal` (`unknown` observed, `unavailable` expected). Direct `scripts/check-tests.sh` reproduction confirmed the same failure; this documentation-only change does not touch the failing code/test.
- [x] Archive the completed change without `--skip-specs` and promote its canonical spec. Evidence: archived at `2026-10-18-ci-cd-data-command-flow`; `ci-cd-system-model-documentation` has two added requirements. Related implementation and OpenSpec artifacts are committed separately from the HANDOFF-only closeout as required by repository workflow.
