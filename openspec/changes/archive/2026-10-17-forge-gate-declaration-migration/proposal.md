# Proposal: Make Forge Gate evidence declaration-aware without owning policy

## Why

Driftwatchdog already owns Gate planning, execution, row verdicts, blocking, aggregate status, categorized outcomes, and process exit semantics. Forge currently discards part of that structured result: it merges `REVIEW_REQUIRED` and `UNBOUND`, drops row `source` and `severity`, retains only a normalized aggregate, and does not retain the runtime's raw status, blocked bit, categorized lists, or exact exit code. Its read surfaces then reconstruct overlapping answers independently.

Projects also declare Gate facts in more than one owned location. `forge.yaml`, workspace-governance's `.project.json`, and the Driftwatchdog-selected Gate manifest can declare the expected rule pack, while a Forge-owned verification floor can state where each implementation is expected. Forge does not yet consume that information. This leaves users unable to distinguish a native check, a bound command, an exemption, an unbound check, an undeclared implementation, or a declared file that is absent on the current host.

The migration must not turn missing declarations into retroactive failures. In particular, an undeclared pack expectation is a named gap but does not stale revision-matched evidence. Driftwatchdog's current row vocabulary is exactly `PASS`, `FAIL`, `REVIEW_REQUIRED`, `NOT_APPLICABLE`, and `UNBOUND`; `SKIP` exists only as compatibility input for old Forge evidence and is not a current Driftwatchdog word.

## What Changes

- Persist Driftwatchdog's five current row states one-to-one, each row's structured runtime `source` and `severity`, the existing bounded note, the runtime's raw top-level status and blocked bit, its exact process exit code, and its `failures`, `pending_reviews`, `not_applicable`, and `unbound` lists. `GateAggregate` remains only a normalized compatibility projection; `forge gate` mirrors the captured runtime exit code.
- Add an optional schema-1 `forge.yaml verification` block with optional `rule_pack` and required `checks`. Authored implementation sources are only `native`, `bound`, `pack-default`, and `exempt`; `absent`, `unbound`, and `undeclared` are observations, while old evidence renders a missing implementation source as `unrecorded`.
- Preflight all current Gate declarations through one Core context before either a run or a read: absent `forge.yaml` is valid, but a present invalid/null verification block, malformed `.project.json` or selected Gate manifest, or an applicable malformed governance selection returns its typed error before runtime spawn or absent-evidence short-circuit. Invalid input leaves the invocation marker, prior evidence, journal, and sibling history unchanged.
- Attribute each runtime row deterministically: an exact floor entry wins; otherwise `UNBOUND` becomes `unbound`, `NOT_APPLICABLE` becomes `exempt`, exact runtime source `adapter:project-runtime` becomes `bound`, exact runtime source `driftwatchdog` becomes `native`, and every other structured source becomes `undeclared`. Forge never reads diagnostics, findings, commands, remediation, or notes to decide attribution and never infers `pack-default` without a floor entry.
- Resolve pack expectation in this order: `forge.yaml verification.rule_pack`, `.project.json verification.rule_pack`, then the Driftwatchdog-selected `gate.toml`, `.driftwatch/gate.toml`, or `.ai-gate/gate.yaml`, then undeclared. Carry both identity and provenance. A declared mismatch or missing observed identity is stale; undeclared expectation leaves the existing revision/aggregate freshness result unchanged and adds the exact reason `pack expectation undeclared`.
- Build one shared read projection for CLI JSON/human output, doctor, the checker document, release preflight, the admin API, and the workbench. The checker carries per-row data in an additive top-level Gate projection rather than creating one alert per passing row. The workbench adds text to its existing rows and adds no control.
- Preserve old evidence without rewrite. A missing implementation source projects as `unrecorded`; a serialized old `skip` state remains readable, but parsing a current runtime row containing `SKIP` follows the unknown-state path and can never produce new `skip` evidence.
- Extend the existing `platform.gate-result/1.0.0` projection with total mappings for the new row states while preserving every old field and meaning: `ReviewRequired`/`Unresolved` remain review-required, `Unbound` remains unbound, and blocked aggregate precedence never invents a failed root without a failed row. Additive owning-capability fields are compatible; byte identity is required only for unaffected surfaces.
- `forge new` emits exactly a multiline `verification:\n  checks: []` block. Accepted import still writes its existing minimal `forge.yaml`, but omits `verification`.
- Add Forge's schema-1 root `forge.yaml` and declare the nine-check floor, including the governance-quality path relative to a stored enabled `workspace-governance` provider selection's `workspace_root`. A missing selection, root, or file affects only that row's attribution and never changes the runtime verdict.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-gate-declaration-migration` | Forge preflights current declarations, preserves Driftwatchdog's structured result, adds declaration-based attribution, and projects one read model everywhere | Forge; Rust 2021 Core/CLI/API plus existing vanilla JavaScript workbench | Existing `0.1.0` Gate evidence and `platform.gate-result/1.0.0` contracts with additive owning-capability fields; schema-1 `forge.yaml`; additive checker/API fields | Existing Driftwatchdog status document and workspace-governance declarations/provider selection | Invalid declarations cause zero invocation/write; blocked/passing fixtures retain exact runtime facts/exit; declaration/pack/contract matrices and all read surfaces match the specified projection |

