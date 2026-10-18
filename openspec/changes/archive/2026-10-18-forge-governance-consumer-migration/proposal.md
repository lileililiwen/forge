# Proposal: hide governance internals behind Forge

## Why

Forge now owns Workspace Governance source, but its concrete tooling projects
still name the old owner in Gate manifests and checker scripts. They call
`../workspace-governance/scripts/workspace_check.py` or resolve a sibling
checkout to find `check_concern.py`. Their `rule_pack` also exposes the
implementation project identity. These paths defeat the consolidation: the
products remain coupled to a checkout and users must understand an internal
module to run the Forge-provided checks.

## What Changes

- Add a stable `forge concern check <id> [PROJECT_ROOT]` CLI facade for the
  existing read-only generic concern checker.
- Bundle that checker as a versioned Forge resource in release archives and
  resolve it from the source checkout during development.
- Migrate the consolidated Driftwatchdog and `cargo-*` projects' Gate identity
  and checker calls to Forge-owned names/commands; remove product-local calls
  to Workspace Governance paths and its per-project portfolio audit.
- Keep portfolio audit and project registration behind Forge's own management
  surface; product-local Gate remains focused on source, build, test and
  declared concern checks.
- Repoint Forge's internal contract sync source to the in-tree owner and
  remove Workspace Governance naming from current product-facing usage docs.
- Leave external sibling repositories unchanged in this package. Add a
  dependency-ordered portability migration using the versioned Forge templates
  and each sibling's HANDOFF/Git workflow.

## Package Boundary and Split Assessment

The outcome is that a concrete project uses Forge's stable CLI and Gate
contract without knowing Workspace Governance's name, checkout path or Python
module layout. The owner is this Forge Rust workspace and its consolidated
tooling crates. The facade and its in-tree consumers must ship together:
changing their manifests to a command that does not yet exist would leave an
unusable intermediate commit. The included Python implementation remains
internal source owned by Forge and is bundled as a resource for the CLI.

The external sibling fleet has a different release boundary: those projects
must consume a pinned Forge release and have their CI/pre-commit metadata
upgraded with conflict-safe generated assets. That package is independently
verifiable and depends on this stable CLI/resource contract, so it is split as
`forge-governance-consumer-portability`. The module source-size conformance
work is independent of consumer decoupling and remains required before origin
retirement, not a precondition to the facade.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-governance-consumer-migration` | Forge-owned products invoke checks through Forge without Workspace Governance path or identity coupling | Forge CLI Rust, `modules/workspace-governance` Python resource, consolidated Rust tool crates | `forge concern check` CLI; `forge-baseline/1.0.0`; packaged check resource | completed in-tree import | source and manifest audit finds no active product-path reference; facade works from source and install archive |
| `forge-governance-consumer-portability` | external sibling projects consume pinned Forge checks/templates without a Workspace Governance checkout | each sibling's native language/toolchain | generated/versioned Forge assets and its `.project.json` adoption record | this migration | each sibling's local Gate, hook and CI pass without the Workspace Governance directory |
| `forge-workspace-governance-gate-conformance` | module Gate measures Python scripts and enforces the 1,000-line limit | Forge Python module and Driftwatchdog runtime | additive `source_size.include` | completed in-tree import | module Gate evaluates real candidates and reports no file over 1,000 lines |
| `forge-origin-repository-retirement` | make eligible source origins read-only/archive after all consumers are verified | source repository owners | explicit remote operation | all source, consumer and Gate conformance work | verified migration ledger plus explicit operator action |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge concern resource | `modules/workspace-governance/scripts/check_concern.py`, `CHECKERS`, `main` | Existing read-only accessibility, privacy, provenance, fixture and schema checkers with JSON output and exit codes | Its implementation path currently leaks to each caller | Forge owns the CLI and packages the resource under `share/forge`; Workspace Governance remains the source module | **extend shared owner** with a Forge-owned command facade |
| Forge resource packaging | `src/cli/runtime_template.rs`, `scripts/package.sh`, `scripts/install.sh` | Existing source-versus-installed resource resolution and checksum-verified archive/install flow | Only runtime templates are currently installed as resources | Forge binary/release workflow | **adopt** the current resource-bundle pattern |
| Driftwatchdog Gate | `crates/driftwatchdog/src/gate/manifest.rs`, `crates/driftwatchdog/.ai-gate/gate.yaml` | Gate consumes versioned baseline and declarative optional checks | Product Gate manifests still name `workspace-governance@1` and bind workspace-specific audit commands | Forge owns the in-tree baseline; Driftwatchdog retains Gate execution | **extend shared owner** by using the Forge baseline and generic concern command |
| Standard templates | `standards/baseline-client/`, CI/pre-commit template contracts | Existing generated Forge Gate workflow/hook and receipt ownership | Existing external consumers still have older sibling scripts and metadata | Forge template release, adopted by each project | **adapt through a generic adapter** in the dependent portability package |

## User Experience and Interface Impact

Actors are product maintainers running Gate locally or in CI and Forge
maintainers operating the workspace. A product maintainer discovers one
command family (`forge gate`, `forge concern check`) and sees the existing
human/JSON check result; the CLI does not name Workspace Governance or require
the user to locate Python files. The current generic checker output, finding
paths, `NOT_APPLICABLE` state and exit codes stay intact. Gate failures and
missing Forge/resource installations remain explicit with remediation text.

This is a CLI-only workflow. No browser route, screen, responsive layout or
authentication state changes. Accessibility and keyboard behavior do not
apply to this command-line surface; machine-readable JSON remains available.

## BFS Impact Map

| Area | Impact and invariant |
|---|---|
| Product workflow | Consolidated crates continue to run their native build/test checks and use Forge for Gate and shared concerns |
| CLI contract | New `forge concern check <id> [root]`; existing `forge check <target>` checker-protocol command remains unchanged |
| Configuration | Concrete crate manifests use `forge-baseline/1.0.0`; no `workspace-governance@1`, sibling path or module script path remains in live product config |
| Resource/distribution | Source checkout resolves the canonical module script; release/install bundle includes it at a documented Forge-owned resource path; checksum/smoke validate it |
| Callers | Four consolidated tooling crates' concern wrappers and Gate manifests migrate together; Forge module's own self-check remains runnable |
| Governance data | Workspace registry, schemas and audit stay internal to Forge; product-local checks do not replace workspace-level registration/audit |
| Failure boundaries | Unknown concern, missing resource, missing Python runtime, missing project root and checker exit statuses are surfaced distinctly; finding exit status is preserved |
| Compatibility | `forge check` behavior, Python concern JSON schema, existing root/hook templates and project declarations remain compatible |
| Security/privacy | Checker remains local/read-only; no network, path traversal, source upload, or automatic sibling discovery is introduced |
| Verification | CLI/source/install resource contracts, consolidated crate Gate manifests, module self-check, package smoke and Forge Gate |

## Capabilities

- `forge-governance-consumer-migration`

## Non-goals

- Migrating external sibling repositories in this change.
- Removing Workspace Governance from Forge's internal project registry,
  canonical history or maintainer-facing architecture documentation.
- Deleting the workspace audit, schemas or checkers.
- Replacing native project builds/tests with Forge or changing product source
  language/package identity.
- Archiving the Workspace Governance source origin or performing any remote
  write.
- Completing the source-size split of the four large governance scripts; that
  remains a separate Forge-owned package before origin retirement.
