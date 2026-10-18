# Tasks: Restore declaration and registry integrity

## 1. BFS — Baseline and impact coverage

- [x] Capture the baseline: run
  `python3 scripts/workspace_check.py --root .. --audit --json` and record the
  exact ERROR/WARNING set as this change's before-evidence. Baseline was
  14 ERROR / 18 WARNING.
- [x] Map each affected code (`DISCOVERED_UNREGISTERED`, `DECLARATION_MISSING`,
  `DECLARATION_ID`, `DECLARATION_SCHEMA`, `VERIFICATION_COMMAND`,
  `DEPLOYMENT_PROMOTION_STALE`) to its consumers
  (`forge_governance_adapter.py`, `auto_gate.py`, `jenkins_manifest.py`,
  `maturity_audit.py`); none relies on `DISCOVERED_UNREGISTERED` staying at
  WARNING.
- [x] Confirm the canonical `id` for each of the 11 unregistered directories
  using the design's derivation rule; aliases recorded:
  `lileililiwen.github.io` → `lileililiwen-site`.
- [x] Verify the implementation-handoff gate: language Python 3.11, project
  `workspace-governance`, no shared-library boundary.
- [x] Add failing test skeletons for R1–R5 under `tests/` using synthetic
  workspaces; wired as `test_registry_integrity.py`, `test_init_project.py`,
  `test_maturity_audit.py`.
- [x] Confirm proposal, design and spec agree on the five requirement names.

## 2. DFS — Requirement-by-requirement implementation

- [x] R1: add the 11 registry entries (`id`, `path`, `adoption`) to
  `projects.json`; add duplicate-`id` and duplicate-`path` guards to
  `workspace_check.py` (`PROJECT_DUPLICATE`, `PROJECT_PATH_DUPLICATE`).
- [x] R1: raise `DISCOVERED_UNREGISTERED` to ERROR by setting
  `discovery.unregistered_policy: "fail"`, only after the registry entries are
  in place.
- [x] R2: fix `id` in `campusMesh` (`campus-mesh`) and `careBridge`
  (`care-bridge`); add `id` to `retakeKit` (`retake-kit`) and `trailCrew`
  (`trail-crew`); add `schema_version: 1` to `citylens`, `retakeKit`,
  `trailCrew`, `vowra`; convert `vowra`'s `verification_command` into
  `verification.command` with `gate_runtime` and `evidence_status`, and
  normalise `lifecycle` to the schema enum.
- [x] R2: create `.project.json` for `dharmatlas`, `hookit`,
  `lileililiwen.github.io` and `openTeacherKit` from `init_project.py` output,
  then set each real `verification.command`.
- [x] R3: implement derived-id logic and the conflicting-`--id` refusal in
  `init_project.py`, including the dotted-directory alias path.
- [x] R4: make `maturity_audit.py` report `workspace_check`'s `errors`/`warnings`
  through `authoritative_summary()`; exit non-zero with the reason when the
  audit cannot be read.
- [x] R5: surface stale promotions as a WARNING naming both revisions and the
  remedy, and record the re-promotion triage decision in the adoption queue.
- [x] Update `README.md`, `ROADMAP.md` and `docs/adoption-queue.md` as
  documentation consequences.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-run the audit and confirm no previously-passing project changed level
  except the 11 intended registrations.
- [x] Confirm `forge_governance_adapter.py` still returns the correct status for
  one clean, one unregistered and one unknown project.
- [x] Confirm `jenkins_manifest.py --check` still emits exactly the 7 promoted
  projects and no unpromoted product.
- [x] Confirm the `crossalheart` `REQUIRED_FILE` finding still surfaces (it is
  deferred, and must not be silently dropped).
- [x] Confirm no secret, host path or credential entered the registry or any
  declaration.

## 4. Verification

- [x] `python3 -m unittest discover -s tests -v` — 112 tests, OK.
- [x] `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- [x] `python3 scripts/workspace_check.py --root .. --audit --json` — 2 errors
  (`crossalheart` `REQUIRED_FILE`, deferred) / 7 warnings (triaged stale
  promotions); 0 findings for the change's six in-scope codes.
- [x] `python3 scripts/maturity_audit.py --root .. --json` — reports 2 errors /
  7 warnings, matching the audit exactly.
- [x] `python3 scripts/jenkins_manifest.py --check --root /home/paul/code` —
  7 projects, 0 errors.
- [x] `node scripts/check-openspec-change-names.mjs` and
  `openspec validate --all --strict --no-interactive` — valid; 5 passed,
  0 failed.
- [x] `git diff --check` — clean.
