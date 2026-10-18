# Project Workflow

Use BFS → DFS → BFS. Map impact and evidence first, implement one coherent change, then re-audit callers, persistence, APIs, tests, security, and release boundaries.

Work directly on `main` by default; do not create a branch or worktree unless the user asks for one or designates one.

Run the shared governance checker and the project verification command before archive or completion. Run `node scripts/check-openspec-change-names.mjs` from the governance repository before OpenSpec status/instructions.

Keep at most one active OpenSpec change in a repository. Set `current_spec` in `HANDOFF.md` before work, complete and archive that change before creating another, and set `current_spec: none` when none remain.

Each completed OpenSpec spec/change requires exactly two commits: first the implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence commit. Stop after commit 2.

Docs follow the code: any directory, tool, or verification-command change updates the `AGENTS.md` Project facts block and the affected docs in the same change; Gate 3 of `.ai-rules/completion.md` fails a change that leaves them stale.

When an agent makes a decision error, add one decidable prohibition (a Never rule) to the lowest layer that governs it — `AGENTS.md` for entry-point-level facts, `.ai-rules/` for workflow detail — and record the dated cause in `HANDOFF.md` evidence. Never rules accrue from real failures, not from theory.
