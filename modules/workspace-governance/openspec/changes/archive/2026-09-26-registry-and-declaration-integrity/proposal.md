# Proposal: Restore declaration and registry integrity across the workspace

## Why

The portfolio's strict audit currently fails while its maturity report claims
a full pass, so the two governance views disagree and neither can be used as a
trustworthy base for gate evidence or platform adoption.

Observed on 2026-09-26:

- `python3 scripts/workspace_check.py --root .. --audit` → 14 ERROR, 18 WARNING.
- `python3 scripts/maturity_audit.py --root ..` → "No findings — all projects
  pass basic checks."

Concrete defects:

- 11 directories ship a `.project.json` the registry does not list:
  `campusMesh`, `careBridge`, `citylens`, `mewo`, `openTeacherKit`,
  `platform-contracts`, `retakeKit`, `rust-platform-libs`,
  `therapist-commons`, `trailCrew`, `vowra` (`DISCOVERED_UNREGISTERED`).
- 3 registry entries are `adoption: adopted` with no `.project.json`:
  `dharmatlas`, `hookit`, `lileililiwen-site` (`DECLARATION_MISSING`).
- 6 declarations fail identity or format rules: `campusMesh`, `careBridge`,
  `retakeKit`, `trailCrew` (`DECLARATION_ID`); `citylens`, `retakeKit`,
  `trailCrew`, `vowra` (`DECLARATION_SCHEMA`).
- `vowra` uses a legacy `verification_command` key with no
  `verification.command` (`VERIFICATION_COMMAND`).
- 7 promotions are stale against current `HEAD`
  (`DEPLOYMENT_PROMOTION_STALE`).

Registry and declarations are the input to the gate runtime, the capability
manifest, the Jenkins manifest and the Forge adapter, so every downstream
governance claim currently rests on inputs that fail their own validation.

## What Changes

- Register (or explicitly classify) every discovered declaration so an active
  product cannot be `DISCOVERED_UNREGISTERED`.
- Give every adopted entry a schema-valid `.project.json` whose `id` equals the
  registry `id`, with `schema_version: 1` and the required `verification` and
  `deployment` blocks.
- Derive the canonical `id` from the directory name at authoring time in
  `init_project.py`, permitting a recorded alias only when the directory name
  is not a valid identifier.
- Make `workspace_check.py` the single authoritative audit by having
  `maturity_audit.py` report its counts instead of recomputing a second,
  disagreeing result.
- Re-record the 7 stale promotions at current `HEAD`, or triage and name them.
- Add unit tests and synthetic fixtures for every rule this change touches.

## Package Boundary and Split Assessment

**Single outcome:** every discovered or adopted project has exactly one
schema-valid declaration and one registry identity, and the audit agrees with
itself.

**Included:** `projects.json`, `scripts/workspace_check.py`,
`scripts/init_project.py`, `scripts/maturity_audit.py`, `tests/`, and the
enumerated declaration/registry remediations in sibling directories.

**Excluded:** gate execution and evidence production; `.ai-rules` and HANDOFF
template consolidation; kind/profile vocabulary; every platform library; every
product source file.

**Split signals considered:** this package pairs a checker change with sibling
declaration edits, but both share one owner (`workspace-governance`), one
contract (`.project.json` v1 + registry), one state model
(discovered/registered/adopted) and one acceptance oracle (audit codes → 0).
Splitting "tighten the checker" from "fix the data" would leave an unusable
intermediate where a stricter checker fails on unchanged data. This is the
smallest independently verifiable unit.

**Dependencies:** none. Packages `shared-rules-single-source`,
`portfolio-vocabulary-normalization` and `gate-run-evidence` depend on it.

