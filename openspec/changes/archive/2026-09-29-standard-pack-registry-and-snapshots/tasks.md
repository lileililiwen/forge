# Tasks: standard-pack-registry-and-snapshots

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map profile registry, generator, asset ownership receipts, SQLite migrations, CLI dispatch, and existing generation contracts.
- [x] 1.2 Freeze pack descriptor, receipt, lifecycle, conflict, atomicity, and standalone fixtures.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add versioned pack descriptors, support state, compatibility, and digest validation.
- [x] 2.2 Render `.standard/`, local verification, CI, quality, Compose, and receipt files atomically.
- [x] 2.3 Add `standard list|inspect|check|diff|upgrade` through Core contracts and CLI.
- [x] 2.4 Add deterministic, conflict, refusal, generated-build, and no-sibling-checkout tests.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove existing `forge new`, import, profile, registry, and native-generation behavior remains compatible.
- [x] 3.2 Exercise supported, deprecated, missing-asset, modified-file, and external-template-unavailable paths across CLI and persistence.

## 4. Verification

- [x] 4.1 Run formatting, build, clippy, focused/full tests, Compose config checks where available, strict OpenSpec validation, and `git diff --check`.
- [x] 4.2 Record native toolchain and Docker evidence separately from rendering/contract evidence.
