# Rust Platform Workflow

Use BFS → DFS → BFS. Map crate boundaries, public traits, serialization,
consumer compatibility, security invariants, dependency costs, and tests before
implementation. Implement one coherent contract family, then re-audit all
consumers and feature combinations.

Run `node scripts/check-openspec-change-names.mjs`, `cargo fmt --check`, the
declared tests, applicable Gate, and strict OpenSpec validation before archive.
New framework/provider dependencies require two-consumer evidence or an
explicitly recorded design decision.

Each completed OpenSpec spec/change requires exactly two commits: first the
implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence
commit. Stop after commit 2.
