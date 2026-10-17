# Project Agent Instructions

- Read `HANDOFF.md` before work.
- Use OpenSpec for non-trivial changes.
- Use BFS → DFS → BFS for impact, implementation, and regression review.
- Run the declared verification command before completion.
- Build/test/skeleton success is not runtime or release evidence.
- Each completed OpenSpec spec/change requires exactly two commits: first the implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence commit.

Detailed rules: `.ai-rules/workflow.md` and `.ai-rules/completion.md`.

## Git workflow

- Work on `main`. Do not create a branch or a worktree unless the user explicitly asks.
- Never push. The user owns publishing.
- Stage explicit paths; never `git add -A`.
