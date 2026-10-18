# Tasks: standalone-project-boundary

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Map the existing project schema, vocabulary loader,
      workspace_check validator pattern, adapter observation
      surface, and test fixtures for capability and
      release-evidence blocks.
- [ ] 1.2 Inventory the six canonical statuses
      (`standalone`, `standalone-with-optional-integrations`,
      `library-dependent`, `workspace-dependent`,
      `service-dependent`, `blocked`), the integration required
      states, the bounded static scan targets, and the secret
      redaction hook.
- [ ] 1.3 Confirm the change is additive: no existing declaration
      becomes invalid, no existing finding code changes, no
      adapter exit code changes, and no sibling repository
      needs to be touched.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Add `standalone_statuses` and
      `integration_required_states` to `vocabulary.json`; extend
      `scripts/vocabulary.py` to expose them and fail closed on
      missing, empty, non-array, or duplicate values.
- [ ] 2.2 Add optional `standalone` and `integrations` blocks to
      `schemas/project.schema.json` (new `$defs.standalone` and
      `$defs.integration`; no existing field changes; existing
      declarations remain valid).
- [ ] 2.3 Implement `validate_standalone(...)` in
      `scripts/workspace_check.py` covering STB-001 to STB-007:
      the 6 statuses, required/optional integrations, absolute
      path rejection, `..` escape rejection, secret redaction,
      bounded static scan, evidence_ref resolution, and the
      canonical-status check.
- [ ] 2.4 Wire `validate_standalone(...)` into
      `validate_project(...)` after the release-evidence and
      quality checks so the new validator only fires when the
      block is present and the existing failure mode is
      unchanged for absent blocks.
- [ ] 2.5 Extend `scripts/forge_governance_adapter.py` to import
      `classify_standalone(...)` from `workspace_check`, populate
      `standalone_status`, `standalone_observed_revision`,
      `standalone_integrations`, and `standalone_findings` on
      the observation document, and apply the existing
      `scrub_line` redaction to `standalone_findings`.
- [ ] 2.6 Add `scripts/workspace-check`-compatible CLI surface
      so a `python3 scripts/workspace_check.py --standalone
      <project-id>` (or equivalent) returns the per-project
      classification as JSON; keep the existing CLI flags
      backward compatible.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Add `tests/test_standalone_project_boundary.py`
      covering every status, every failure path, every bounded
      static scan target, the adapter's new fields, the
      read-only guarantee, and the vocabulary loader.
- [ ] 3.2 Re-run the full unittest suite and the existing
      self-check, auto-gate, Jenkins manifest, change-name
      validator, and OpenSpec validator; confirm 0 new
      portfolio-level findings and the existing 217 baseline
      tests still pass.
- [ ] 3.3 Confirm the governance repository's own
      `.project.json` does not carry a `standalone` block
      (it is the orchestrator, not a sibling candidate), so
      self-check remains 0 errors / 0 warnings.

## 4. Verification

- [ ] 4.1 Run `python3 -m unittest discover -s tests -v`,
      `python3 scripts/workspace_check.py --self-check`,
      `python3 scripts/auto_gate.py --self-check
      --no-deployment`, `python3 scripts/jenkins_manifest.py
      --check`, `node scripts/check-openspec-change-names.mjs`,
      `openspec validate --changes --strict --no-interactive`,
      and `git diff --check`.
- [ ] 4.2 Render three fixture projects (a `standalone`
      product, a `standalone-with-optional-integrations`
      product, a `workspace-dependent` candidate) through both
      the checker and the adapter; assert the four new fields
      are present and the existing observation keys are
      unchanged.
- [ ] 4.3 Confirm the secret-shaped value in
      `integrations[0].evidence_ref` is replaced by
      `[redacted]` in the evidence line; confirm a
      `STANDALONE_SECRET_VALUE` error is still emitted.
- [ ] 4.4 Archive the change with
      `openspec archive --change standalone-project-boundary
      --skip-specs` (or equivalent) so the canonical spec
      moves to `openspec/specs/standalone-project-boundary/`.
- [ ] 4.5 Update `HANDOFF.md` with the archived change pointer
      and the verification evidence (separate commit per the
      workflow rules).
