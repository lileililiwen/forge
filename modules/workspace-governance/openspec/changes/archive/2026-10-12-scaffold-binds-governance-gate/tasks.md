# Tasks: Bind governance checks when scaffolding a project

## 1. BFS — Baseline and impact coverage

- [x] Confirm current initializer creates the manifest without invoking the binder and accepts arbitrary profile values.
- [x] Confirm binder owns `PROFILE_DEFAULTS` and currently omits ten Driftwatchdog workspace profiles.
- [x] Map existing target preservation behavior, CLI selectors, and the empty-scaffold unresolved case.
- [x] Record Python module boundaries, invocation contract, failure behavior, and excluded runtime policy in proposal/design.
- **Done when** every changed surface and observable acceptance boundary is named.

## 2. DFS — Profile validation and binder sequencing

- [x] Add focused tests for profile registry completeness and typo refusal before target creation.
- [x] Add initializer integration tests proving binder runs after manifest creation for only the target repository.
- [x] Add the ten supported workspace profile names to the binder registry.
- [x] Invoke the binder in apply mode after scaffold writes and propagate errors without deleting partial scaffold output.
- [x] Verify derivable commands are written and unresolved commands remain absent for an empty scaffold; the real temporary scaffold bound the repository check and retained four unresolved concerns.
- **Done when** focused tests cover positive and negative paths and no command is fabricated.

## 3. BFS — Cross-surface regression and completeness

- [x] Check command construction, argument boundaries, root selection, and preservation of existing files.
- [x] Run focused initializer and binder tests (153 passed, 1 skipped), then the complete Python suite (839 passed, 1 skipped).
- [x] Run strict OpenSpec validation (12/12), workspace self-check (0 errors, 0 warnings), auto-gate (PASS, 0 errors, 0 warnings), change-name check, and `git diff --check`.
- **Done when** all observations agree with this change's scenarios and unrelated dirty files remain excluded.

## 4. Verification

- [x] Record exact verification results in `HANDOFF.md`.
- [x] Commit explicit spec, code, tests, and docs paths as commit 1; commit `HANDOFF.md` only as commit 2.
- [x] Keep the change active; do not archive without authorization.
