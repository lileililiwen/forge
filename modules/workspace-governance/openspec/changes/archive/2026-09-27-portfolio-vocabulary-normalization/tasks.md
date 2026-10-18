# Tasks: Normalize the portfolio vocabulary

## 0. Caveats and confirmations

- [ ] **Confirm the canonical vocabulary before enforcing it.** The 15-profile and
  12-kind sets in the design are a proposal derived from the values observed on
  2026-09-26, not a measurement. Enforcement turns non-canonical values into
  ERROR/WARNING, so review and agree the sets first.
- [ ] **Extend the remap tables, do not special-case.** The table lists only the
  values seen at authoring time; scan every declaration for any other
  non-canonical value and add it to the table rather than adding a code branch.
- [ ] **Prove no project changes its required-file outcome.**
  `profile_requirements` reads the registry profile only, but verify per project
  that normalizing a declaration `kind`/`profile` changes no required file, CI or
  OpenSpec outcome.
- [ ] **The secret list must never narrow.** It must stay a superset of today's
  `SECRET_WORDS` and of `platform-contracts/schemas/registry.json`'s
  `secret_field_substrings`; dropping a term silently weakens the audit.
- [ ] **Reconcile the profile documents.** `profiles/*.json` covers only 6 of the
  profiles in use; `PROFILE_DOC_UNKNOWN` must not fire for the canonical set, so
  add or rename documents as part of this change.
- [ ] **`kind` stays descriptive.** Do not make it drive requirements or
  deployment; only `profile` does that.
- [ ] **Do not touch the other copies.** `platform-contracts` and
  `rust-platform-libs` own their secret lists; reconciliation is package 7's job.
- [ ] **Stage only this change directory.** This repository has a pre-existing
  uncommitted `.project.json` edit and an untracked `MATURITY_PROGRESS_SUMMARY.md`;
  they must not be swept into the change's commits.

## 1. BFS — Baseline and impact coverage

- [ ] Record the baseline: enumerate every distinct registry `profile` (13) and
  declaration `kind` (22) / `profile` (17) with the projects that use each.
- [ ] Confirm the registry's in-use profiles are already within the canonical
  set, and that declaration `kind`/`profile` never affect `profile_requirements`.
- [ ] Add failing tests for vocabulary load, canonical-set enforcement, remap
  acceptance and the secret scan under `tests/test_vocabulary.py`.
- [ ] Verify the implementation-handoff gate: Python 3.11,
  `workspace-governance`, no shared-library boundary.

## 2. DFS — Requirement-by-requirement implementation

- [ ] R1: add `vocabulary.json` (`schema_version`, `profiles`, `kinds`,
  `placeholder_markers`, `secret_field_substrings`); fail hard when it is
  missing, unparseable or contains an empty list or duplicate entry.
- [ ] R2: load the canonical profile set in `workspace_check.py`; emit ERROR
  `PROFILE_UNKNOWN` for a value outside it; add ERROR `PROFILE_DOC_UNKNOWN` when
  a `profiles/<name>.json` file names a non-canonical profile.
- [ ] R3: add `validate_kind` emitting WARNING `KIND_UNKNOWN`.
- [ ] R4: replace `SECRET_WORDS` in `workspace_check.py` and
  `DEFAULT_PLACEHOLDER_MARKERS` in `product_code_quality.py` with values loaded
  from `vocabulary.json`; keep the per-project `placeholder_markers` override.
- [ ] R5: remap every non-canonical declaration `kind`/`profile` per the design's
  remap tables, and scan for any value not in the tables.
- [ ] Update `README.md` and `ROADMAP.md` as documentation consequences.

## 3. BFS — Cross-surface regression and completeness

- [ ] Re-run the audit and confirm no project changed its required-file or
  deployment outcome; only `KIND_UNKNOWN`/`PROFILE_UNKNOWN` findings may change.
- [ ] Confirm `product_code_quality.py` still PASSes the governance repo and a
  project with a custom `placeholder_markers` override still honours it.
- [ ] Confirm the secret scan still rejects a fixture with a secret-like key.
- [ ] Confirm `forge_governance_adapter.py` still emits a valid observation for a
  clean, an unknown and a failing project.

## 4. Verification

- [ ] `python3 -m unittest discover -s tests -v`.
- [ ] `python3 scripts/workspace_check.py --root .. --audit --json` — 0 errors,
  no `KIND_UNKNOWN`.
- [ ] Direct scan: zero non-canonical `kind`/`profile` values remain.
- [ ] `python3 scripts/workspace_check.py --self-check` and
  `python3 scripts/maturity_audit.py --root .. --json` (counts must match).
- [ ] `node scripts/check-openspec-change-names.mjs` and
  `openspec validate --all --strict --no-interactive`.
- [ ] Record actual outputs; report any environment-blocked check with its exact
  next action rather than as passing.
