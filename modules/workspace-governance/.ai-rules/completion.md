# Completion Rules

A governance change is complete only when:

- registry and schema requirements are covered;
- project declarations and generated deployment output are validated;
- required scenarios have tests;
- missing or invalid projects fail closed;
- `python3 -m unittest discover -s tests -v` passes;
- a repository carrying unresolved gate debt opens no new OpenSpec change until
  the debt is bound or an exception is recorded — enforced by
  `scripts/workspace_check.py`, not by this file;
- `python3 scripts/workspace_check.py --self-check` passes;
- `python3 scripts/auto_gate.py --self-check --no-deployment` passes
  (the unattended entry point — also runs the Jenkins manifest when a
  workspace root is supplied);
- deployment-related changes pass `python3 scripts/jenkins_manifest.py --check`;
- `node scripts/check-openspec-change-names.mjs` passes (the active
  OpenSpec change name matches `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`);
- `openspec validate --all --strict --no-interactive` passes (every
  canonical spec and every active change validates);
- `git diff --check` passes;
- no secrets appear in tracked files;
- `HANDOFF.md` records exact evidence and unresolved adoption gaps.

Build/test/skeleton success is not runtime or release completion.

Each completed OpenSpec spec/change requires exactly two commits: first the implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence commit. Stop after commit 2.
