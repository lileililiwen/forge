# Tasks: Generate workspace metadata so new projects are adoptable

## 1. BFS — Baseline and impact coverage

- [x] Extract the sibling `.project.json` schema (required keys, profile
  vocabulary, verification semantics) into a fixture schema note.
- [x] Map generator stages, ownership receipts, import/upgrade callers.
- [x] Add per-profile mapping table entries to descriptors with a
  `no-mapping` sentinel for unmapped profiles.
- [x] Add test skeletons: emission matrix, opt-out parity, import
  preservation, ownership conflict.

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement emission at generation (staged files, hash-covered).
- [x] Implement mapping lookup from profile descriptors with honest
  omission and a printed note for unmapped profiles.
- [x] Enforce honesty rules (planned evidence, null deployment,
  gate_runtime only when declared).
- [x] Record import-time observation of existing metadata without writes.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-verify generated projects build/test natively with Forge and the
  file absent (inertness) and present.
- [x] Re-verify `forge new` determinism hashes with flag off equal the
  prior release byte-for-byte.
- [x] Re-verify upgrade ownership conflict and doctor presence reporting.
- [x] Confirm no machine-specific paths and no guessed profile strings.

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo build`; `cargo test
  --all-targets`; `cargo clippy --all-targets -- -D warnings`.
- [x] Validate an emitted fixture against the sibling's checker vocabulary
  locally (`workspace_check.py --project` on a scratch tree) and record the
  actual result.
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate
  --all --strict --no-interactive`; `git diff --check`.
