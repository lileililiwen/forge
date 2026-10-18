# Proposal: Gate delivery preflight on current quality evidence

## Why

`forge delivery preflight` currently invokes the publish provider using the registry's recorded commit without checking Forge Gate. A provider success can therefore be mistaken for Forge quality approval, and a stale registry revision can be delivered.

## What Changes

Require the registered revision to equal the source repository's current Git `HEAD`, and require the persisted Gate evidence and shared freshness classifier to report fresh `passed` evidence for that revision before delivery preflight resolves/reserves/invokes the provider. Refusals remain typed across CLI/API/workbench, explain how to refresh registration or run Gate, and create no delivery journal row. Update the CI/CD model gap register after implementation.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-delivery-gate-precondition` | Delivery preflight requires current registered source and passing Gate evidence | Forge Rust modular monolith | `run_preflight` consumes registry revision, Git HEAD, and the Gate evidence freshness classifier; existing CLI/API error envelope | Existing Gate evidence and delivery workflow | Contract tests prove rejection has no provider call or journal row and valid evidence allows preflight |
| Hosted CI evidence handoff (future) | Import runner-produced evidence into Forge | Forge Rust + GitHub Actions | New authenticated ingest contract | This package's Gate prerequisite | Remote runner envelope can be accepted and projected as trusted evidence |

This package has one owner, one preflight boundary, and one acceptance oracle. Hosted evidence import has a distinct trust and persistence lifecycle and remains a separate follow-up.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge Gate runtime | `src/gate/engine.rs::load_latest_evidence` and `evidence_freshness`; `src/gate/model.rs::GateEvidence` | Shared persisted evidence model and revision freshness classifier | None; delivery must use the common evidence reader/classifier rather than reimplement freshness | Forge Core and Forge binary | **adopt** |
| Publish provider contract | `src/delivery/handlers.rs::run_preflight`; `forge-publish-provider/0.1.0` adapter | Existing provider invocation, journal and typed error contract | Provider contract does not represent Forge Gate approval | Forge coordinates; provider remains external | **adapt through a generic adapter** |
| Workspace Governance / sibling tooling | README consolidated tooling boundary; no sibling checkout is required at runtime | No external code needed: Gate runtime is already bundled in Forge | Cross-project dependency would undermine standalone operation | Forge owns the delivered binary | **keep local** |

## User Experience and Interface Impact

Actors are project operators using `forge delivery preflight`, authenticated delivery API routes, or the Delivery workbench. Existing workbench action remains the entry point. On rejection, CLI and API retain the typed error; the workbench's existing status/error region must show the reason and next action: run `forge gate <project-path>` and retry, or re-register after changing the commit. A successful fresh PASS continues the existing preflight → stage journey. Missing, stale, blocked, failed or unknown Gate evidence is an actionable error. No new screen or control is needed. Preserve existing semantic error announcement and keyboard behavior; verify through API/UI contract coverage and the existing Chromium delivery journey. Responsive layout is unchanged.

## BFS Impact Map

- **Capabilities/actors:** project-to-production-workflow; operators of CLI, authenticated project API, Delivery workbench.
- **Contracts/data:** no schema change; use registry `last_commit`, Git `HEAD`, Gate evidence JSON and shared freshness classifier; no new operation row on refusal.
- **Integrations/configuration:** Git process is read-only; no new environment setting or provider contract.
- **Callers:** CLI, project API and workbench share `run_preflight`; all retain existing error serialization.
- **Failure/boundaries:** unknown/unavailable Git revision, registry/HEAD mismatch, absent/stale/non-passing Gate all block before provider operation. Invalid Gate declaration remains a typed failure. Only aggregate `passed` with freshness `fresh` is accepted.
- **Tests/dependencies:** delivery contract, cross-surface/API and browser delivery tests; reuse existing Gate evidence, provider stub, operation journal.
- **Sibling assessment:** repeated CI evidence trust is a Forge-owned concern; sibling projects consume Forge commands and need not implement this precondition. Hosted evidence transfer is deferred to its own contract and authentication boundary.
- **Compatibility/security:** existing provider request remains unchanged; no provider command executes before the checks; never expose secrets. Existing successful preflight callers must prepare real Gate evidence at the current commit.

## Capabilities

- Modify `project-to-production-workflow`: make current registered source and fresh passing Gate evidence explicit preconditions for provider preflight.
- Modify `ci-cd-system-model-documentation`: close this gap and reprioritize remaining handoffs.

## Non-goals

- Running Gate automatically from delivery preflight.
- Importing GitHub-hosted Gate evidence or changing Gate trust/authentication.
- Requiring a clean working tree; Gate evidence binds to Git `HEAD` under the existing Gate contract.
- Changing stage, promotion, release, provider contracts, journal schema, or deployment UI layout.
