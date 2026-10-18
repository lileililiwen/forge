# Workspace Governance Roadmap

> **Portfolio snapshot (last reviewed 2026-10-01):** Phases 1–4 were recorded as shipped. Packages 1
> (`registry-and-declaration-integrity`), 2 (`shared-rules-single-source`),
> 3 (`portfolio-vocabulary-normalization`), 8 (`governance-front-door`),
> 9 (`post-mvp-readiness`), 10 (`toolchain-decision-register`),
> 11 (`shared-postgres-runtime-profile`), and 12
> (`standalone-project-boundary`) are all archived in
> `openspec/changes/archive/` and indexed in `docs/package-backlog.md`.
> Packages 4–7 are specified implementation-ready in their owning
> repositories (`driftwatchdog`, `dotnet-platform-libs`,
> `rust-platform-libs`, and `platform-contracts` respectively).
> Remaining follow-ups: re-promote the pilot projects at their current
> `HEAD`, replace the pilot-context `release_evidence` placeholders, and
> replace the 9 scaffold `HANDOFF.md` bodies.

**Local governance update (2026-10-12):** This repository has no active
OpenSpec changes. Its local workflow permits one active change at a time and
requires it to be archived before a subsequent change is created.
`HANDOFF.md` records recent completion evidence; the archive is the source
for completed contracts. The wider portfolio snapshot above has not been
re-audited here.

## Phase 1 — Registry and mandatory checks

- Register the existing project portfolio.
- Define project metadata and governance profiles.
- Validate required documents, OpenSpec names, CI, gate ownership, and deployment declarations.
- Generate a Jenkins-safe deployment manifest.

## Phase 2 — Adoption

- Add `.project.json` to every active product and shared project.
- Add the shared `driftwatchdog` gate declaration to each adopted project.
- Add repository-local `verify` commands and CI parity.
- Classify learning, scratch, archived, and deployment-only directories with lighter profiles.
- Add the `quality` block (see `docs/product-code-quality.md`) and bind the
  product-code quality checker in each adopted product's `.ai-gate/gate.yaml`.
  Missing migration metadata stays `REVIEW_REQUIRED`.

## Phase 3 — Control-plane integration

- Make `forge` consume `projects.json` and checker reports.
- Import OpenSpec status and `HANDOFF.md` pointers.
- Track maturity and evidence without claiming runtime completion from planning artifacts.
- Adopt the `capabilities` and `release_evidence` blocks (see
  `docs/capabilities.md`) so Forge observations can carry the same vocabulary
  as the workspace audit.
- Ship the `forge-governance-adapter` shim (see
  `openspec/specs/forge-governance-adapter/spec.md`) so a Forge control-plane
  can translate the per-project audit into a v0.1.0 governance observation
  without this repository gaining any UI or export surface of its own.

## Phase 4 — Deployment coordination

- Make `jenkins-local` consume `deployment/projects.json`.
- Require governance PASS before project job generation or deployment.
- Add per-project rollback and release evidence links.
- Record every `deployable: true` declaration in `deployment/promotions.json`
  via `scripts/promote_project.py`; the audit refuses hand-edited flags
  (`DEPLOYMENT_PROMOTION`) and surfaces stale revisions as a triage warning
  (`DEPLOYMENT_PROMOTION_STALE`).

## Proposed lower-priority runtime-template follow-up

> **Status (2026-10-01):** The follow-up is shipped as
> package 11 (`shared-postgres-runtime-profile`); see
> `openspec/changes/archive/2026-10-01-shared-postgres-runtime-profile/`
> and the canonical spec at
> `openspec/specs/external-postgres-runtime-profile/spec.md`. The four
> `*-postgres-external` families (`dotnet-web-postgres-external`,
> `rust-web-postgres-external`, `python-web-postgres-external`,
> `worker-postgres-external`) connect an app container to a
> supplied PostgreSQL service without starting a database container,
> image, volume, or provisioning command. The six per-project
> container-backed templates remain the default and are byte-identical
> to their pre-change bytes. Each project must still use its own
> database and role; endpoint and credentials are environment-specific
> and no Mac hostname is a product default.

## Rules drift report — now a ratchet gate

> **Status (2026-10-03):** SHIPPED as
> `openspec/changes/archive/2026-10-03-rules-drift-gate-wiring/`, tracked as
> decision `D5` in `decisions.json`.
>
> `scripts/rules_drift.py --check` remains the **detailed report**: it names
> every clause a repository does not visibly carry and rewrites nothing. What
> changed is that the audit now gates on the **delta**. `workspace_check.py`
> emits ERROR `RULES_DRIFT_NEW` for any finding whose
> `(project, file, label)` is absent from `rules-drift-baseline.json`, and
> WARNING `RULES_DRIFT_BASELINE_STALE` when a baselined entry is no longer
> reported and the baseline can be tightened.
>
> `rules-drift-baseline.json` records the **140** clause-level gaps present
> today across 72 repositories. The other 8 of the 148 total findings are
> `HANDOFF_SCAFFOLD`, which is reported by a different path and is not
> ratcheted. The baseline is a data file so it is reviewable as a diff, and it
> fails closed: missing, unparseable, non-object, empty or malformed is a hard
> error, never an empty-set fallback, because an empty baseline would report
> every existing gap as new.
>
> Measured on landing: the fleet audit stayed at `errors=211 warnings=17` with
> **zero** `RULES_DRIFT_*` findings. New drift now blocks; known debt does not;
> and the baseline can only tighten.
>
> A repository's legitimate escape is the per-file
> `<!-- workspace-governance:rules-customized -->` marker that
> `rules_drift.OPT_OUT` already honours, in that repository. There is no
> environment variable, no skip flag, and no exemption registry.

## Explicit non-goals

- No universal application framework.
- No central product database.
- No credential storage.
- No replacement for Jenkins, DriftWatchdog, Sisyphusfy, Ariadex, or language-specific toolchains.
- No automatic deployment merely because a project is registered.
