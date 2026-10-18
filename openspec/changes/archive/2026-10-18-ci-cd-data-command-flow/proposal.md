# Proposal: Make Forge CI/CD flows understandable

## Why

Forge has distinct Gate, release, generated-project CI, and staged-delivery capabilities. Their commands, evidence stores, and handoff boundaries are spread across code and specifications. Maintainers need one current map to understand what runs, where results are stored, and which steps actually depend on previous evidence.

## What Changes

- Add a maintained architecture guide describing command and data flows for generated project CI/pre-commit, Forge's own CI/release, project release preparation, and provider-backed delivery.
- Inventory the records and ownership boundaries, and identify verified gaps between flows without claiming unimplemented automation.
- Link the guide from the existing developer documentation entry points and correct the README statement that obscured the existing admin Gate routes.

## Package Boundary and Split Assessment

This is one documentation outcome: give Forge maintainers a source-linked model of the current CI/CD paths and a ranked gap list, while correcting one stale README description discovered during the audit. It does not change runtime behavior, persistence, schemas, workflows, templates, providers, or UI. The distinct product implementation gaps have separate owners and acceptance oracles and are recorded as follow-up candidates in the guide, not implemented here.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `ci-cd-data-command-flow` | Current, evidence-linked CI/CD model and gap inventory | Forge docs, Markdown | Documentation links to current CLI, Gate, release, delivery, CI, and standard-pack implementations | Existing canonical specs and source | Link/reference review, strict OpenSpec validation, diff checks |

## Sibling and Shared Architecture Reconnaissance

This documentation describes Forge-owned capabilities; it does not add a reusable runtime or cross-project contract. Forge owns Gate and project templates. GitHub Actions owns hosted job execution and GitHub Releases; configured publish/deploy providers own their external operations. No sibling source changes or shared extraction are needed.

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge Gate | `src/gate/`, `openspec/specs/gate-runtime-evidence/spec.md` | Shared baseline, extensions, evidence, projections | None for a documentation change | Forge release | keep local |
| Forge release and delivery | `src/release/`, `src/delivery/`, canonical specs | Existing separate workflows and provider boundaries | Their evidence is not one unified pipeline record | Forge plus configured providers | keep local |
| Generated project CI | `standards/baseline-service/`, `standards/baseline-client/` | Versioned snapshots and pinned Forge installer | Hosted evidence remains owned by GitHub unless explicitly imported | Forge pack version; GitHub workflow | keep local |

## User Experience and Interface Impact

The actors are Forge maintainers and project operators. The guide is discoverable from the root README and explains command entry points, transitions, evidence locations, failure semantics, and follow-up ordering. It has no interactive screen, responsive behavior, or keyboard flow because the deliverable is a repository document; Markdown rendering and link validity are the applicable presentation checks. UI/UX: N/A for this documentation-only package; no user-facing behavior changes.

## BFS Impact Map

- **Capabilities:** Gate execution/evidence, standard snapshots, Forge CI, release preflight/apply, delivery preflight/stage/promote, workbench read surfaces.
- **Data:** Git revisions, manifests and pack versions, Gate evidence JSON, local SQLite operation journal, release state/check evidence, CI artifacts and provider responses.
- **Callers:** CLI, generated GitHub Actions, pre-commit, admin Gate API/workbench, release engine, delivery handlers.
- **Integrations:** GitHub Actions/Release and configured publish/deploy adapters.
- **Failure/boundaries:** stale or absent Gate evidence, workflow failure, missing pinned Forge release, provider failure, revision mismatch, and explicit production confirmation.
- **Compatibility/security:** Existing source behavior is authoritative; keep credentials out of docs/examples, distinguish local from hosted evidence, label inferences and unverified external settings.
- **Verification:** Source-path review, link checks where available, OpenSpec validation, `git diff --check`, and local Forge Gate per repository policy.

## Capabilities

### CI/CD system model documentation

Maintainers can follow the current command path and the evidence each stage reads or writes, identify which system owns each transition, and prioritize missing handoffs from concrete source evidence.

## Non-goals

- Implementing CI result ingestion, a hosted runner, workflow orchestration, or a new evidence schema.
- Changing Gate, release, delivery, deployment, standard packs, generated projects, portal routes, or provider contracts.
- Claiming branch protection, real external publication, or deployment behavior without live evidence.
- Migrating sibling repositories or changing roadmap implementation scope.
