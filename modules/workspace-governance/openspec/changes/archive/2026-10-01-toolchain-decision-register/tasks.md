# Tasks: toolchain decision register

## 0. Caveats and confirmations

- [ ] **Fail-closed is inherited, not suggested**: every loader error names
  file, entry index, and field; no code path may return an empty list for a
  broken register (vocabulary rule).
- [ ] **Do not weaken the public-site boundary**: D2's `policy_boundary`
  field must restate the root HANDOFF exclusion of shared login/analytics
  for the GitHub Pages catalog; a string assertion in tests guards it.
- [ ] **Registry ids are the source of truth** for `owner_project` and
  `downstream[].repo`; do not add project aliases in the register.
- [ ] **No sibling repository edits**; forge shim exposure is a separate
  forge-owned change (design L6).
- [ ] **`current_spec`**: set exactly one pointer line in `HANDOFF.md` at
  cycle start per repo cadence; two-commit completion sequence.

## 1. BFS — Baseline and impact coverage

- [ ] Read `scripts/vocabulary.py` loader discipline, `workspace_check.py`
  self-check + finding-code conventions (`finding()` helper, levels/codes),
  and `tests/` layout for an analogous fixture-based test module.
- [ ] Map REG-001..004 scenarios to test names in `tests/test_decisions.py`;
  confirm proposal ↔ design ↔ spec consistency.
- [ ] Inventory the four `TOOLCHAIN_ROADMAP.md` decision entries and every
  registry id they reference (all must exist in `projects.json` before
  seeding).
- [ ] Verify the implementation-handoff gate: python 3.11 stdlib-only,
  governance-owned files only (design §1).

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 REG-001: `decisions.json` skeleton (`version: 1`, empty list) and
  `scripts/decisions.py` — `Decision` dataclass, `load_decisions()` with the
  §4 rule table and `DecisionError`; unknown top-level key fatal, unknown
  entry key tolerated.
- [ ] 2.2 REG-002/003: `validate_against_registry(decisions, registry_ids)`;
  id pattern/uniqueness, status enum, ISO date, kebab change names;
  supersede documented in `docs/toolchain-decisions.md` rules section.
- [ ] 2.3 REG-004: `workspace_check.py` wiring — self-check ERROR when the
  register is missing; portfolio-audit single WARNING code; `--decisions`
  flag printing the summary (JSON under `--json`, exit 1 on loader error).
- [ ] 2.4 REG-003 seeding: transcribe accepted D1–D4 with dates, owners,
  decision text, D2 `policy_boundary`, and downstream references
  (`openpanel`/`ariadex`… names as registered) matching roadmap B rows.
- [ ] 2.5 REG-004 docs: `docs/toolchain-decisions.md` human view; append the
  D3 ownership split (schema=platform-contracts, instances=governance
  registry) to `docs/capabilities.md`; one README line.

## 3. BFS — Cross-surface regression and completeness

- [ ] Run `python3 -m unittest discover -s tests` — new module green,
  existing 167 tests unmodified and passing.
- [ ] `workspace_check.py --self-check` 0 errors; temp-copy missing-register
  run yields the exact new ERROR code; `--decisions` output shape asserted
  against design §4.
- [ ] Portfolio audit `--root .. --audit --json` shows no new project-level
  errors attributable to this change; jenkins manifest check untouched.
- [ ] Confirm vocabulary.json, profiles/, projects.json, and all sibling
  repos are byte-identical (`git diff --stat` names only §1 files).

## 4. Verification

- [ ] `python3 -m unittest discover -s tests`
- [ ] `python3 scripts/workspace_check.py --self-check`
- [ ] `node scripts/check-openspec-change-names.mjs`
- [ ] `openspec validate --changes --strict --no-interactive`
- [ ] `git diff --check`
- [ ] Record evidence in `HANDOFF.md`; commit implementation+archive, then
  the HANDOFF-only pointer commit; stop.
