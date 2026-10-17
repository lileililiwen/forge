# Proposal: Make Forge the built-in developer gate

## Why

Concrete projects need a useful gate before they write project-specific checks. Today `forge gate` resolves an external gate executable, and a project without a gate manifest may receive no checks. Forge already vendors the gate engine and a repository-aware source-file-size scanner, so Forge can provide a baseline gate without requiring a separate project or sibling checkout.

## What Changes

- Make `forge gate` execute the gate engine bundled in Forge; remove external executable resolution from this command.
- Apply a versioned Forge baseline to every project, including projects with no gate manifest. The initial universal baseline requires the built-in source-file-size check with a 1,000 physical-line limit.
- Keep project-owned checks additive and explicit through the existing `gate.toml` / `.ai-gate/gate.yaml` check and command declarations.
- Preserve Forge's revision-bound evidence, status projections, JSON/human outputs, and exit behavior while identifying Forge as the gate owner.
- Update the authoritative brief and current gate documentation to describe Forge-owned execution. Do not implement CI or pre-commit template installation in this package.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-native-gate-checks` (this change) | Forge runs a mandatory built-in baseline plus explicitly registered project checks without an external gate runtime | Forge Rust workspace | `forge gate [TARGET]`, existing gate manifest format, Forge gate evidence contract | Existing Forge CLI, vendored gate engine | Clean project with no gate manifest runs the baseline; configured extension runs and contributes its result; no external gate binary is needed |
| `forge-ci-precommit-templates` (follow-up) | New projects receive Forge CI and pre-commit templates, and legacy projects can adopt them | Forge Rust workspace; generated consumer files | Versioned Forge templates invoke the `forge gate` contract | `forge-native-gate-checks` | Generated project and legacy fixture run the same baseline locally and in CI; ownership conflicts are refused |

This change has one gate-execution outcome. CI/pre-commit template distribution and generator/adoption behavior have a separate lifecycle and acceptance oracle, so they are deferred to the dependent follow-up. No second active OpenSpec package is created while this change is current.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge-vendored gate engine | `crates/driftwatchdog/src/gate/`, `crates/driftwatchdog/src/commands/gate.rs`, `crates/driftwatchdog/src/gate/source_size.rs` | Existing deterministic check planning, bounded project-command adapter, aggregation, built-in source-file scanner, and machine-readable results | Current Forge wrapper launches an external executable; absent manifests currently provide no universal baseline | Forge owns the compiled workspace and its CLI/package; the archived standalone tool is not a consumer prerequisite | adopt |
| Forge gate evidence and consumers | `src/gate/`, `src/doctor/`, `src/readiness/`, `src/release/`, `src/api/`, `frontend/` | Revision-bound evidence, shared projections, doctor/readiness/API/workbench readers | Current invocation records an external runtime identity and permits external binary resolution | Forge Core and its existing `forge gate` contract | keep local |
| Workspace Governance | Current gate declarations in `.ai-gate/gate.yaml`; sibling workspace checks are referenced by Forge's own policy commands | Optional workspace-level governance checks | Requires a separate checkout and is unsuitable as a universal Forge baseline | Workspace Governance owns fleet policy; Forge must remain independently usable | adapt through a generic adapter |

No sibling project is required to execute the built-in baseline. Project-specific checks remain local declarations. Workspace Governance checks remain opt-in extensions and are not silently added to every project.

## User Experience and Interface Impact

The actors are developers running `forge gate` and maintainers reviewing its CI or hook result. The entry point remains `forge gate [TARGET]`; `--dry-run` shows the resolved baseline and registered extensions, and human/JSON output identifies each check, outcome, and remediation. A clean project with no gate configuration now runs the baseline rather than reporting that there is nothing to gate. A failing baseline or extension exits non-zero; malformed configuration and incomplete required coverage are reported as blocked/unavailable and never as a pass.

This is a terminal and automation workflow, with no browser UI. Output remains readable without color, diagnostics go to stderr where appropriate, JSON stdout remains machine-pure, and every result names the check and affected relative path. Responsive layout and browser accessibility do not apply; terminal output must not require color or interactive input.

## BFS Impact Map

- **Capability/actor/flow:** Forge gate execution; developer and CI callers; `forge gate`, `forge gate status`, dry-run, and existing hook/CI consumers in the dependent package.
- **Contracts/data:** Preserve `GATE_CONTRACT_VERSION` and readable historical evidence. New evidence identifies runtime `forge`; baseline check provenance is explicit. Include the Forge baseline version in the effective gate identity/digest.
- **Configuration:** Keep `gate.toml` and `.ai-gate/gate.yaml` as optional extension declarations. Their checks add to the non-removable baseline. `verification.gate_runtime` and `FORGE_GATE_BIN` no longer select the implementation of `forge gate`.
- **Integrations/callers:** Forge CLI, doctor, provider matrix, readiness, release preflight, admin API, workbench, packaging, CI, and tests. All readers continue using the existing shared evidence projection.
- **Persistence/failures:** Forge `.forge/gate` evidence and operation journal remain authoritative. Dry-run writes neither. Invalid configuration fails before execution or evidence writes. Baseline scanning is read-only, bounded, repository-confined, and reports incomplete enumeration as review-required.
- **Security/compatibility:** No sibling binary resolution, network install, or implicit project mutation. Existing check commands remain explicit project-owned extensions and use the existing bounded/redacted execution adapter. Historical evidence remains readable and is not rewritten.
- **Verification:** Unit and integration tests for default baseline, extension union, runtime independence, dry-run side effects, blocked exit, and old evidence compatibility; package smoke test; strict OpenSpec; local Forge Gate rehearsal and run.

## Capabilities

- `forge-native-gate`: Forge executes its own versioned baseline and explicit project extensions, persists revision-bound evidence, and projects it consistently.

Traceability: `forge-native-gate` maps to authoritative `requirement.md` sections 23, 24, and 47; `gate-runtime-evidence` maps to the existing canonical gate evidence capability.

## Non-goals

- CI and pre-commit template generation, project scaffolding changes, and legacy adoption commands (the dependent follow-up package).
- Making project-specific scripts mandatory or automatically discovering executable files.
- Making Workspace Governance, network access, or any external runtime a prerequisite for the built-in baseline.
- Replacing the existing evidence contract or adding an MCP/API gate-write surface.
- Publishing, pushing, or deploying Forge.
