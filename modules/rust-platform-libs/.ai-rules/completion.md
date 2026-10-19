# Rust Platform Completion

Completion requires every requirement and scenario verified, public crate APIs
reviewed, compatibility fixtures passing, dependency direction preserved,
consumer boundaries checked, the project verification command and applicable
Gate passing, and strict OpenSpec validation passing. Build/test/skeleton
success is not runtime or release evidence. Unverified consumer adoption must
remain explicit.

Each completed OpenSpec spec/change requires exactly two commits: first the
implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence
commit. Stop after commit 2.
