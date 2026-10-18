# Proposal: Correct governance and portal status surface regression coverage

## Why

The recurring Forge Gate failure comes from `tests/governance_cross_surface.rs` treating the portal `settings` row as a governance observation. The row is currently a Gate quality-check projection: when no Gate evidence exists, `unknown` is the required honest state. The same test also expects a `governance_provider` attribute the portal row does not define. The original test was introduced with Workspace Governance adapter consumption, then the portal settings row was repurposed to show Gate evidence. The assertion now couples two independent status models and fails every workspace test run.

## What Changes

- Correct the cross-surface regression test to assert adapter failure through the governance status command and the checker alert.
- Assert that the portal settings quality row remains `unknown` when Gate evidence is absent, with no governance-provider attribute.
- Exercise an adapter-reported failure through governance status and checker, without asserting that it changes Gate state.
- Add a canonical requirement that governance observations and Gate quality evidence keep their own status ownership and projections.

## Package Boundary and Split Assessment

This is one test-contract correction for the existing governance/Gate read surfaces. The root cause is a stale assertion in one integration test, not a broken adapter status mapping: `src/portal/sections.rs::build_settings_section` reads only Gate evidence, and its no-evidence branch is specified as `unknown`. Changing production projections to satisfy the stale assertion would conflate Gate state with governance state. There are no independent persistence, runtime, or UI outcomes to split.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `governance-status-surface-regression` | Keep governance adapter status and Gate quality status separately attributed across existing surfaces | Forge Rust integration contracts and OpenSpec | `tests/governance_cross_surface.rs`, governance CLI/checker, portal settings projection; no schema change | Existing governance provider and Gate evidence contracts | Focused cross-surface integration test plus the complete Forge Gate |

## Sibling and Shared Architecture Reconnaissance

This change tests Forge's status projection boundary and does not add governance semantics or an adapter. Workspace Governance remains the owner of its audit semantics; Forge's existing normalized governance provider contract remains the wire boundary. No sibling source needs modification.

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge governance provider | `src/governance/engine.rs::evaluate_project`; `openspec/specs/governance-provider-contract/spec.md` | Normalized adapter observation and unavailable/fail statuses | None; this change corrects only its test consumer | Forge release | keep local |
| Forge Gate portal projection | `src/portal/sections.rs::build_settings_section`; `openspec/specs/quality-checks-portal-ui/spec.md` (archived change promotes portal UI behavior) | Revision-bound Gate evidence and absent-evidence `unknown` state | Governance observations are a separate contract and are intentionally not represented on this row | Forge release | keep local |
| Workspace Governance module | `modules/workspace-governance/scripts/forge_governance_adapter.py` | Audit adapter and its response semantics | Not involved in the regression; no module change is needed | In-tree Forge module, independently maintained | keep local |

## User Experience and Interface Impact

There is no new or changed user interaction. Existing governance status and checker outputs keep their behavior; the portal settings row continues to display Gate quality evidence. This package changes the automated oracle to match those existing ownership boundaries. UI/UX: N/A for runtime changes because no interface behavior is being introduced or altered; the existing CLI and portal are covered as consumers by integration assertions.

## BFS Impact Map

- **Capabilities:** normalized governance observations, checker alerts, portal Gate quality row.
- **Actors and flows:** project operators inspect governance through `forge governance status`, `forge check`, and portal settings.
- **Contracts/data:** no wire or persistence shape changes; governance provider observations and Gate evidence remain independently sourced.
- **Callers:** governance CLI and checker evaluate the selected adapter; portal settings reads Gate evidence only.
- **Failure/boundaries:** missing adapter becomes governance `unavailable` and a checker warning; adapter `fail` becomes a checker error; absent Gate evidence stays portal `unknown`.
- **Tests:** replace the incorrect portal assertions in `tests/governance_cross_surface.rs`; retain failing-adapter and healthy-isolation coverage.
- **Compatibility/security:** no new process, path, credential, or remote behavior.
- **Verification:** focused regression, related governance contracts, full test binding, Forge Gate dry-run/full run, strict OpenSpec validation.
- **Unaffected:** adapter execution, provider protocol, database, portal routes and UI remain unchanged.

## Capabilities

### Governance and Gate status projection separation

Operators and maintainers can distinguish governance provider health from project Gate quality evidence across the existing read surfaces; a status from one owner cannot overwrite or masquerade as a status from the other.

## Non-goals

- Changing governance adapter execution or status mapping.
- Adding governance data to the portal settings row or changing Gate absent-evidence semantics.
- Adding a new endpoint, UI surface, persistence field, or sibling dependency.
- Fixing other unrelated Gate failures.