The runtime facts, declaration attribution, evidence migration, and shared projection are one lifecycle: splitting them would create either un-attributed stored records or independently reclassified surfaces. The browser text is included because it is already a human-facing consumer of this evidence. New Driftwatchdog checks, workspace-governance schema changes, project registration, and provider selection are independent outcomes and remain outside this package.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Driftwatchdog | `/home/paul/code/driftwatchdog/src/gate/types.rs::GateStatus`, `GateResult`; `src/commands/gate.rs::print_json` | Authoritative five row statuses, row source/severity, aggregate, blocked bit, categorized lists, and exit semantics | Forge currently drops or normalizes these facts and has legacy `SKIP` compatibility | Driftwatchdog owns policy/runtime releases; Forge consumes its document | **adopt** — consume the structured result exactly and do not duplicate policy |
| workspace-governance | `.project.json verification`; Forge `src/governance/model.rs::GovernanceProviderConfig` | Declared runtime/rule-pack input and stored `workspace_root` on an enabled provider selection | Forge lacks a verification floor and currently does not use the stored root for attribution | workspace-governance owns declarations/provider implementation; Forge owns its manifest and evidence projection | **adapt through a generic adapter** — read declarations and the stored provider selection; never write the sibling |
| cargo tooling projects | `/home/paul/code/cargo-audit-boost`, `cargo-matrix-boost`, `cargo-sonar` `.project.json` plus Gate manifest | Real direct-Gate shape without `forge.yaml` | Registry/doctor/workbench flows require broader project management and are not needed for direct Gate attribution | Each tooling repository owns its files and release | **keep local** — add a no-manifest Forge fixture for direct Gate only; do not change those repositories |
| Forge Gate consumers | `src/gate`, `src/doctor`, `src/checker`, `src/release`, `src/api/admin/gate.rs`, `frontend/app.js` | Existing bounded invocation, evidence persistence, and human/API surfaces | Freshness and row rendering are reconstructed in several callers | Forge owns the additive read projection and releases it with this change | **keep local** — one Forge Core projection removes duplication without creating a cross-project library |

No shared extraction is warranted. The repeated policy vocabulary already has a sibling owner, and the desired declaration remains project-local schema-1 Forge configuration. Dependency direction stays Driftwatchdog/workspace-governance → Forge consumer; no sibling repository is written.

## User Experience and Interface Impact

Actors are maintainers reading or running Gate and operators inspecting the existing workbench Health card. Entry points remain `forge gate`, `forge gate status`, `forge doctor`, `forge check`, release preflight, and the existing workbench Gate card. Each row names runtime state, runtime source, severity, implementation source, and the optional stable implementation path. Freshness text names pack expectation identity/provenance or the undeclared gap. Initial/empty behavior stays `unverified`; malformed declarations or evidence remain explicit typed errors; runtime-unavailable behavior preserves prior evidence; absent implementation is row attribution and does not rewrite the runtime verdict.

