# Design: Forge runtime template bundle

## Implementation boundary

Forge repository only. Add the command variant and dispatch in `src/main.rs`, resource discovery in `src/cli/runtime_template.rs`, and the migrated renderer and assets under `scripts/governance/runtime-templates/`. Extend `scripts/package.sh` and `scripts/install.sh` to carry assets beside the binary. Do not alter sibling files or bootstrap/audit implementations.

## Language and runtime

The public CLI and resource resolution use the existing Rust 2021 binary and stable toolchain declared by `Cargo.toml`. The renderer remains Python 3 with standard-library dependencies only. Invoke `python3 <absolute bundled script> <args...>` through `std::process::Command`; never assemble a shell command. Build and test with `cargo build --workspace`, focused Rust integration tests, `python3 -m py_compile`, and `sh -n` for changed packaging scripts. Linux release package behavior is the local archive/install target; path resolution is platform-neutral.

## Ownership and shared code

Forge owns the copied renderer and runtime template assets as ordinary source files. `manifest.json` pins every bundled file by relative path and SHA-256; `runtime_templates.py` validates the complete file set and hashes before serving a command. The existing Workspace Governance implementation is read-only migration evidence; no Git submodule or runtime checkout dependency is introduced. Preserve attribution/provenance in a `PROVENANCE.md` naming the source revision and files. The future bootstrap and workspace audit packages remain separate and may not redefine this renderer contract.

## User experience and interface

Exact command hierarchy: `forge runtime-template <list|inspect|render|check> [renderer arguments]`. Clap accepts the operation and forwards remaining arguments and flags unchanged. The command inherits stdin/stdout/stderr and returns the renderer exit status. Missing resources or Python produce a concise Forge error. JSON formatting remains the renderer's own stable JSON output. CLI only; browser/device UX is N/A because no such surface exists in the affected workflow.

## Behavioral model

Resource root resolution order: explicit `FORGE_RUNTIME_TEMPLATES_HOME` (must be an existing directory containing `runtime_templates.py`, `templates/runtime/registry.json`, and `manifest.json`), then the Forge source checkout's `scripts/governance/runtime-templates` path in development, then `<current_exe_parent>/../share/forge/runtime-templates` in an installed layout. A configured but invalid override is a hard error and does not fall through. No CWD-parent, sibling, or network search. Read operations are side-effect free. Render delegates unchanged to the migrated implementation; its existing preflight and atomic file-write rules remain authoritative.

The release layout is `bin/forge` and `share/forge/runtime-templates/{runtime_templates.py,templates/runtime/...}`. `scripts/install.sh` copies the binary and the resource tree under the supplied prefix. Archive creation includes that tree. Source development runs resolve from `CARGO_MANIFEST_DIR`.

## Contract and compatibility

The existing renderer CLI, JSON registry schema version 1, IDs, variables, template file paths, `.project.json` keys and refusal codes remain unchanged. `FORGE_RUNTIME_TEMPLATES_HOME` is optional and points at the renderer bundle root. Forge does not parse/rewrite renderer JSON; it forwards bytes and exit code. Missing Python/resource is typed unavailable and exits nonzero.

## Failure and boundary policy

Invalid explicit override, absent resource files, unavailable Python, or child spawn failure returns nonzero with a bounded diagnostic. Child exit code is preserved; signal termination maps to exit 1. Template validation, conflict, secrets, path traversal, declaration and Compose errors remain the renderer's existing outcomes. The bundle's registry and every declared file are validated before package success. Installation verifies the archive checksum before writes and installs only into the explicit prefix.

## Verification oracle

Tests assert all registry entries/assets resolve, list/inspect JSON match the v1 contract, render and check preserve metadata/refusal semantics, override failures do not fall through, no sibling path is consulted, source and installed resource layouts dispatch, and the package archive contains all required files. Run the focused tests, Python compilation, package/install/smoke scripts, full workspace checks, OpenSpec strict validation and Forge Gate before archive.

## Decision ledger

- **Resolved:** retain Python renderer rather than rewrite it; it is a mature, standard-library-only 354-line contract with an existing 347-line test suite and stable version-1 behavior.
- **Resolved:** bundle Python source and data with Forge binary; the renderer's Python prerequisite is explicit and the existing Forge Gate already requires Python for governance checks.
- **Deferred:** project bootstrap and fleet audit/provider ownership are separate packages because they change different contracts and state stores.
- **No blocker:** the sibling repository remains unchanged and the original implementation is available for read-only compatibility comparison.
