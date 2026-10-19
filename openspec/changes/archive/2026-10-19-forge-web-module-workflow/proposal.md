# Proposal: Authenticated web workflow for Forge modules

## Why

Forge now offers `forge module list` and `forge module verify <id>` for its
consolidated in-tree projects, but the new workflow is CLI-only. The Forge
admin portal already provides a browser entry point for Gate status and runs.
Operators should be able to discover and verify Forge-owned modules there
without learning a separate command path or exposing source paths to the
browser.

## What Changes

- Add an authenticated Modules page to the standalone Forge web UI.
- Expose declared module metadata and latest Gate evidence through typed,
  session-gated `/v1/admin/modules` routes.
- Allow a user to run the existing Gate engine for one exact module ID with
  the `fast` or `full` profile, then render the actual evidence and freshness.
- Move module discovery and path validation into a shared Forge library
  capability used by CLI and API.
- Map the module CLI verbs to their new web surface in the command catalog.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-module-workflow` | Let an authenticated Forge operator discover declared in-tree modules and run their existing Gate checks in the browser | Forge Rust Core/API and standalone HTML/CSS/JavaScript frontend | Shared module catalog plus versioned `/v1/admin/modules` read/run routes and a Modules page | `forge-developer-workflow-facade`, `forge-native-gate`, `forge-web-command-catalog` | API contracts prove bounded IDs, auth and Gate evidence; browser contract proves list, run, states, keyboard and responsive layout |

The inventory, status and run flow share one module identity and the existing
Gate evidence lifecycle. Splitting API and UI would leave this human-facing
capability unusable or create an intermediate browser with no action. No new
persistence schema, external service, sibling release, or project-specific
web app is required.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge module CLI | `src/cli/module.rs`, `forge module list`, `forge module verify` | Exact in-tree discovery rules, identity validation and path containment | Discovery currently lives in the binary and is not callable by the API library | Forge source checkout; CLI and API remain transports over the same library | **extend shared owner** by moving discovery to a Forge library module |
| Forge Gate API | `src/api/admin/gate.rs`, `crate::gate::run_gate`, `save_evidence`, `evidence_freshness` | Existing profile parser, actual Gate outcome mapping, evidence and freshness behavior | Existing endpoints require a registered project ID and cannot resolve a module declaration | Forge Core owns Gate; module endpoint resolves only a module ID | **adopt** the existing Gate engine and evidence contract |
| Forge web command catalog | `src/api/command_catalog/`, `frontend/app.js` command reference | Typed CLI metadata, web availability states, authenticated JSON reads | Module commands are still cataloged as CLI-only | Forge portal owns command discovery and navigation | **extend shared owner** with the module routes and web dispositions |
| Platform and manager projects | `modules/dotnet-platform-libs/`, `modules/rust-platform-libs/`, `modules/workspace-governance/` | Independently runnable modules declare native Gate checks | They do not own Forge's admin sessions, API, or browser shell | Each module owns its checks; Forge owns the operator experience | **keep local** because UI/session/runtime reuse belongs to Forge and coupling a language module to the portal reverses ownership |

No `common` or separate manager project owns the Forge admin UI or module
discovery contract. This change does not modify sibling modules; they remain
independently runnable and keep their own Gate declarations.

## User Experience and Interface Impact

The actor is an authenticated Forge administrator working from the Forge
source checkout. The new **Modules** navigation entry opens a page listing
declared module IDs, stack/profile, Forge-relative directory, and the latest
Gate result/freshness. Each row can open a focused detail panel with `Fast`
and `Full` profiles and an explicit `Run Gate` button. A run disables duplicate
submissions for that module and renders the returned check rows, revision,
timestamp, aggregate and recovery message. The command reference links these
verbs to Modules.

The page supports initial loading, empty inventory, unavailable source,
unauthorized session, module-specific status failure, running, Gate PASS,
Gate FAIL/REVIEW_REQUIRED and recoverable request failure. A Gate FAIL is a
valid recorded result, not an API failure. Errors and rows expose relative
module paths only; absolute source paths and raw server diagnostics stay
server-side. At narrow widths rows become stacked cards; all actions work by
keyboard, use visible focus, labelled controls, live result announcements and
status text that does not rely on color. Verify with the existing Chromium
browser harness at desktop and narrow viewport widths, including keyboard and
accessibility assertions.

## BFS Impact Map

| Surface | Impact and evidence target |
|---|---|
| Capability and users | In-tree module discovery and authenticated operator Gate workflow; one admin user journey from Modules list to evidence |
| Core contract and callers | Extract shared module catalog from `src/cli/module.rs`; keep CLI JSON/human output stable; API reads catalog and resolves server-side IDs |
| API and auth | Add GET list, GET module Gate status, POST Gate run under the existing global admin guard; browser sends IDs/profile only, never a path or command |
| Data and persistence | Reuse module-local Gate evidence and the existing Forge registry operation journal; no new schema or external service |
| Integrations | Reuse `crate::gate` engine, Gate profile, evidence and freshness; no sibling checkout, provider, hosted CI or deployment integration |
| UI and navigation | Add Modules destination, loading/empty/error/result states, command-catalog route descriptions and responsive layouts |
| Failure and security | Reject malformed/unknown IDs, bad profiles, unauthenticated requests, source absence, duplicate active module runs and Gate runtime failures; enforce canonical containment and scrub absolute paths |
| Compatibility | Preserve existing `forge module list` and `forge module verify` behavior and command catalog exhaustiveness |
| Verification | Unit/path tests, API auth/contract tests, command catalog parity, browser list/run/state/a11y checks, name preflight, strict OpenSpec and local Forge Gate |

## Capabilities

- `forge-web-module-workflow`: authenticated discovery, Gate status and
  constrained verification of Forge-owned modules in the admin web portal.

## Non-goals

- Discovering modules outside the running Forge source checkout.
- Browser-supplied file paths, command strings, binaries, arguments, shell
  execution, arbitrary Gate declarations or arbitrary timeout values.
- Replacing module-native CI, editing module Gate declarations, creating
  projects, publishing packages, deploying projects, or changing sibling
  release workflows.
- Adding a new Gate engine, general remote job runner, persistent job table,
  or generic CLI-over-HTTP capability.
