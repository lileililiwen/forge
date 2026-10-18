# Tasks: Platform capability and release governance

## 1. BFS — Baseline and impact coverage

- [ ] Inventory `.project.json`, `.ai-gate`, registry, Forge metadata, Gate
  evidence, and Jenkins deployment fields.
- [ ] Map capability declarations to platform ownership and required evidence.
- [ ] Add valid, invalid, missing, stale, and unverified evidence fixtures.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Add schema-compatible capability declarations and validation.
- [ ] Add release evidence vocabulary and status mapping.
- [ ] Add audit output for declared/configured/verified/unverified states.
- [ ] Add documentation and generated-profile guidance.

## 3. BFS — Cross-surface regression and completeness

- [ ] Re-run workspace discovery, adoption, auto-gate, registry, and Jenkins
  manifest checks.
- [ ] Verify metadata cannot promote a project to deployable without native
  build, runtime, release, and deployment evidence.
- [ ] Verify existing project declarations remain backward compatible.

## 4. Verification

- [ ] `python3 -m unittest discover -s tests -v`.
- [ ] `python3 scripts/workspace_check.py --self-check`.
- [ ] `python3 scripts/auto_gate.py --self-check --no-deployment`.
- [ ] `openspec validate --changes --strict --no-interactive`.
