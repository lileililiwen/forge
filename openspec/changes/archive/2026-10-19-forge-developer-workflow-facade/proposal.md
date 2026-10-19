# Proposal: Forge developer workflow facade

## Why

Forge now contains independently runnable Rust, .NET and Python governance
modules. Their source trees and native verification commands are intentionally
different, but contributors should not need to remember internal module paths
or call a different orchestrator for each one. `forge gate <PATH>` already
owns execution, evidence and exit semantics; the remaining gap is deterministic
module discovery and an ID-based route into that existing Gate. Forge must
remain the developer control plane and must not become a generic hosted CI/CD
replacement.

## What Changes

- Add `forge module list` to discover immediate Forge `modules/*` entries from
  their `.project.json` declarations and valid Gate manifests.
- Add `forge module verify <ID>` to resolve one declared module and delegate
  once to the existing `forge gate` execution path, preserving Gate profiles,
  dry-run, timeout, evidence and exit behavior.
- Refuse duplicate or malformed module identities, path escapes, missing
  module data and unknown IDs with actionable errors. Never fall back to the
  caller's directory or a sibling checkout.
- Document the local module workflow and its explicit boundary from hosted CI,
  package publication, deployment and `forge use` product-binary routing.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-developer-workflow-facade` | Contributors discover in-tree modules and verify one by stable project ID using Forge Gate | Forge Rust CLI | `forge module list|verify`; `.project.json`; existing `forge gate` contract | Consolidated modules and Gate runtime, both complete | CLI integration tests prove sorted discovery, path confinement, dry-run/run delegation, evidence and failures |

The request concerns one actor, one local command surface and the existing Gate
state model; discovery and verification are an inseparable workflow. The
browser portal is not expanded: this is a local maintainer CLI operation, and
the CLI itself is the interaction surface. Existing web command-catalog
discovery remains automatic from Clap help and must continue to include the new
command accurately. No project registry database, new workflow engine, CI
provider or publication lifecycle is introduced.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge Gate | `src/cli/functions_13.rs::cmd_gate`, `src/gate/`, `forge gate <PATH>` | Existing declaration resolution, Fast/Full profiles, bounded execution, revision-bound evidence, persistence and blocking exit codes | Requires a path rather than a stable imported-module ID | Forge owns the public CLI and Gate semantics | **extend shared owner** with a thin ID resolver that delegates to this entry point |
| Consolidated modules | `modules/*/.project.json`, `.ai-gate/gate.yaml`, `gate.toml` | Each module owns native commands, runtime declaration, tests and package policy | Different languages and directories make one hard-coded command recipe incorrect | Each module owns native checks; Forge owns routing and Gate | **adopt** declarations and Gate manifests without rewriting native checks |
| Workspace Governance | `modules/workspace-governance/projects.json` | Workspace project inventory and external portfolio reconciliation | The module verifier needs only Forge's immediate module boundary; loading the wider workspace registry would couple Forge internals to external workspace state | Forge metadata and module declarations are local; Governance retains portfolio registry ownership | **keep local** to `modules/*/.project.json`; do not read workspace registry |
| Existing `forge use` | `src/cli/use_cmd.rs`, `openspec/specs/cli-facade/spec.md` | Typed binary catalog and argument-preserving product delegation | It resolves executables, not module roots or Gate evidence | Forge CLI facade | **keep separate**; product binary routing is not module verification |
| Hosted CI and release providers | `.github/workflows/`, module publisher workflows, CI/CD system model | Existing hosted checks and owner-gated publication | A local verifier must not emulate hosted permissions, schedules, secrets or remote writes | GitHub Actions and each package registry own remote release execution | **keep separate**; local verification invokes Gate only |

## User Experience and Interface Impact

The actor is a Forge maintainer working in the Forge checkout. Entry is
`forge module list`, followed by `forge module verify <ID>`; `--dry-run`,
`--profile fast|full` and `--timeout-secs` expose the existing Gate controls.
The list is sorted by ID and shows module ID, stack/profile and repository-
relative path. Verification prints the existing Gate evidence and returns its
exit code. Empty discovery gives a successful empty list; malformed metadata,
duplicate IDs, unknown IDs, missing local module data and failed checks give
typed diagnostics with the exact module or path. This is a terminal workflow,
so responsive web layout, browser focus management and device testing do not
apply; command help, stable text/JSON output, stderr errors and CLI tests are
the interaction and accessibility oracles. No browser route or visual state
is added.

## BFS Impact Map

| Surface | Current evidence | Change and verification |
|---|---|---|
| CLI behavior and discovery | `src/main.rs` Clap commands; `forge gate PATH`; `forge use` separate binary facade | Add `module list|verify`; test help, sorted entries, JSON, aliases not applicable, explicit errors and exit propagation |
| Module declarations and paths | Three current direct modules: Workspace Governance, .NET platform libraries, Rust platform libraries; `.project.json` plus `.ai-gate/gate.yaml` or `gate.toml` | Enumerate only direct children of Forge `modules/`; canonicalize paths and reject escapes/symlinks outside the module root; verify duplicate and malformed IDs |
| Gate and evidence | `cmd_gate` resolves target path, Gate manifests select native checks and runtime | Reuse the existing handler exactly once; verify dry-run creates no evidence and full verify produces module-scoped Gate evidence |
| Persistence | SQLite project registry and per-module `.forge/gate` evidence already exist | No new database tables, registry writes or evidence schema; reuse existing Gate persistence |
| Web UI / MCP / API | Web command catalog derives CLI inventory; MCP/API do not expose arbitrary command execution | Preserve generated command visibility and CLI-only classification; add no execution endpoint or browser UI |
| CI/CD and releases | Root CI, module quality workflows and owner-gated package publishers already exist | Local module verification is not a hosted workflow; do not trigger, edit or impersonate remote CI/release operations |
| Security and portability | Gate bounds execution and redacts evidence; modules are committed in the Forge repository | No shell interpolation, npx, network fallback, sibling path or arbitrary path input; installed binary without local module assets fails explicitly |
| Tests and compatibility | Gate contract tests, CLI parser, installed binary packaging and module checks | Add focused module-discovery and command tests; run root tests, Gate and packaged smoke boundary as applicable; keep existing `forge gate` and `forge use` unchanged |

## Capabilities

- `forge-developer-workflow-facade`: discover and run local verification for
  Forge-owned modules by stable project ID through Forge Gate.

## Non-goals

- Replacing GitHub Actions, CI services, hosted runners, workflow files or
  package publication.
- Adding generic shell-command execution, arbitrary external-project scanning,
  sibling checkout discovery, npx, a module plugin framework or a new registry.
- Changing module-owned checks, profiles, Gate policies, package versions or
  release approvals.
- Replacing `forge use`, `forge gate <PATH>`, `forge run`, or the browser
  command-execution security model.
