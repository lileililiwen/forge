# Proposal: expose Gate profiles in the project workbench

## Why

Forge Gate now records `fast` or `full` evidence, but the project workbench hides
that distinction and its rerun route always selects the default `full` profile.
Operators cannot tell which scope produced evidence or deliberately request the
quick baseline from the browser.

## What Changes

- Display the selected profile beside local Gate evidence and hosted observations
  when profile data is present.
- Add an explicitly labelled `fast`/`full` selector to the Quality checks card;
  initialize it to `full` and submit the selected value to the existing run route.
- Parse and validate the typed profile in the authenticated API handler, preserve
  omitted-profile compatibility as `full`, and return a typed client error for
  malformed or unsupported profiles before any Gate execution or persistence.
- Preserve the existing shared evidence renderer, status meanings, authorization,
  and no-evidence behavior.

## Package Boundary and Split Assessment

This is one Forge-owned, independently verifiable operator workflow spanning the
existing authenticated Gate run endpoint and its existing browser control. The
request value, execution choice, evidence profile and rendered profile share one
state transition and one acceptance oracle. Splitting API and UI would leave the
profile feature unusable or silently defaulted for browser operators. No sibling
project, language, persistence migration, or release owner is added. No external
contract package is needed: the already versioned Gate evidence `profile` field
and existing API route are the boundary. Sibling projects remain unchanged.

## Sibling and Shared Architecture Reconnaissance

This is not Forge's first implementation change. The relevant shared capability
is already owned by Forge Gate and its versioned evidence contract; this change
consumes it without changing sibling projects or extracting a second profile
schema.

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge Gate | `src/gate/model.rs:GateProfile`, `src/gate/engine.rs:run_gate`, `src/api/admin/gate.rs` | Existing `fast`/`full` enum, default and persisted evidence field | API accepts no typed selection and UI omits profile | Forge; same source/API release | **adopt** |
| Forge project workbench | `frontend/index.html` Quality checks card, `frontend/app.js:renderGateStatus`/`runGate` | Existing status card, authenticated rerun route, safe text rendering | No profile control or profile label | Forge; same browser release | **extend shared owner** |
| Sibling projects | No sibling source change in scope | Their hosted evidence already transports versioned Gate data | No need for sibling-specific profile UI | Each sibling owns its own release | **keep local** |

## User Experience and Interface Impact

The authenticated Forge operator opens a managed project's Quality checks card.
The card retains its current evidence and next-step hierarchy, adds a labelled
Gate profile selector defaulting to `full`, and labels local and hosted evidence
with its recorded profile when available. The operator can choose `fast` for the
mandatory baseline or `full` for baseline plus declared extensions, then activate
the existing rerun action. The action shows pending state; a typed request error
is announced without replacing prior evidence; a completed run uses the existing
renderer and announcement. The control remains keyboard operable, visibly
labelled, responsive with the existing card, and uses no color-only state. The
workbench's narrow grid must wrap values and keep the Quality card within the
viewport. Local legacy evidence retains Gate's documented `full` default;
unrecognized profile values show `Profile unavailable`. Browser verification
covers both profiles, validation/error handling,
loading and preserved evidence; accessibility verification covers label/name,
keyboard use, focus and narrow viewport reflow.

## BFS Impact Map

| Surface | Impact and verification |
|---|---|
| Requirement/state | Selected profile flows from UI request to `GateConfig`, persisted evidence and rendered result. |
| API contract | Existing authenticated POST accepts `{profile?: "fast"|"full"}`; absent field remains `full`; invalid shape/value returns 400 before running or writing. |
| Persistence | Existing evidence schema already stores the profile; no schema or migration change. |
| Callers | Browser sends a typed field; older clients omitting it retain full behavior. CLI and hosted ingest semantics are unchanged. |
| Failure/boundary | Invalid JSON/profile, unauthorized requests and unavailable Gate retain typed failures and prior evidence; fast evidence remains ineligible for hosted delivery under existing rules. |
| UI/security | Use existing session gate, server-resolved project ID, safe text rendering, no paths or arbitrary process input. |
| Tests | API contract checks default, fast/full propagation, invalid request non-execution and existing authorization; browser contract verifies labels, selector, request, result/error states and responsiveness. |
| Runtime | Local browser flow and applicable accessibility check plus Forge Gate dry-run/full run before archive. |

## Capabilities

- `gate-profile-portal-controls`: select, execute, identify and inspect a Gate
  profile through the authenticated project workbench.

## Non-goals

- Changing profile semantics, baseline checks, hosted evidence acceptance or CLI
  defaults.
- Adding a third profile, changing persisted evidence, introducing a new route,
  changing sibling projects, publishing Forge, or deploying the web UI.
