# Proposal: make project quality evidence the primary portal view

## Why

The project workbench already reads Forge Gate evidence, but hides it inside
the Health card. Its initial read also passes the wrapped API response to a
renderer expecting the inner envelope, so a recorded result can appear as
unverified. The Projects assurance browser exposes provider reads among
ordinary records. Operators need a clear quality view, while provider details
belong behind an explicit advanced disclosure.

## What Changes

- Present the workbench Gate surface as a dedicated **Quality checks** card
  with aggregate status, freshness, revision, observation time, each check,
  and a state-specific next step.
- Normalize the existing Gate API response envelope for both status reads and
  explicit reruns; retain the existing rerun control and server contract.
- Recast the legacy portal `settings` section as a `Quality` summary sourced
  from cached Gate evidence, while keeping its stable `settings` identifier.
- Link that section to the workbench and replace governance implementation
  commands with `forge gate status` and `forge gate` guidance.
- Put the read-only Assurance tools, including provider observations, under a
  collapsed **Advanced diagnostics and records** disclosure in the Projects
  view.

## Package Boundary and Split Assessment

This is one Forge-owned portal experience: operators discover project quality
in the workbench, receive honest recovery guidance, and can expand advanced
records when needed. The workbench card, legacy portal pointer, and collapsed
diagnostics disclosure share the same operator boundary and are independently
verified through the SPA, portal projection, and existing contracts. No sibling
project is modified. Sibling portability remains a later package.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-quality-checks-portal-ui` | Make Gate quality evidence clear and provider diagnostics secondary in Forge's project portal | Forge, Rust + HTML/CSS/JavaScript | Existing Gate evidence API, `PortalSection` summary, workbench and assurance SPA | `forge-web-project-workbench`, `web-assurance-browser`, `portal-spa-convergence` | Portal projection tests, web contracts, and a Chromium operator flow |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge Gate evidence | `src/api/admin/gate.rs`, `frontend/app.js::renderGateStatus` | Existing typed read/run routes, persisted revision-bound evidence, per-check results and freshness | Renderer currently misses the outer `envelope` on initial reads; the result is buried under Health | Forge owns the portal; Gate evidence stays behind Forge's existing interface | **extend shared owner** |
| Forge portal and assurance SPA | `src/portal/model.rs::PortalSection`, `frontend/index.html#assurance-browser` | Stable section identifiers, SPA deep links, native HTML disclosure and text-only rendering | `settings` currently projects provider state and assurance reads are always expanded | Forge owns the internal operator UI and its portal contract | **extend shared owner** |
| Sibling UI/platform packages | `crates/*/AGENTS.md`, in-tree modules and canonical portal contracts | Siblings consume Forge checks; no reusable operator UI component is owned by them | A sibling UI dependency would expose Forge internals and reverse ownership | Keep the implementation in Forge; no sibling release boundary changes | **keep local** |

## User Experience and Interface Impact

The actor is a signed-in Forge operator managing a registered project. From
**Projects**, the operator opens a project in **Workbench** and finds **Quality
checks** as its own card beside project health. The initial/read-only state
shows the last real Gate result, freshness, short revision, timestamp,
summary, and check list. Missing evidence says **Not run** and explains that
absence is not a pass. Stale evidence says it belongs to an earlier revision.
Blocked or failed evidence directs the operator to review the listed checks,
fix the project, and use the existing **Re-run gate** control. Fresh passing
evidence says the current revision passed.

When evidence cannot be read or the explicit run fails, the card and existing
workbench notice retain the unavailable/error result and never claim success.
The **Projects** Assurance tools remain available under a keyboard-operable,
collapsed **Advanced diagnostics and records** disclosure. Governance provider
reads are inside it and still require an explicit operator action; page load
does not probe or run providers.

All state and next-step text is available to assistive technology through the
existing polite status region. The existing re-run button remains the only
Gate control. The layout uses the current responsive workbench grid, retains
visible focus and touch targets, and adds no motion or dependency. Completion
requires a signed-in Chromium workbench flow at desktop and 390px, including
empty, fresh result, stale result and diagnostics disclosure states.

## BFS Impact Map

| Area | Impact and invariant |
|---|---|
| Workbench UI | Promote the existing Gate status to a dedicated Quality card; preserve the existing route and rerun action |
| Evidence contract | Consume the current Gate API response; do not add an endpoint, infer a pass, rerun automatically or expose runtime paths |
| Portal projection | Keep `settings` as a stable section id while reporting Gate aggregate/freshness and linking to the Quality card |
| Assurance UI | Collapse all read-only registry tools under Advanced diagnostics; writes/probes remain unavailable in the browser |
| States and privacy | Preserve `unknown`, `stale`, `absent`, `blocked`, `failed`, and unavailable distinctly; never serialize project paths from the portal summary |
| Siblings and shared code | No sibling `common`/`manager` capability is appropriate; Forge owns the control plane and sibling consumption is deferred |
| Tests and compatibility | Update the existing portal mapping and Gate UI contracts; retain the `settings` wire identifier, API payload and contract version |

## Capabilities

- `forge-web-project-workbench`
- `web-assurance-browser`
- `portal-spa-convergence`

## Non-goals

- Do not migrate sibling consumers in this change.
- Do not change Gate execution, persisted evidence, provider selection,
  authentication, APIs or contract versions.
- Do not run provider observations automatically or make product projects
  display Forge's governance implementation.
- Do not add Gate controls beyond the existing explicit rerun.
