# Proposal: Make Forge Gate cover and enforce Workspace Governance source files

## Why

Forge's mandatory source-size Gate currently scans conventional source roots but does not recognize `scripts/`. Running Forge Gate against the maintained Python governance module therefore reports a successful scan with no candidates, while four maintained Python files under `scripts/` exceed Forge's 1,000-line baseline. This creates a false assurance in the consolidated tree.

## What Changes

- Configure the in-tree Workspace Governance native Gate manifest to include its maintained `scripts/**/*.py` files in the existing source-size scanner.
- Split the four oversized governance scripts into import-compatible modules while keeping their CLI entry points and observable outputs stable.
- Include the new product-quality helper modules in Forge's packaged and installed governance resources so installed concern checks keep working.
- Document the Forge Gate manifest precedence and the command-alignment rule for module maintainers.
- Exercise the Python module Gate against the real source tree and prove both detection and the post-split passing result.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-workspace-governance-gate-conformance` | The in-tree Workspace Governance Gate evaluates maintained Python files under `scripts/`, and the four files meet the existing 1,000-line limit | Forge Gate, Rust 2021; in-tree Workspace Governance, Python 3.11 | Existing `source-file-size` concern configured through the module's native `gate.toml` | Archived `forge-governance-consumer-migration` and existing Driftwatchdog scanner | Forge Gate identifies real candidates and passes after extraction; Python suite and governance self-check preserve behavior |

The broader roadmap item `forge-governance-consumer-portability` spans separate sibling owners and release lifecycles. It remains queued for consumer-specific changes; this package does not edit external sibling repositories. Splitting the scanner coverage fix from Python module decomposition would leave the false-success condition unresolved or leave an unverified code split, so they share one acceptance boundary. The existing Forge archive and installer must carry the two quality helper modules because installed `forge concern check` executes this checker. No persistence, public wire contract, or schema changes are included.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Vendored Driftwatchdog Gate | `crates/driftwatchdog/src/gate/source_size.rs`; `SourceSizePolicy`, `select_candidates`, `scan_with_baseline` | Existing Git-aware, symlink-safe scanner, configured include/exclude behavior, 1,000-line default and normalized results | The module's current business-YAML Gate has no source-size include override | Forge owns the vendored implementation; project-owned `gate.toml` supplies supported include configuration | **adopt** |
| Workspace Governance module | `modules/workspace-governance/scripts/{workspace_check,bind_gate_commands,product_code_quality,ui_conformance}.py`; `.project.json` | Python 3.11 scripts, unittest suite, existing root CLI wrappers | Four maintained scripts exceed 1,000 lines; tests establish import and output compatibility | Forge owns the in-tree module; its Python files remain independently runnable | **keep local** |
| External sibling consumers | `/home/paul/code/openpanel/scripts/check-governance-quality.sh`; sibling `HANDOFF.md` files | Existing consumers and their own Gate ownership | Independent specs and local untracked files exist; no authorized consumer edits are needed for this Forge package | Each sibling repository owns its own main-branch change and release | **adapt through a generic adapter** |

The scanner is shared within Forge and already supports project-owned includes; the in-tree module configures that existing mechanism instead of changing Forge-wide discovery. Python splitting remains local because these are four module-owned files with no shared Python-library consumer boundary in the inspected tree. External consumer migrations remain separately scoped.

## User Experience and Interface Impact

Actors are Workspace Governance maintainers running the existing `forge gate modules/workspace-governance` command. The command, Quality/Gate result surfaces, and CLI remain unchanged. A previously empty candidate set becomes measured; an over-limit file appears as a bounded `source-file-size` finding with its repository-relative path and line count, and a compliant tree reports evaluated candidate totals. The primary flow is local or CI Gate → inspect failing path → split behavior-preservingly → rerun. Error and review-required behavior for unreadable or unresolvable source boundaries remains unchanged. This is a terminal interface: output remains plain text/JSON, non-color-dependent, readable in narrow terminals and screen readers, and actionable without interaction. No browser/device surface applies.

## BFS Impact Map

- **Capabilities and callers:** `forge gate`'s mandatory source-size concern; only the in-tree module receives an explicit `scripts/**/*.py` include. Other projects and conventional roots are unchanged.
- **Contracts and persistence:** no result schema, manifest, database, or evidence format changes. Existing Gate evidence records the newly evaluated candidate set.
- **Integration/configuration/distribution:** add the module's native `gate.toml`, preserving all currently selected checks and commands while setting the existing source-size include. Update Forge's existing archive and installer to carry the two adjacent quality helper modules used by the installed concern checker. No scanner change, language parser, new dependency, or external sibling checkout is added.
- **Failure boundaries:** current Git boundary, file-read review behavior, exclusion policy, newline counting, finding bounds and exit status remain intact. Empty `scripts/` trees still pass with measured zero candidates only when there are genuinely no candidates.
- **Tests:** scanner unit/integration tests prove scripts discovery, oversized failure, exclusion handling and the module's candidate count. Python tests and self-check protect refactored behavior.
- **Compatibility/security/privacy:** keep Python import shims and command-line invocation stable; the scanner continues to read only repository-owned files, skip symlinks/binary files, and execute no discovered scripts.
- **Workspace Governance sibling assessment:** the in-tree module owns its behavior and package. The vendored Driftwatchdog implementation is the Forge Gate owner. External sibling consumers have distinct repo workflows and remain outside this package.
- **Unaffected:** Forge-wide source-root discovery, UI, API, MCP, CI provider, release publication, registry schema and remote state.

## Capabilities

- `source-file-size-gate` — the Workspace Governance module's native Gate manifest configures maintained Python source under `scripts/` for Forge's mandatory source-size scan.
- `forge-workspace-governance-gate-conformance` — the in-tree Python module is below the existing per-file limit and its Gate verifies real candidates.

## Non-goals

- Migrating external sibling projects away from Workspace Governance commands.
- Changing the 1,000-line limit, global conventional source roots, or zero-candidate semantics.
- Rewriting Python behavior, changing CLI output or requiring Forge at runtime for standalone module commands.
- Publishing a Forge release or modifying CI providers.