**Package map (full initiative):**

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `registry-and-declaration-integrity` | one schema-valid identity per project; audit agrees with itself | workspace-governance / Python 3.11 | `.project.json` v1 + `projects.json` | — | `workspace_check.py --root .. --audit` → 0 ERROR for codes `DECLARATION_*`, `VERIFICATION_COMMAND`, `DISCOVERED_UNREGISTERED`, `DEPLOYMENT_PROMOTION_STALE` |
| `shared-rules-single-source` | `.ai-rules/*` and stub HANDOFF derive from one template set | workspace-governance / Python 3.11 + Markdown | `templates/project/*` + drift checker | 1 | drift check reports 0 divergent files; `crossalheart` `REQUIRED_FILE` cleared |
| `portfolio-vocabulary-normalization` | one canonical `kind`/`profile` set and one vocabulary source | workspace-governance / Python + JSON | `profiles/*.json`, vocabulary module | 1 | no `kind`/`profile` outside the canonical set; audit clean |
| `gate-run-evidence` | real driftwatchdog runs recorded as evidence that populates `release_evidence` | driftwatchdog (executor, Rust) + workspace-governance (vocabulary) | gate-run record / evidence schema | 1, 3 | a gate run emits a schema-valid evidence record the audit reflects |
| `platform-adoption-dotnet` | `dotnet-platform-libs` consumable through one documented path | dotnet-platform-libs / C# | `build/Platform.Consumer.props`, feed, version automation | — (parallel) | `scripts/conformance.sh` packs and consumes; a consumer builds from the feed |
| `platform-adoption-rust` | `rust-platform-libs` ships a lint/MSRV baseline, consumer recipe and conformance harness | rust-platform-libs / Rust | toolchain + lints, consumer recipe | — (parallel) | conformance harness packs and consumes a real consumer crate |
| `contracts-single-source` | `platform-contracts` is the sole contract source consumed by both platforms | platform-contracts / JSON Schema + Python | schema registry + fixture parity | 5, 6 | fixture parity covers 9/9 families in both consumers |
| `governance-front-door` | one auto-triggering skill routes to per-concern references | workspace-governance / Markdown + skill | skill + references | 1, 2, 4 | skill loads and every referenced command and path exists |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| workspace-governance (this repo) | `projects.json`; `schemas/project.schema.json`; `scripts/workspace_check.py:run()`; `scripts/init_project.py` | registry, declaration schema, checker, authoring script | none — it already owns the contract | workspace-governance; released with this repo | **extend shared owner** |
| forge | `scripts/forge_governance_adapter.py`; README `forge governance use workspace-governance` | read-only adapter already translates audit → observation | none; adapter contract is v0.1.0 | forge owns the consumer; no change here | **adapt through a generic adapter** |
| driftwatchdog | `.project.json` `verification.gate_runtime: driftwatchdog`; `driftwatchdog/src/gate/*` | gate executor consumes declarations | gate execution is out of scope for this package | driftwatchdog; separate package `gate-run-evidence` | **adopt (unchanged here)** |
| jenkins-local | `deployment/projects.json`; `scripts/jenkins_manifest.py` | deployable manifest consumer | consumes only `deployable: true`; unaffected by these fixes | jenkins-local; no change | **keep local (no change)** |
| platform-contracts | `platform-contracts/schemas/registry.json` (9 runtime families) | language-neutral JSON Schema + validator | no `project-registry` or `project-declaration` family exists; governance metadata is not a runtime contract | platform-contracts; not the owner of governance metadata | **keep local** (evidence: no matching family) |
| sisyphusfy | `sisyphusfy/.project.json` | agent session supervision | no registry/declaration surface | sisyphusfy | **not applicable** |
| dotnet-platform-libs | `dotnet-platform-libs/.project.json`; `build/Platform.Consumer.props` | consumer bootstrap | different language and lifecycle; has its own package | dotnet-platform-libs | **keep local to `platform-adoption-dotnet`** |
| rust-platform-libs | `rust-platform-libs/.project.json`; `crates/*` | Rust platform crates | different language and lifecycle; has its own package | rust-platform-libs | **keep local to `platform-adoption-rust`** |
| alltools-platform | `alltools-platform/README.md` ("not the portfolio-wide governance authority") | none reusable portfolio-wide | is a product, not infrastructure | alltools-platform | **not applicable** |

No sibling implements registry or declaration validation, so no extraction is
authorized. The selected owner is the existing shared owner:
`workspace-governance`.

## BFS Impact Map

- **Capabilities:** registry integrity (`projects.json`), declaration validation
  (`.project.json`), audit reporting (`workspace_check.py`,
  `maturity_audit.py`).
- **Users and flows:** the governance maintainer and any agent running
  `workspace_check`/`auto_gate`; Forge via `forge_governance_adapter.py`;
  jenkins-local via `deployment/projects.json`.
- **Contracts and data:** `.project.json` schema v1 (unchanged property set);
  registry entry `{id, path, adoption}`; audit finding
  `{level, code, project, message}`.
- **Integrations and configuration:** `.ai-gate/gate.yaml` is read but not
  modified by this change.
- **Callers:** `forge_governance_adapter.py`, `auto_gate.py`,
  `jenkins_manifest.py`, `maturity_audit.py`.
- **Failure and boundary behavior:** unregistered declaration → ERROR; adopted
  without declaration → ERROR; identity mismatch → ERROR; stale promotion →
  WARNING.
- **Tests:** one new unittest case per rule, run against synthetic workspaces
  under a temp root.
- **Dependencies:** Python 3.11 standard library only; the `openspec` CLI for
  strict validation.
- **Compatibility, privacy, security:** existing valid declarations are
  untouched; only the enumerated defective declarations are edited; the
  schema's `id` pattern is retained, not relaxed. Registry and declarations are
  public metadata and the secret scan must remain clean.

## Capabilities

- `registry-integrity`.

## Non-goals

- No gate execution and no gate-run evidence records (`gate-run-evidence`).
- No `.ai-rules`/HANDOFF template consolidation (`shared-rules-single-source`);
  the `crossalheart` `REQUIRED_FILE` error stays open and is deferred there.
- No kind/profile vocabulary change (`portfolio-vocabulary-normalization`).
- No edits to `dotnet-platform-libs`, `rust-platform-libs`, `platform-contracts`
  or any product source.
- No promotion of any project to `deployable: true`.
