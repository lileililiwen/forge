# Proposal: Task-first dashboard with one-confirm bulk onboarding

## Why

The dashboard currently leads with the fleet table and the full command
catalog, while the actions that manage projects — workbench, creation and
workspace onboarding — sit further down the page. For an operator with an
expanding workspace this reads as a catalog to study, not a tool to use:
onboarding 70+ siblings takes a Discover click plus one preview/confirm cycle
per 25-item batch, and nothing on first paint says how many siblings are
still unmanaged. This change reorders the dashboard around work and makes
bulk onboarding a single reviewed confirmation however large the selection.

## What Changes

- Reorder the dashboard sections and sidebar nav to match the operator's
  workflow: fleet → workbench → management (creation + workspace
  onboarding) → portfolio → delivery → command catalog (reference, last).
- Auto-run workspace discovery when the signed-in dashboard loads, and show
  an “N workspace directories not yet onboarded” line under the fleet that
  links to the onboarding panel. A failed or unconfigured discovery degrades
  to the existing manual Discover button and prerequisite notice.
- Chunked one-confirm onboarding: selections larger than the 25-item server
  batch are previewed per chunk, shown as one combined plan with one digest
  per chunk, and applied chunk-by-chunk under a single confirmation tick;
  any chunk refusal stops the run with honest partial results.
- Relabel the commands section as reference (“Browse the CLI surface…”)
  without removing any row or changing any disposition.

## Package Boundary and Split Assessment

One independently verifiable outcome: a signed-in operator opening the
dashboard sees work first, knows immediately how many siblings are
unmanaged, and can onboard an arbitrarily large selection with one reviewed
confirmation. Reordering, auto-discovery, the count line and chunked apply
share one surface (`frontend/`), one journey and one browser oracle;
splitting them would ship incoherent intermediate states. No new API routes,
no catalog changes, no Core changes.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-command-workflows` (**this**) | Task-ordered dashboard with auto-discovery, unonboarded count and one-confirm chunked onboarding | Forge `frontend/` / JS+HTML | Existing `/v1/admin` routes only; DOM order + flow | `forge-web-workspace-onboarding` | Extended Playwright onboarding drive (27-dir fixture: auto-discovery, 2-chunk confirm, fleet-appears) |
| (none) | — | — | — | — | — |

No further split: everything here is presentation and orchestration of
already-shipped, already-tested routes.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Management/onboarding panel | `frontend/app.js` `wsDiscover`/`wsPreview`/`wsRun`, `frontend/index.html` `#management` | Existing panel, controls and styles | Manual discover; single-batch; buried below catalog | `frontend/` owns presentation | **extend shared owner** — reorder, auto-run, chunk |
| Fleet summary | `frontend/app.js` dashboard fleet load, `#project-count` | Existing fleet fetch | No unonboarded signal | `frontend/` | **extend shared owner** — count line from the discovery payload |
| Command catalog section | `frontend/index.html` `#commands` | Full row rendering, filters | Dominates the page by position | `frontend/` | **keep local** — move last, relabel, change nothing else |

No sibling checkout is touched; no host folder appears in code.

## User Experience and Interface Impact

Actor: the authenticated global admin managing an expanding workspace.
Entry point: the dashboard itself — the first viewport now shows fleet
health plus the unonboarded count, and the first actions below it are the
workbench and management panels. Primary flow: open dashboard → see “N
directories not yet onboarded” → onboarding table already populated →
Select all → Preview (one combined plan) → tick confirm → Run (chunks
apply in order) → Reload fleet. Alternate flows: manual Discover/Refresh
still available; unconfigured root still shows the prerequisite notice;
partial chunk failure stops with per-item results and a re-preview prompt.

Content priority: what needs doing (unonboarded count, onboardable rows)
above reference material (catalog). Responsive/keyboard/a11y behavior reuses
the existing table region, labelled controls, live regions and focus hooks;
new text is covered by the browser contrast check. Browser evidence: the
extended Playwright drive asserts section order, auto-discovery, chunked
confirm, fleet-appears, keyboard, contrast and no-path-rendered.
`UI/UX: N/A` does not apply.

## BFS Impact Map

- **Capabilities:** `forge-web-command-workflows` (new). No API, catalog,
  Core or journal changes.
- **Users / flows:** every signed-in dashboard visit; onboarding selections
  of any size.
- **Contracts / data / persistence:** none new; chunk previews reuse the
  existing per-batch digest contract.
- **Callers:** `frontend/app.js`, `frontend/index.html` only.
- **Failure / boundary behavior:** discovery failure keeps manual controls;
  a refused chunk stops the run, keeps completed chunk results, and prompts
  re-preview; reload still manual via button.
- **Tests:** extended browser drive (27-dir fixture) + frontend source
  contract (section order, auto-discover, chunking markers); all existing
  suites unchanged.
- **Privacy / security:** unchanged — directory leaves and typed overrides
  only, as before.

## Capabilities

- `forge-web-command-workflows`: task-ordered dashboard with one-confirm
  bulk onboarding.

## Non-goals

- No new API routes, catalog rows, Core behavior or journal shapes.
- No removal of the command catalog; it moves last as reference.
- No automatic writes: discovery auto-runs (read-only), onboarding still
  requires explicit preview review + confirmation tick.
- No change to single-item management controls.
