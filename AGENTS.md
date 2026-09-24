# Forge agent instructions

## Scope and source

- Read [README.md](README.md), [ROADMAP.md](ROADMAP.md) and [HANDOFF.md](HANDOFF.md).
- [requirement.md](requirement.md) is the authoritative full product brief.
- This is a planning-only repository until implementation evidence says otherwise.
- Preserve existing user files and unrelated worktree changes.
- Use direct technical language; omit greetings and filler.

## Required workflow

- Non-trivial changes require an active OpenSpec change.
- Follow BFS → DFS → BFS in planning and implementation.
- Read [.ai-rules/workflow.md](.ai-rules/workflow.md).
- Completion is governed by [.ai-rules/completion.md](.ai-rules/completion.md).
- Applicable concerns are in [.ai-rules/concerns/](.ai-rules/concerns/).
- Load [.ai-rules/architecture.md](.ai-rules/architecture.md) for boundary changes.
- Run `node scripts/check-openspec-change-names.mjs` before selecting a change.
- Use `openspec list` and the roadmap to select one eligible change.
- Set the single `current_spec: <active-change-name>` line in HANDOFF before work.
- Implement one change → local verify/Gate → strict validate → archive.
- Never archive with `--skip-specs`; promote canonical specs.
- Commit only related implementation, tests and specs.
- Update HANDOFF and its pointer, commit handoff, then stop; do not push.
- Advance the pointer after archive; remove it when no active changes remain.
- Planning artifacts do not authorize implementation, archive or commits.

## Validation and invariants

- Run `node scripts/check-openspec-change-names.mjs`.
- Run `openspec validate --all --strict --no-interactive`.
- Run `git diff --check` and review newly added files too.
- Record actual build/test commands when application tooling is introduced.
- Build success, test success and skeleton completion are not by themselves DONE.
- Verify all scoped requirements, scenarios, callers and failure boundaries.
- Run applicable local checks before archive; CI is a second layer.
- Gate runtime consumption exists (`forge gate`, `gate-runtime-evidence`); no executed gate pass is recorded for this repository and no shared completion Gate is configured.
- If enabled later, Gate FAIL or unresolved REVIEW_REQUIRED blocks completion.
- Deterministic assembly precedes AI; generated projects must work without Forge.
- Do not replace DriftWatch, the existing PTY manager or external content/analytics.
- Do not force higher maturity, publish, push or deploy implicitly.
