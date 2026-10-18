# Tasks: ui-convention-guidance

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read the seven finding IDs from `scripts/ui_conformance.py` and align
  the skill reference and human documentation with that vocabulary.
- [x] 1.2 Read the four recorded policy rulings from
  `ui-adoption-migration.json`; carry them forward without reopening them.
- [x] 1.3 Confirm every `somedanote` path cited by the reference exists in the
  consumer repository.
- [x] 1.4 Keep the package boundary to governance guidance and a one-repository
  gate binder; leave consumer migration, Forge generation, checker behavior,
  baseline data, and UI artifacts to their owners.
- [x] 1.5 Record ownership, exact CLI behavior, manifest shape, conflict policy,
  sibling-checkout assumption, and verification oracle in `design.md`.
- [x] 1.6 Resolve stale `HANDOFF.md` pointer to the archived
  `ui-auth-adoption-evidence` change before implementation completion.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add exactly one UI route to `skills/workspace-governance/SKILL.md`.
- [x] 2.2 Add `skills/workspace-governance/references/ui-convention.md` with the
  checker codes, declaration model, adoption sequence, stack paths, and policy
  rulings.
- [x] 2.3 Add the one-repository `scripts/bind_ui_gate.py` binder with dry-run
  default, explicit `--apply`, `--check`, JSON output, schema refusal, conflict
  preservation, and idempotent writes.
- [x] 2.4 Add `tests/test_bind_ui_gate.py` coverage for success, failure,
  boundaries, command path validation, no-write behavior, and idempotence.
- [x] 2.5 Add `docs/ui-convention.md` with matching findings, baseline behavior,
  adoption guidance, and measured workspace counts.
- [x] 2.6 State that the bound command requires the governance checkout at its
  repository-relative sibling path; do not claim standalone-checkout support.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Align proposal, design, requirements, reference, and durable docs on
  the same seven finding codes and the existing one-consumer evidence.
- [x] 3.2 Confirm the binder is scoped to one target and preserves existing
  manifest intent on conflicts or unrecognized shapes.
- [x] 3.3 Confirm that no application source, UI package, baseline entry, or
  second repository is modified by this package.
- [x] 3.4 Run the binder test suite and review all positive, negative, and
  boundary cases. `python3 -m unittest tests.test_bind_ui_gate -v`: 30 tests,
  OK. Review found two deviations, fixed before archive: the design behaviour
  table claimed a non-zero default dry run, contradicting the design's own CLI
  section, the spec and the tests (fixed in `design.md`), and the spec's second
  run must report `no change` (added to the `ALREADY_BOUND` detail text and
  pinned in `tests/test_bind_ui_gate.py`).

## 4. Verification

- [x] 4.1 Run `python3 -m unittest tests.test_bind_ui_gate -v`.
  Result: `Ran 30 tests`, `OK`.
- [x] 4.2 Run `python3 scripts/check_skill_refs.py`.
  Result: `skill refs ok: SKILL.md + 9 references`, exit `0`.
- [x] 4.3 Run the scratch-repository dry-run, apply, and byte-identical second
  apply sequence described in `design.md`.
  Result: in a fresh `/tmp` scratch repository — default dry run exits `0` and
  leaves the manifest sha256 unchanged; `--check` on the unbound manifest exits
  `1`; `--apply` exits `0` and writes the two entries; a second `--apply`
  reports `already_bound … no change`, exits `0`, and the manifest sha256 is
  unchanged byte-for-byte; `--check` after binding exits `0`. No real
  workspace repository was written.
- [x] 4.4 Run `python3 scripts/workspace_check.py --self-check`.
  Result: `Summary: errors=0 warnings=0`, exit `0`.
- [x] 4.5 Run `openspec validate --changes --strict --no-interactive` and
  `openspec validate --specs --strict --no-interactive`.
  Result: changes `15 passed, 0 failed` (including
  `✓ change/ui-convention-guidance`); specs `15 passed, 0 failed`. Also
  `node scripts/check-openspec-change-names.mjs`: `OpenSpec change names valid`.
- [x] 4.6 Run `git diff --check` and confirm only the selected repository has
  scoped changes; leave `HANDOVER.md` and `driftwatch.toml` untouched.
  Result: `git diff --check` clean (exit `0`); the touched paths are exactly
  the ones named in `design.md`; `HANDOVER.md` and `driftwatch.toml` remain
  untracked and untouched.
- [x] 4.7 Archive only after every required verification has evidence, then
  make the implementation/archive commit followed by the HANDOFF-only commit.
  Archived as `2026-10-09-ui-convention-guidance` with the canonical spec
  promoted to `openspec/specs/ui-convention-guidance/spec.md` (no
  `--skip-specs`). Full-suite note: `python3 -m unittest discover -s tests`
  runs 935 tests with the same 2 pre-existing failures already recorded in
  `HANDOFF.md` (`test_identity_and_taxonomy` registry drift on `somodanote`,
  `test_promote_project` root-data accounting) — unchanged by this change.
