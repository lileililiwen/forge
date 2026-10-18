# Gates, evidence, and release readiness

## Read first

- `docs/capabilities.md` — the capability declaration vocabulary.
- `docs/product-code-quality.md` — the product-code quality contract.
- `docs/deployment-promotion.md` — the promotion transaction and ledger.
- `openspec/specs/product-code-quality-boundary/spec.md` — the quality-gate
  contract.
- `openspec/specs/platform-capability-manifest/spec.md` — the capability
  manifest contract.
- `openspec/specs/deployable-batch-onboarding/spec.md` — the deployable
  onboarding contract.

## Run

- `python3 scripts/auto_gate.py --root .. --report auto-gate-report.json` —
  the unattended entry point. Jenkins runs the same command before project
  verification or deployment.
- `python3 scripts/product_code_quality.py <project_dir> --json` — test code
  in product scope and placeholder debt. Fail-closed: gaps are
  `REVIEW_REQUIRED`, never a guessed pass.
- `python3 scripts/maturity_audit.py --root .. --json` — maturity indicators.
  It reports the authoritative `workspace_check` counts; it never computes a
  second result.
- `python3 scripts/jenkins_manifest.py --check` — the safe handoff to
  jenkins-local. Emits only `deployable: true` projects backed by a
  promotion ledger entry.

## Shared-state actions need approval

Promotion, deployment, and gate execution live behind the existing
explicit-approval boundary:

- `scripts/promote_project.py` records a promotion transaction; it never runs
  a project's verification itself.
- The Forge control plane consumes this repository read-only via
  `scripts/forge_governance_adapter.py`
  (`openspec/specs/forge-governance-adapter/spec.md`); the adapter never
  writes, registers, or mutates anything.

Do not run these on a project's behalf. Link them, get approval, and let the
owning operator or pipeline run them.
