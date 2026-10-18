# Shared rules baseline

## Read first

- `templates/project/AGENTS.md` — the shared rules text a new project receives,
  including its Project facts block (stack, toolchain, verification command,
  layout) that the project keeps true as its structure changes.
- `templates/project/.ai-rules/workflow.md` and
  `templates/project/.ai-rules/completion.md` — the canonical rule files.
  `templates/project/**` as a whole is the single source; `AGENTS.md` is one
  file in it, not the only one.
- `openspec/specs/rules-baseline/spec.md` — the canonical contract.

## The work-on-`main` rule

Work directly on `main` by default; do not create a branch or worktree unless
the user asks for one or designates one.

It is canonical baseline text, carried byte-identically in
`templates/project/.ai-rules/workflow.md` and `templates/project/AGENTS.md`,
and copied to a new project by `scripts/init_project.py`. It is reported
fleet-wide as the `work-on-main rule` clause of
`scripts/rules_drift.py --root .. --check`, against the needle `` on `main` ``.

Two limits worth stating rather than discovering:

- The drift report is **triage, not a gate**. It is wired into no check, so
  this rule is *reported*, never *enforced*. Gate wiring is tracked
  separately as `rules-drift-gate-wiring` in `ROADMAP.md`.
- A repository that documents the rule in its own words and carries the
  marker `<!-- workspace-governance:rules-customized -->` in the rule file is
  skipped for that file.

## Run

- `python3 scripts/rules_drift.py --root .. --check` — a non-destructive,
  clause-level drift report. It changes no project file.
- Scaffold `HANDOFF.md` bodies surface from the audit as `HANDOFF_SCAFFOLD`;
  replacing them is product-owned work.

## Rules of the road

- `templates/project/` is the single source; `scripts/init_project.py` reads
  it from disk. Never copy a second set of the rules into a script.
- Repositories may document the shared rules in their own words; the drift
  report is triage, not a mandate to paste canonical paragraphs.
