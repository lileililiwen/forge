# Tasks: Profile-aware Forge Gate

## 1. BFS — Baseline and impact coverage

- [x] Confirm the package impact map against CLI, bundled Gate, evidence readers, hosted evidence, delivery, standard assets and callers.
- [x] Add acceptance fixtures for fast/full planning, legacy evidence decoding, hosted rejection and delivery refusal before implementation.
- [x] Confirm implementation handoff: Rust stable 2021, Forge owns the profile, no sibling edits, terminal-only UX.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add typed Gate profile and profile-aware planning to the bundled runtime; preserve mandatory baseline and full default.
- [x] Expose `forge gate --profile`, forward profile to the private engine, and project the saved profile in human/JSON/status evidence.
- [x] Extend evidence compatibility and enforce full-only hosted upload/ingest and delivery preflight.
- [x] Add immutable `baseline-service@1.6.0` and `baseline-client@1.3.0` assets, update current defaults, and verify ownership receipts/provenance.
- [x] Update CI/CD model and promote canonical capability deltas.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify fast profile cannot satisfy delivery or hosted ingestion and cannot suppress malformed manifest refusal.
- [x] Verify full profile has the same extension selection and failure semantics as current behavior.
- [x] Verify status, CLI JSON, hosted envelope, generated service/client packs, and old pinned snapshots remain compatible.
- [x] Review all original impact surfaces, including no sibling-source coupling, deterministic rendering and unchanged shared Gate-result contract.

## 4. Verification

- [x] Run focused Gate, delivery, hosted-evidence and standard snapshot contracts; record exact commands and results.
- [x] Run formatting, build, shell/Python syntax, name preflight, strict OpenSpec validation and `git diff --check`.
- [x] Record fast elapsed time; run Gate dry-run and full bounded Gate and record verdict in HANDOFF.
- [x] Archive with canonical specs promoted; commit implementation/archive, then HANDOFF only.
