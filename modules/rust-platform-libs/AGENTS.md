# Rust Platform Libraries Agent Instructions

- Read `HANDOFF.md` before work.
- Use OpenSpec for every non-trivial change.
- Follow BFS → DFS → BFS and select one active change at a time.
- Keep crates small, opt-in, provider-neutral, and application-owned at the
  persistence/domain boundary.
- Do not add universal Axum, SQLx, login, dashboard, or business behavior to
  the foundation crates.
- Build/test/skeleton success is not runtime or release evidence.
- Each completed OpenSpec spec/change requires exactly two commits: first the
  implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence
  commit.

Detailed rules: `.ai-rules/workflow.md` and `.ai-rules/completion.md`.

## Git workflow

- Work on `main`. Do not create a branch or a worktree unless the user explicitly asks.
- Never push. The user owns publishing.
- Stage explicit paths; never `git add -A`.