The workbench changes text only: no route, navigation item, button, confirmation, or destructive action is added. Existing semantic list markup remains; text wraps at 390 CSS pixels, has no color-only meaning, adds no focus stop, does not alter focus restoration, and introduces no motion. Keyboard and reduced-motion behavior are therefore unchanged. Browser contract coverage at desktop and 390-pixel viewport proves the added text remains visible without horizontal overflow and the existing re-run control remains the only control.

## BFS Impact Map

| Area | Impact |
|---|---|
| Capability/state | New `gate-declaration-migration` delta; five current runtime states remain distinct; legacy `skip` is decode-only |
| Desired configuration | Optional schema-1 `verification` block; closed authored source/root words, unique check ids, paired root/path fields, confined relative paths, and no unknown fields |
| Observed persistence | Additive optional `GateEvidence` and `GateCheck` fields; contract remains `0.1.0`; old records are read without rewrite |
| Runtime boundary | One declaration preflight before any read/spawn; then at most one bounded Driftwatchdog invocation; capture raw status, blocked bit, exact exit, category lists and row source/severity; no policy or exit recomputation |
| Attribution/filesystem | Exact floor-id lookup first; at most the declared root/path is resolved; project root or stored enabled workspace-governance root only; no environment or parent probing |
| Pack declaration/freshness | Ordered read across Forge manifest, workspace declaration, selected Gate manifest; declaration mismatch adds stale reason; undeclared adds a gap only |
| Shared callers | CLI run/status JSON and human, doctor, checker document, release preflight, admin status/run API, and frontend Gate rows consume one Core projection |
| Generation/import | New projects get an empty floor; accepted imports retain their existing minimal manifest and omit verification |
| Repository configuration | Root schema-1 `forge.yaml` declares the nine effective checks and `workspace-governance@1` expectation |
| No-manifest boundary | Direct Gate path resolution remains supported with `.project.json` and a Gate manifest; no registration, doctor, or workbench expansion |
| Failures/security | `manifest-invalid`, `gate-invalid`, and conditionally `governance-invalid` are chosen before spawn/read short-circuit; invalid declarations change no invocation marker, evidence, journal or sibling history; relative paths are confined and host paths use placeholders |
| Tests | Red-first unit, preflight side-effect, fixture, platform-contract, cross-surface, browser, generation/import, release, compatibility, and no-write coverage; final breadth review covers every caller and all three deltas |
| Compatibility | Gate evidence `0.1.0`, manifest schema `1`, and platform Gate contract `1.0.0` stay versioned; old fields/meanings and consumers remain valid while owning capabilities add fields; unaffected surfaces remain byte-identical |
| Source requirements | `requirement.md` §24 (DriftWatch remains policy engine), §25/§29 (evidence-backed maturity/release), §32 (existing systems retain identity), §34 (CLI), and §38 (Core/thin transports) |

## Capabilities

- `gate-declaration-migration` (new): Forge preserves the Gate runtime's structured result, adds deterministic implementation and pack attribution from current declarations, and exposes one compatible read projection across all existing consumers.

## Non-goals

- No change to Driftwatchdog policy, check selection, execution, blocking, aggregate, status vocabulary, categorized lists, or exit semantics.
- No edit to driftwatchdog, workspace-governance, cargo-audit-boost, cargo-matrix-boost, cargo-sonar, or any other sibling repository.
- No new Gate check, `pack-default` inference, command parsing, diagnostic/findings/note inspection, parent-directory search, environment-root fallback, or network lookup.
- No manifest schema or Gate evidence contract version bump, no rewrite/backfill of old evidence, and no new compatibility alias or deprecated path.
- No broadening of no-manifest projects into registration, doctor, release, API, or workbench management; the fixture covers direct Gate run/status only.
- No MCP Gate tool, new browser control, navigation change, policy alert per passing row, provider-selection mutation, or modification of Driftwatchdog's Gate manifest commands.
