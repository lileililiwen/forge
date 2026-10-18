# Proposal: ingest authenticated hosted CI Gate evidence

## Why

Generated project workflows can run Forge Gate, and Forge validates a versioned hosted evidence envelope, but Forge cannot currently accept an authenticated report or show it beside local Gate evidence. Operators therefore cannot correlate a hosted run with a registered project through Forge.

## What Changes

Forge will accept evidence only from GitHub Actions OIDC tokens with a fixed issuer and audience, verify the token through GitHub's published signing keys, compare signed repository/run/SHA claims to the envelope and a Forge-managed immutable repository ID, persist observations append-only and idempotently, and expose them in the authenticated project workbench. Versioned service and client standard snapshots will add a least-permission upload job and a pinned Forge-owned uploader script. Hosted evidence remains a separate observation and never becomes local Gate or delivery approval.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-hosted-ci-evidence-ingest` | Authenticated GitHub Actions Gate observations are recorded and reviewable per registered Forge project | Forge, Rust and existing HTML/JS workbench | `forge-hosted-ci-evidence/1.0`, OIDC issuer/audience, SQLite registry, admin API, standard uploader and generated workflows | `hosted-ci-evidence-envelope` | Signed-token fixture upload → persisted idempotent read → browser workbench; invalid identity has no write |

Dependency order: `hosted-ci-evidence-envelope` → this change. The request contains API, persistence, workflow and UI edits, but these share one observation lifecycle and cannot be independently accepted: the generated producer must successfully reach the operator-visible record. A separate CI service, provider workflow engine, delivery approval integration, and sibling-project edits are excluded.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge hosted envelope | `src/ci_evidence.rs`; `openspec/specs/hosted-ci-evidence-envelope/spec.md` | Typed envelope with Gate record and closed fields | No auth, persistence or transport | Forge crate/API; additive registry migration | **extend shared owner** |
| Forge Gate and workbench | `src/gate/`; `src/api/admin/gate.rs`; `frontend/app.js` `loadGateStatus` | Existing evidence semantics, session-guarded routes, project Quality card | Hosted observation needs separate history and freshness projection | Forge modular monolith | **adopt** |
| Forge standard snapshots | `standards/baseline-service/1.4.0`; `standards/baseline-client/1.1.0` | Pinned generated workflow and install scripts | No OIDC upload step or uploader script | Forge standard pack versions | **extend shared owner** |
| Workspace Governance / sibling projects | No hosted evidence endpoint is owned there; Forge's vendored governance is internal policy | No compatible OIDC evidence store | Cross-project runtime coupling would violate Forge's generated-project independence | Projects depend on Forge's published CLI/contract | **keep local** |

The durable contract, verification policy, registry and upload implementation have one owner in Forge. Sibling projects consume generated workflow artifacts and the installed Forge binary; they do not import Forge source or require a sibling checkout. This change does not modify or extract code in another project.

## User Experience and Interface Impact

The Forge operator opens a project's existing **Quality checks** card. Initial state loads local and hosted observations independently; an empty hosted history explains how to enable upload and whether the project has a trusted repository ID. A bounded history lists workflow, run link, revision, observed time, aggregate and freshness, with newest first. Loading, no observations, stale revision, unavailable API, unauthorized session, and successful upload states are explicit. Repository ID binding is an operator-confirmed action in that card and is immutable after first bind; changing it requires a separate deliberate reset operation added only if current identity lifecycle supports safe reset. Layout remains usable at 390px, keyboard-only, and screen-reader status uses existing live regions. Verify signed-in Chromium flows, contrast/semantics, focus, and narrow layout; API tests alone do not satisfy this change.

## BFS Impact Map

| Surface | Impact and invariant |
|---|---|
| Actor / journey | Operator binds trusted GitHub repository ID, enables standard workflow upload, inspects hosted history in project Quality checks. |
| Contract | Existing envelope stays `forge-hosted-ci-evidence/1.0`; request body stays closed and bounded. Signed OIDC claims are authoritative for repository, repository ID, workflow, run ID/attempt, event/ref and SHA. |
| Persistence | Add project repository ID and append-only hosted observations. Unique identity is `(project_id, run_id, run_attempt)` because GitHub's signed Actions claims do not attest the job name; exact body digest replay is success, conflicting replay is 409. Never overwrite history. |
| Integration | Public workload endpoint uses a separate OIDC verifier; operator setup/read routes continue to use Forge admin sessions. Standard workflows split verification from upload so only upload receives `id-token: write`. |
| Callers | Add a pinned standard uploader script; service/client workflows invoke it only when `FORGE_CI_EVIDENCE_URL` is configured. |
| Failure boundaries | Reject bad signature/issuer/audience/expiry/key/repository/repo ID/run/SHA/project binding, malformed envelope, unknown project, conflicting duplicate, oversized payload and unavailable key service without persisting. Never log bearer tokens. |
| Security/privacy | Fixed GitHub issuer and audience; RS256 allowlist, `kid`-selected JWKS, bounded HTTPS fetch/cache; no caller-selected key URL, redirects, logs or environment snapshots. Keep raw OIDC token out of persisted record and response. |
| Compatibility | Additive SQLite migration; prior project rows have no trusted repository ID and cannot ingest until operator binds one. Prior pack snapshots remain reproducible; new pack versions are opt-in/upgradeable. |
| UI | Extend existing Quality card with repository binding and hosted observation history. Preserve local Gate projection and delivery precondition exactly. |
| Tests | JWT positive/negative fixtures, JWKS rotation/cache failures, repository claims, registry migration/idempotency/races, route auth, workflow permissions, CLI uploader, and browser/a11y/narrow-screen flow. |
| Unchanged | No sibling source/runtime dependency, no automatic delivery approval, no branch-protection replacement, no raw artifact/log upload, no generic Forge scheduler. |

## Capabilities

- `hosted-ci-evidence-ingest` — authenticate, bind, store, upload and display hosted CI Gate observations.

## Non-goals

- Scheduling or running hosted builds in Forge.
- Treating GitHub observations as local Gate evidence, branch protection, release approval, or delivery preconditions.
- Accepting provider types other than GitHub Actions in this contract version.
- Uploading source, build artifacts, logs, secrets, or arbitrary environment data.
- Modifying sibling projects or establishing a runtime dependency on Forge after generation.
