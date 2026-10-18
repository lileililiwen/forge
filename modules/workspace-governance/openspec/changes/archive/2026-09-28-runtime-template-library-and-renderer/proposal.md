# Proposal: Add reusable runtime Docker and Compose templates

## Why

The workspace has many product repositories with different runtimes but no
reusable, source-backed way to create Dockerfiles and Compose stacks. Copying
a generic sample produces invalid or misleading deployment configuration.

## What Changes

- Add a versioned runtime-template library owned by Workspace Governance.
- Support typed templates for `.NET web + PostgreSQL`, `Rust web + PostgreSQL`,
  `Python web + PostgreSQL`, `Node web`, `static site`, and
  `worker + PostgreSQL` projects.
- Add a repository-relative renderer that inspects a target, requires an
  explicit template type for ambiguous projects, renders Dockerfile,
  `compose.yaml`, `.dockerignore`, CI/Gate bindings, and a README contract,
  and refuses unsafe overwrites.
- Record the selected template type and rendered file paths in
  `.project.json` deployment metadata without marking a project deployable.
- Add deterministic template validation and portability tests.

## Package Boundary and Split Assessment

This remains one package because the template schema, renderer, metadata
contract, and validation oracle form one reusable bootstrap capability owned by
the same repository. Jenkins execution and DriftWatchdog enforcement consume
the rendered contract but are not changed by this package. Product repositories
are not modified by this change; later adoption changes can render one target
at a time.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| workspace-governance | `scripts/init_project.py`, `profiles/*.json`, `schemas/project.schema.json` | Bootstrap ownership, profile vocabulary, declaration schema | No runtime template registry or renderer | Workspace Governance | extend shared owner |
| jenkins-local | `project.sh`, `jenkins_manifest.py`, `shared-infra/compose.yml` | Deployment execution, port allocation, shared infrastructure overlays | Must consume project-owned Compose; must not author product files | Jenkins-local release | adapt through a generic adapter |
| driftwatchdog | `templates/deployable/`, `deployable` Gate profile | Gate contract and Compose/CI validation concern IDs | Existing template is intentionally only a scaffold and is not a typed runtime library | DriftWatchdog release | adapt through a generic adapter |
| existing product deployments | `arivio/deploy/docker-compose.yml`, `hookit/docker-compose.example.yml`, `mewo/deploy/compose.staging.yml` | Evidence of concrete .NET/Rust/SQLite/PostgreSQL patterns | Project-specific secrets, paths, and lifecycle commands cannot be copied blindly | Product repositories | keep local; use as compatibility fixtures |

The reusable source of truth is therefore Workspace Governance. Existing
product files are reconnaissance fixtures only; this change does not copy
their secrets, ports, or deployment assumptions into other repositories.

## BFS Impact Map

| Area | Impact |
|---|---|
| Template registry | Add machine-readable type metadata and versioned template directories. |
| Rendering | Add explicit `list`, `inspect`, `render`, and `check` operations. |
| Project declaration | Extend deployment metadata with template identity and rendered-file records. |
| Bootstrap | Keep existing `init_project.py` additive; expose a separate deployment-template command. |
| Jenkins | No execution behavior change; generated `compose_file` remains the handoff. |
| DriftWatchdog | No code change; generated Gate bindings use the existing contract. |
| Security | Reject absolute paths, secrets, path traversal, unsafe overwrite, and unbounded substitutions. |
| Tests | Unit tests for discovery/rendering/schema; fixture tests for every template type; portability checks. |
| Runtime evidence | Rendering proves configuration generation only, not image build, service health, or deployment readiness. |

## Capabilities

- `runtime-template-library`
- `runtime-template-renderer`

## Non-goals

- Automatically deciding that a repository is deployable.
- Running deployment, publishing images, allocating production ports, or
  storing credentials.
- Replacing Jenkins, DriftWatchdog, project-native build systems, or product
  architecture.
- Rendering a template for an ambiguous project without an explicit type and
  required variables.
- Migrating all existing projects in this change.
