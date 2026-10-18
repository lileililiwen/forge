# Proposal: define the hosted CI evidence envelope

## Why

GitHub Actions runs Forge Gate, but its runner-local evidence disappears with the job. Forge has no stable document that joins the existing Gate result to an authenticated hosted run. Upload and ingestion require a precise payload and trust boundary first.

## What Changes

Forge defines and validates a versioned hosted-run envelope that embeds the existing Forge Gate evidence record (current contract version `0.1.0`) with GitHub repository, workflow, run, attempt, job and URL context. The document records producer claims; it does not authenticate those claims or grant release/delivery approval. A later authenticated ingest capability will bind repository identity and caller authority to the registered Forge project.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-hosted-ci-evidence-envelope` | Forge owns a strict, serializable hosted CI evidence envelope | Forge / Rust | `forge-hosted-ci-evidence/1.0` wrapping current Gate evidence | Existing Gate evidence contract (`0.1.0`) | Valid and invalid fixtures round-trip or fail closed |
| `forge-hosted-ci-evidence-ingest` | Authenticated, idempotent receipt and readback of hosted runs | Forge / Rust | Authenticated API and durable evidence store | This envelope | Authorized exact project/repository/SHA ingest and replay tests |

The envelope is independently verifiable, has no persistence or remote side effects, and is a required contract dependency for ingest. This split prevents schema definition from implying that arbitrary runner JSON is trusted.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge Gate | `src/gate/model.rs::GateEvidence`; `openspec/specs/gate-runtime-evidence/spec.md` | Existing versioned project/revision-bound checks and aggregate | Contains no hosted provider/run identity or authentication proof | Forge source and Forge release | **extend shared owner** by wrapping the stable Gate evidence unchanged |
| Shared platform Gate result | `rust-platform-libs/crates/platform-contracts/src/payload/gate_result.rs`; `src/contract/mod.rs::emit_gate_result` | Portable, distilled Gate result vocabulary already consumed by Forge | It omits repository and workflow identity and is not a substitute for Forge's complete Gate evidence | Platform contract owner; Forge projects it through a vendored contract | **adapt through a generic adapter**; retain the existing projection without changing its schema |
| Generated CI packs | `standards/*/` and `src/standard/layout.rs` | Forge-owned workflow templates invoke Gate | No export/upload step and no run envelope | Forge pack snapshots distributed to generated projects | Future consumer of the contract; no template edit in this package |
| Prior governance/DriftWatch evidence | `src/gate/evidence.rs::EvidenceExport` | Separate release-evidence vocabulary and safe parsing patterns | Different purpose and schema; no GitHub run context | Forge consumes a DriftWatch export | **keep local** to the Gate evidence owner; do not reuse as hosted CI schema |
| External projects and shared libraries | `/home/paul/code`, `platform-contracts`, `workspace-governance` | No hosted-run envelope found in current shared contracts | Shared result is summary-only and has no authenticated run provenance | Forge owns hosted Gate association | **keep local**; no sibling source dependency |

## User Experience and Interface Impact

This package has no operator workflow: it adds a machine-readable contract and Rust validation used by a future workflow uploader and Forge ingest endpoint. `UI/UX: N/A` because the contract itself has no screen, human action, or stateful workflow. The consuming `forge-hosted-ci-evidence-ingest` package must expose receipt status in the authenticated project workbench and API, with permission, stale, rejected, accepted, loading and recovery states and browser verification.

## BFS Impact Map

- **Inputs:** untrusted hosted workflow JSON, existing Gate evidence, provider run metadata.
- **Core:** typed Rust model, exact contract version, strict field and cross-field validation.
- **Persistence:** none; no database migration or filesystem write.
- **Transport:** none; no upload, callback, endpoint, token, or GitHub API call.
- **Consumers:** future generated workflow exporter and authenticated Forge ingest package.
- **Compatibility:** existing Gate evidence bytes remain unchanged and embedded as a typed object.
- **Security:** run metadata is an assertion until the future endpoint authenticates its caller and checks registered repository mapping. `run_url` is descriptive metadata and never proof. Secrets, tokens and raw logs are forbidden.
- **Verification:** positive, malformed, unsupported-version, wrong-revision, rehearsal, and contradictory-aggregate fixtures.

## Capabilities

- `hosted-ci-evidence-envelope`

## Non-goals

- Uploading evidence from GitHub Actions.
- Authenticating GitHub identity or establishing repository-to-project ownership.
- Ingest API, database persistence, event/timeline views, release or delivery approval.
- Changing generated project workflows or Gate execution.
