# Tasks: cli-facade

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read proposal/design/spec, `.ai-rules/workflow.md`, and confirm the Rust 2021 Forge-local boundary and the no-AI-rewriting requirement.
- [x] 1.2 Confirm the consolidated crates build into `target/{debug,release}` and that `cargo-sonar` is an alias for package `cargo-sonar-boost`.
- [x] 1.3 Enumerate the resolution/failure matrix (CF-001..CF-004) and the six failure boundaries.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement `PRODUCTS` catalog and `resolve_product`/`workspace_root`/`find_in_repo_binary`/`find_in_path` in `src/cli/use_cmd.rs`.
- [x] 2.2 Implement `delegate`/`delegate_npx` returning the child exit code via `ExitCode`.
- [x] 2.3 Implement `cmd_use` (list, rust, external, unknown) and `render_help`/`render_list`/`emit_error` (JSON shape).
- [x] 2.4 Add `Commands::Use` (early-return `ExitCode`) and `Commands::Help` (`disable_help_subcommand = true`) in `src/main.rs`; wire imports and the match arm.
- [x] 2.5 Document the facade in README.md and docs/facade.md.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Revisit every failure boundary with a manual run: no-id, not-built rust, unknown (no npx), external PATH, external npx, spawn failure, JSON shape.
- [x] 3.2 Revisit alias resolution (`cargo-sonar`, `grbk`) and cwd-independent resolution.
- [x] 3.3 Revisit that delegation preserves args/flags/exit and performs no output rewriting.

## 4. Verification

- [x] 4.1 Run `cargo build --workspace`, `cargo check --workspace`, `git diff --check`; record exact results.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate cli-facade --strict --no-interactive`, and `openspec validate --all --strict --no-interactive`; record PASS.
- [x] 4.3 Run `forge gate --dry-run`, then `forge gate --timeout-secs <bounded>`; record verdict and any pre-existing unrelated failure (the change introduces no new failure).
- [x] 4.4 Archive `cli-facade` without `--skip-specs`; verify the `cli-facade` canonical spec promotes.
- [x] 4.5 Commit archived package, promoted canonical spec, and HANDOFF; do not push.
- [x] 4.6 Update HANDOFF evidence and remove the `current_spec` pointer (no active changes remain); commit HANDOFF separately.
