# Proposal: Human-centered dashboard language and visual system

## Why

The dashboard manages real projects but speaks programmer: fleet rows lead
with kebab-case ids, source/lifecycle/access codes and evidence chips;
workbench and delivery cards print 40-hex revisions, operation ids and
plan digests; errors quote identifier grammars. For the operator this is a
wall of implementation detail, not a tool: nothing on screen answers “what
is this project, how is it doing, what do I do next” in human words, and
the light-generic styling gives no visual hierarchy to act on. This change
makes the dashboard human-readable and gives it one coherent famous visual
system (Linear-style dark command center) without weakening any safety
property — digests, ids and hashes stay in machine state, they just stop
shouting at people.

## What Changes

- Fleet table speaks human: project display name first (id as quiet
  secondary), plain state words, one “needs attention” signal per row
  instead of source/lifecycle/access/evidence codes.
- Workbench, delivery, onboarding and action cards stop printing hashes,
  digests, operation ids and full revisions; confirmations reference the
  reviewed action in words (“onboard the 27 selected directories”).
  Digests/ids remain in JS memory and wire bodies unchanged.
- Backend user-facing refusal strings on the management/onboarding/fleet
  surfaces become plain language (“use lowercase letters, numbers and
  dashes”); typed codes and statuses are unchanged, and pinned tests are
  updated to the new strings.
- A Linear-style dark visual system across `frontend/` (tokens, sidebar,
  cards, tables, badges, buttons, notices, login) meeting WCAG 2.2 AA,
  verified by the existing browser contrast checks.
- After a successful onboard run, the fleet re-fetches and re-renders in
  place (results stay visible) instead of requiring a full-page reload.

## Package Boundary and Split Assessment

One independently verifiable outcome: a signed-in operator can read and
operate the whole dashboard without encountering an identifier grammar, a
hash, or a generic unstyled page. Copy, display rules, error strings and
the visual system share one surface (`frontend/` plus user-facing API
strings), one journey and one browser oracle. API shapes, codes, statuses,
digests and journal behavior are unchanged, so no backend capability splits
off.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-human-dashboard` (**this**) | Human words and a coherent visual system across the dashboard with identical safety properties | Forge `frontend/` + user-facing API strings / JS+CSS+Rust strings | Display layer only; wire/digest/journal untouched | `forge-web-command-workflows` (panel structure) | Extended browser drive (readable copy assertions, contrast, no-hash-rendered) + frontend source contract |
| `forge-workspace-sync` (parked) | `forge workspace sync` CLI mechanism | Forge `src/import` + `src/main.rs` | New CLI/Core, no browser surface | — (independent; resumes after this) | `tests/workspace_sync_contract.rs` |

The parked `forge-workspace-sync` package (proposal/design/tasks/spec
authored, unimplemented) resumes after this change; neither implements or
redefines the other.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Dashboard rendering | `frontend/app.js` render functions, `frontend/index.html` sections | `textContent`-only rendering, live regions, table patterns | Copy is programmer-centric; digests printed | `frontend/` owns presentation | **extend shared owner** — rewrite copy/display rules, keep every id and route |
| Visual system | `frontend/styles.css` (45 lines, light generic) | Every selector the JS/HTML depends on | No coherent system, low hierarchy | `frontend/` | **extend shared owner** — re-tokenize to a dark command-center system, keep all selectors |
| Refusal strings | `src/api/admin.rs`, `src/api/project_management.rs`, `src/api/workspace.rs` error literals | Typed codes + statuses | Messages quote grammars/paths | `src/api` owns messages | **extend shared owner** — plain-language messages, same codes/statuses |
| Linear design language | industry-standard dev-tool aesthetic (dark surfaces, indigo accent, pill badges, quiet borders) | Visual vocabulary only, no code | Must meet AA contrast on dark | none (public design language) | **adapt through a generic adapter** — tokens mapped by hand, no dependency |

No sibling checkout is touched; no host folder appears in code.

## User Experience and Interface Impact

Actor: the operator, non-programmer-readable UI required. Entry: the
dashboard as today. Primary flow unchanged in steps, transformed in words:
fleet rows read as names with one status line; every mutating control
previews in sentences and confirms in sentences; errors say what to do in
plain words. Blocked/unavailable states keep their honesty, minus the
jargon. The dark visual system gives hierarchy: sidebar navigation, glowing
active states, cards over a near-black canvas, accent reserved for actions.
Responsive/keyboard/a11y behavior unchanged in kind (same landmarks,
controls, live regions, focus hooks); contrast re-verified by the browser
oracle on the new palette. `UI/UX: N/A` does not apply — this change IS the
interface.

## BFS Impact Map

- **Capabilities:** `forge-web-human-dashboard` (new). No API shape, code,
  status, digest or journal change.
- **Users / flows:** every dashboard visit and every mutating preview/
  confirm/result cycle.
- **Contracts / data / persistence:** untouched; digests/ids/hashes remain
  in wire bodies and JS memory, removed from display only.
- **Callers:** `frontend/app.js`, `frontend/index.html`,
  `frontend/styles.css`; a bounded set of backend message literals.
- **Failure / boundary behavior:** identical codes/statuses; messages
  rewritten; pinned message assertions updated.
- **Tests:** browser drive gains readable-copy + no-hash-rendered
  assertions; frontend source contract pins the token set and section
  order; contract tests updated only where they pinned exact message
  strings.
- **Privacy / security:** strictly less exposed (fewer identifiers on
  screen); nothing else changes.

## Capabilities

- `forge-web-human-dashboard`: human language and coherent visual system
  for the dashboard with unchanged safety properties.

## Non-goals

- No new routes, commands, catalog rows, Core behavior or journal shapes.
- No change to codes, statuses, digests, idempotency or confirmation
  discipline.
- No removal of information operators need for debugging: full identifiers
  remain one step away (workbench detail/operations journal), just not in
  primary views.
- No light-theme variant; one dark system done well.
