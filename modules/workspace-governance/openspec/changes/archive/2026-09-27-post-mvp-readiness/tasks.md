# Tasks: Post-MVP readiness — README usage, terminal capture, vocabulary

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Inventory the command surface (`workspace_check.py`, `auto_gate.py`,
  `product_code_quality.py`, `maturity_audit.py`, `rules_drift.py`,
  `init_project.py`, `promote_project.py`, `jenkins_manifest.py`,
  `forge_governance_adapter.py`) and the README sections that mention them.
- [x] 1.2 Capture a real `workspace_check.py --self-check` and
  `--audit --json` session; redact tokens, host usernames, absolute private
  paths, and secret-shaped values.
- [x] 1.3 Inventory the registry/evidence vocabulary and its canonical sources
  (`docs/capabilities.md`, `docs/product-code-quality.md`,
  `docs/deployment-promotion.md`, `openspec/specs/*`).
- [x] 1.4 Confirm the boundary: no script, schema, template, registry,
  promotion, or canonical-spec change; record the keep-local reasons for
  `platform-contracts`, `driftwatchdog`, `forge`, and the platform repos.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the consolidated Usage section with one copy-paste example per
  command.
- [x] 2.2 Add the checked-in capture under `docs/assets/`, embed it in the
  README, and add the "Screenshots are not applicable" note.
- [x] 2.3 Add the Vocabulary section (or `docs/vocabulary.md`) defining
  declaration, `evidence_status`, `capabilities`, `release_evidence`,
  `deployable`, the promotion ledger, and finding codes, each linking its
  canonical source.
- [x] 2.4 Cross-link the usage, capture, and vocabulary from the README; keep
  the ownership/mandatory-command/promotion/Jenkins sections accurate.
- [x] 2.5 State explicitly that a declaration and registry presence are not
  implementation evidence.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify every documented command exists and runs `--help`; every new
  and pre-existing README relative link resolves; no external image host.
- [x] 3.2 Verify the capture is redacted, reproducible, and matches a real run;
  screenshots are stated not applicable.
- [x] 3.3 Verify the vocabulary links the canonical sources and does not restate
  policy; the intent-vs-evidence invariant is stated.
- [x] 3.4 Confirm the diff touches only `README.md`, `docs/**`, and the change
  folder, and that the pre-existing uncommitted `.project.json` and untracked
  `MATURITY_PROGRESS_SUMMARY.md` are not swept into the change.

## 4. Verification

- [x] 4.1 `python3 -m unittest discover -s tests` — 122-test baseline passes.
- [x] 4.2 `python3 scripts/workspace_check.py --self-check` — 0 errors, 0
  warnings; `python3 scripts/workspace_check.py --root .. --audit --json` —
  counts unchanged.
- [x] 4.3 `node scripts/check-openspec-change-names.mjs` and
  `openspec validate --all --strict --no-interactive` — pass.
- [x] 4.4 `git diff --check` — clean.
- [x] 4.5 Record actual outputs; report any environment-blocked check with its
  exact next action rather than as passing.
