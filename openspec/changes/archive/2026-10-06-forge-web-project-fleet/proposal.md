# Proposal: Show the complete Forge-managed project fleet

## Why

The new global sign-in reaches a dashboard backed by one selected Forge registry. That is not the operator's whole portfolio, and Forge itself is absent unless separately registered. Operators need one truthful fleet view across explicitly configured sources, without being forced to know internal project IDs to sign in.

## What Changes

- Combine registered Forge projects, an explicitly selected portable inventory source, and a guaranteed Forge-self record into a normalized, provenance-bearing web fleet.
- Preserve source freshness, malformed/unavailable states and duplicate/source conflicts instead of silently dropping entries or scanning sibling directories.
- Allow operators to search, filter, inspect and open projects after global sign-in; distinguish Forge itself, managed projects and externally observed projects.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-project-fleet` | Authenticated web users see the complete, source-attributed project fleet including Forge. | Forge / Rust 2021 API and static web; browser HTML/CSS/JS | Versioned normalized fleet read model | `forge-global-admin-portal` implementation evidence; existing registry and `forge-project-inventory/0.1.0` | Fixture with registry + external inventory + self yields every expected row and source state |
| `forge-web-command-catalog` | Every CLI command has an explicit web entry or a reasoned CLI-only classification. | Forge / Rust 2021 and standalone browser assets | Versioned command catalog with stable command IDs, risk and route mapping | `forge-web-project-fleet` | Enum-to-catalog completeness oracle and browser category/unsupported states |
| `forge-web-project-workbench` | Operators perform project setup, inspection, planning and quality workflows in the browser. | Forge / Rust Core/API plus standalone browser assets | Existing Core operations exposed as authenticated JSON commands | `forge-web-command-catalog` | End-to-end project workflow parity against CLI Core outcomes |
| `forge-web-portfolio-controls` | Operators manage portfolio metadata and inspect fleet/governance/readiness evidence in the browser. | Forge / Rust Core/API plus standalone browser assets | Existing portfolio/catalog/provider read and mutation contracts | `forge-web-command-catalog` | State/evidence parity and source ownership checks |
| `forge-web-delivery-controls` | Operators review and execute supported repository, publish and delivery workflows in the browser. | Forge / Rust Core/API plus standalone browser assets | Existing operation plans, confirmations and journal IDs | `forge-web-command-catalog`; project workbench and portfolio controls | Confirmation/security tests and operation journal parity |

Dependency order is the table order. The requested outcomes have different state owners, risk boundaries and independent acceptance oracles, so this change delivers only the fleet read model. It does not redefine project inventory or deliver command execution.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge portable inventory | `src/publish/inventory.rs`; `openspec/specs/forge-independent-project-inventory-fleet/spec.md` | Versioned validated inventory and explicit local/executable source selection | Publish classifier has deployment-specific fields and is not the web fleet read model | Forge inventory owns input validation; web fleet owns aggregation/provenance | **adapt through a generic adapter** |
| Forge workspace fleet observer | `src/fleet/mod.rs`; `openspec/specs/fleet-registry-observation/spec.md` | Explicit registry path, bounded freshness, malformed-entry reporting | Reads one externally declared workspace registry; does not aggregate or discover Forge itself | Forge fleet module; same Rust release | **extend shared owner** |
| AllTools | User-provided `/home/paul/code/alltools-platform/rust/templates/frontend/login.html` | Visual reference only | Different account, project and runtime contracts | AllTools owns its own release | **keep local** |

## BFS Impact Map

- **Actors/flow:** Forge operator signs in once, views/searches all sources, sees Forge as a first-class row and opens supported project views.
- **Contracts/data:** add a normalized fleet DTO carrying stable identity, display name, source, source identity, management capability, freshness and conflict state. Never use project ID as the login credential.
- **Persistence/configuration:** no new project database or implicit workspace scan. External source remains the existing explicit inventory/registry selection; Forge-self identity derives from the running build/repository metadata with a stable configured override.
- **Callers:** authenticated `/v1/admin/projects` and standalone `frontend/`; existing CLI inventory/fleet commands and `/v1` project APIs remain unchanged.
- **Failures/security:** preserve partial source results, label unavailable/stale/malformed sources, detect duplicate identities and fail closed on unauthorized reads. Do not leak local filesystem paths to browser responses.
- **Verification:** unit tests for normalization, source merge, identity/deduplication and freshness; API contract tests; browser tests for empty/partial/full fleet and self row.
- **Unaffected:** project OIDC, publish routing, inventory provider behavior, and all mutation operations.

## Capabilities

- `forge-web-project-fleet`: expose a complete authenticated project fleet with source provenance, including Forge.

Source requirements: `requirement.md` §§35–36; canonical `control-plane-portal`, `fleet-registry-observation`, and `forge-independent-project-inventory-fleet`.

## Non-goals

- No implicit scan of `/home/paul/code`, sibling repositories, Git hosts or the filesystem.
- No mutation of inventory providers or foreign project registries.
- No claim that every discovered project can be operated by Forge; expose capabilities per source.
- No changes to global login/session policy or CLI command execution.
