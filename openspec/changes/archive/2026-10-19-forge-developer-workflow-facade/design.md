# Design: Forge developer workflow facade

## Implementation boundary

Forge Rust CLI only. Add `src/cli/module.rs`, register its types in
`src/cli/mod.rs` and `src/main.rs`, add focused integration tests under
`tests/`, and update root help/README/facade documentation plus the command
catalog contract if its derived inventory needs an explicit expectation.
Do not edit module Gate manifests or native check scripts.

## Language and runtime

Rust 2021, root pinned toolchain from `rust-toolchain.toml`, Clap command
definitions in `src/main.rs`, serde/serde_json for `.project.json`, existing
`ForgeError`/`Format` output conventions, and existing `cmd_gate` for execution.
Verification uses Rustfmt, `cargo test --test module_workflow_contract`, the
relevant command catalog tests, `scripts/check-tests.sh`, OpenSpec validation
and Forge Gate.

## Ownership and shared code

Forge owns the module resolver and command. Each direct child under
`<forge-root>/modules/` owns its `.project.json` identity and Gate manifest.
Forge does not read `modules/workspace-governance/projects.json`, project
metadata bundles, or sibling checkouts. The resolver reuses the executable-
relative root helper in `cli::use_cmd`; make that helper `pub(crate)` instead
of adding a second root-discovery algorithm. Existing Gate code remains the
only execution owner.

## User interface

```text
forge module list [--format human|json|ndjson]
forge module verify <ID> [--dry-run] [--profile fast|full] [--timeout-secs N]
```

`list` prints sorted rows with ID, stack, profile and relative path. JSON emits
the same rows as an array. `verify` accepts only an ID from the discovered
catalog and delegates to `cmd_gate` with the canonical module path. Human output
is the existing Gate output. JSON output retains existing Gate JSON and typed
errors. Help includes that modules are source-checkout modules and that local
verification does not invoke hosted CI. The CLI is the full UI; no browser,
mobile, portal or accessibility behavior changes. CLI tests assert its help,
list, refusal and Gate invocation behavior.

## Module catalog and data flow

1. Resolve Forge root from the running executable using the existing
   `workspace_root()` (`target/{debug,release}/forge` → repository root).
2. Require `<root>/modules` to be a directory. Do not search CWD, parent
   workspaces, `PATH`, environment overrides or sibling projects.
3. Read immediate child directories only. Ignore children without
   `.project.json`; for a declared child, parse JSON and require a valid
   kebab-case `id`, non-empty `stack`, and `profile`.
4. Require at least one supported Gate declaration: `gate.toml` or
   `.ai-gate/gate.yaml`. Canonicalize the child and require it to remain
   beneath canonical `<root>/modules`; reject symlink escapes. Preserve the
   relative child path for rendering.
5. Sort by project ID and reject duplicate IDs. A malformed declared module
   fails the catalog rather than silently disappearing.
6. `list` renders the immutable catalog. `verify ID` does an exact ID lookup,
   then passes the canonical path and existing options once to `cmd_gate`.
   Gate owns profile selection, dry-run behavior, bounded commands, evidence,
   persistence, redaction and verdict exit code.

There is no new persistence or mutable catalog. Each invocation re-reads
declarations, so edits become visible immediately. Forge's root Gate remains
the repository-wide verifier; this route targets one module Gate.

## Contracts and compatibility

- IDs use `forge::core::validate_project_id` and must equal `.project.json`
  `id` exactly; no path strings, aliases or fuzzy search.
- `stack` and `profile` are non-empty strings used for discovery only.
- Existing `forge gate <PATH>` and `forge use <product>` parsing/behavior are
  unchanged.
- Default module verification uses the Gate `full` profile, matching
  `forge gate`; `--profile fast` is explicit and delegates to the current
  Fast behavior. Dry-run remains a rehearsal and cannot persist evidence.
- A source checkout without `modules/` (for example the standalone portable
  binary installation) returns `module-catalog-unavailable` with the resolved
  executable root and a next action to use a Forge source checkout. There is
  no implicit fallback to another location.
- Unknown IDs return `module-not-found`; malformed/duplicate declarations or
  path escapes return `module-catalog-invalid`; Gate outcomes retain current
  semantics and are not converted to facade errors.

## Failure and side-effect policy

| Condition | Behavior |
|---|---|
| Missing module root | Typed unavailable error; no CWD or sibling fallback |
| No declared modules | `list` succeeds with an empty result |
| Malformed `.project.json`, invalid ID, missing identity fields or Gate manifest | Refuse the whole catalog naming the module and field/path |
| Duplicate ID | Refuse before list or verification; name both module paths |
| Symlink/path escape | Refuse before reading or running checks |
| Unknown module ID | Typed not-found error; no path execution |
| Gate plan dry-run | Return rehearsal; do not execute or persist |
| Gate check failure/timeout/unavailable | Preserve existing Gate evidence and exit/status semantics |
| Remote CI, publish or deploy | Never invoked by this command |

Commands are never composed through a shell. The selected Gate manifest is
already an explicit module-owned executable declaration and the current Gate
engine retains its timeout/redaction/blocking behavior.

## Verification oracle

- Unit-test resolver behavior with temporary Forge roots: sorted IDs, ignored
  undeclared folders, invalid JSON/IDs/required fields, duplicate IDs, missing
  Gate declaration, symlink escape and no-root behavior.
- CLI integration-test `module list` human/JSON output, exact-ID resolution,
  unknown-ID refusal, dry-run no evidence write, full invocation to a temporary
  test Gate runtime, exit-code propagation and unchanged `forge gate`/`forge
  use` behavior.
- Verify Clap help and derived web command-catalog discoverability; new CLI
  operations remain classified CLI-only and are not exposed as arbitrary web
  command execution.
- Run `cargo fmt --all -- --check`, `cargo test --test
  module_workflow_contract`, relevant existing tests, `scripts/check-tests.sh`,
  strict OpenSpec validation, `git diff --check`, and root Gate dry-run/full.

## Decision ledger

- **Resolved:** derive module identities only from immediate Forge module
  declarations, not the external workspace registry.
- **Resolved:** route verification through the existing `cmd_gate` and do not
  create a second Gate runner or workflow engine.
- **Resolved:** installed binaries without module source assets fail clearly;
  no packaging expansion is included.
- **Resolved:** browser UI and remote CI/provider execution are outside this
  local maintainer CLI capability.
- **No unresolved decisions.**
