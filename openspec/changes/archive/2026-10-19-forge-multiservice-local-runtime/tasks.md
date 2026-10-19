## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map CLI parsing, command catalog discovery, executable-resolution helpers, CI/runtime docs and existing `forge use` behavior.
- [x] 1.2 Record the exact Docker Compose and GitRunByKaru argument contracts; confirm no schema, registry or sibling-source mutation is needed.
- [x] 1.3 Add failing tests for command parsing, argv/cwd, missing executables and exit propagation before implementation; observed all three initial tests fail because `run` was not recognized.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add explicit `forge run compose` and `forge run local` CLI routes with bounded input validation and direct argv spawning.
- [x] 2.2 Preserve Compose project root/file/profile/service/action arguments and default `up` behavior without parsing or rewriting Compose configuration.
- [x] 2.3 Delegate local application directories to an installed GitRunByKaru binary, preserving supported runtime options and refusing implicit `npx` installation.
- [x] 2.4 Propagate child exit results and provide stable diagnostics for missing executables, invalid paths, spawn errors and signal termination.
- [x] 2.5 Add help examples and command-catalog guidance; document responsibility boundaries and installation prerequisites.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify exact argv and cwd for Compose and local modes, including spaces, repeated options and passthrough after `--`.
- [x] 3.2 Verify invalid inputs never spawn a child or modify project files, and missing tools never trigger network/package-manager installation.
- [x] 3.3 Verify child success, nonzero exit, signal termination and spawn failure behavior; preserve existing `forge use` behavior.
- [x] 3.4 Review command help/catalog, README and CI/CD local-runtime model for accurate ownership and no unsupported orchestration claims.

## 4. Verification

- [x] 4.1 Run focused CLI/process-boundary tests, affected command-catalog tests, formatter/linter and all applicable workspace checks. Focused CLI tests (7) and catalog tests (9) passed; the complete `scripts/check-tests.sh` workspace suite passed. Changed-file Rustfmt passed. Clippy remains blocked by existing `driftwatchdog` warnings.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, status/instructions checks and `git diff --check`. Name preflight, strict validation (119/119), status/instructions and whitespace checks passed.
- [x] 4.3 Run `forge gate --dry-run` and the full bounded `forge gate`; record the result in HANDOFF. Dry-run resolved all 8 required checks; fresh full Gate passed all 8 at revision `ea9d5e6091d0`, observed `2026-10-19T02:20:01Z`.
- [x] 4.4 Reconcile every impact-map item and requirement scenario, promote canonical specs and archive without `--skip-specs`.
- [x] 4.5 Commit implementation/tests/archive, update HANDOFF to the next eligible change, then make the HANDOFF-only commit.
