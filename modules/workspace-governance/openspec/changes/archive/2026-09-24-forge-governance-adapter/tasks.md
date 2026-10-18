# Tasks: Ship the Forge governance adapter

## 1. BFS — Baseline and impact coverage

- [ ] Pin the Forge v0.1.0 request/observation field list from the Forge
  contract and record it in the test fixtures header.
- [ ] Inventory reusable audit internals (registry load, per-project
  findings, SECRET_WORDS scrubbing) and pick the call path (in-process vs
  `--project --json`).
- [ ] Map README/HANDOFF locations to update and confirm unittest
  discovery covers the new test module.
- [ ] Add fixture registry/finding sets for pass/fail/blocked/unknown and
  malformed stdin before implementing.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Implement request validation (contract/action/ids) with the exit-2
  rule.
- [ ] Implement project lookup, audit evaluation and the status mapping.
- [ ] Implement observation rendering: echoed ids, protocol_version,
  revision probe, evidence cap + secret scrubbing, single-document
  stdout.
- [ ] Implement exit-code table (governance fail is still exit 0; internal
  audit error is non-zero).

## 3. BFS — Cross-surface regression and completeness

- [ ] Tree-hash before/after smoke test proves read-only behavior.
- [ ] Re-run `workspace_check.py --self-check`, the full unittest suite and
  `auto_gate.py` to confirm existing governance flows are unchanged.
- [ ] If the Forge checkout is available locally, run one real adapter
  round trip through the contract (`echo request | adapter` against a
  sample observation parser) or record the exact blocker as the next
  action.
- [ ] Update README optional-consumers section.

## 4. Verification

- [ ] `python3 -m unittest discover -s tests -v`.
- [ ] `python3 scripts/workspace_check.py --self-check`.
- [ ] `node scripts/check-openspec-change-names.mjs` (now that an
  openspec/ directory exists) and strict `openspec validate` for this
  change.
- [ ] Record actual outputs; environment-blocked checks name the exact
  next action and never a pass.
