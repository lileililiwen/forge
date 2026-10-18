# Proposal: Runtime templates for an external shared PostgreSQL service

## Why

Workspace runtime templates currently start a separate PostgreSQL container and persistent volume for each project. The user currently runs one PostgreSQL service on a capacity-limited Mac and wants projects to reuse it, while retaining the option to move projects to standalone containers/databases on cloud hosts later.

## What Changes

- Add versioned external-PostgreSQL runtime templates for .NET, Rust, Python web apps, and Python workers.
- Render app containers that connect to a supplied PostgreSQL endpoint without creating a database-server service or data volume.
- Generate a blank `.env.example`, ignore local `.env` secrets safely, and document one database/user per project.

## Package Boundary and Split Assessment

This is an optional runtime-template variant, not a workspace-wide mandate or a PostgreSQL server manager. It shares the renderer, registry, generated-file refusal behavior, and compose-check oracle with the existing templates; splitting documentation/configuration from template rendering would leave an unusable partial profile.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `shared-postgres-runtime-profile` | Render app containers that use an existing PostgreSQL service | Workspace Governance, Python + Compose templates | four `*-postgres-external` template IDs | runtime template registry/renderer | rendered fixture and Compose config checks |
| Forge and OpenPanel adoption | Select the external service profile for a project's runtime | Forge/OpenPanel, Rust | existing project/runtime declarations and deployment adapters | this template contract | consumer conformance in each owner repo |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Existing runtime templates | `templates/runtime/{dotnet,rust,python}-web-postgres`, `scripts/runtime_templates.py` | versioned rendering, Compose checks, project metadata preservation | all current DB variants start their own `db` service and volume | Workspace Governance | **extend shared owner** |
| Jenkins-local | `shared-infra/compose.yml`, deployment manifest/ports | shared infrastructure can run on the current host | deployment executor must consume project-owned Compose and must not own application DB semantics | Jenkins-local | **adapt through a generic adapter** |
| OpenPanel | database/site runtime management | can later provision isolated host services | current OpenPanel DB context provisions MySQL, not the user's external PostgreSQL service | OpenPanel | **defer** |
| Projects using PostgreSQL | existing generated runtime and project-specific Compose | concrete DSNs and health patterns | local credentials and hostnames are not reusable template constants | each project | **adopt** only as contract fixtures |

## BFS Impact Map

- **Inputs:** existing required project/runtime values plus endpoint host, port, database, user, and a secret variable name; no password passed as a template-render argument.
- **Generated files:** Dockerfile and Compose app service, `.env.example`, deployment README, Docker ignore rules, and a safely amended root `.gitignore` rule for `.env`.
- **Runtime:** no PostgreSQL container or volume is created. The app receives connection environment values from a user-created ignored `.env` or host secret source.
- **Isolation:** each project must use its own database and DB role on the shared server; this package does not provision databases, roles, quotas, backup policies, or credentials.
- **Portability:** host is supplied per environment. `host.docker.internal` may be used in the user's current Mac profile, but is not baked into any template or project contract.
- **Verification:** four template renders, no database service/volume in output, secret ignore checks, valid Compose, and renderer non-overwrite/idempotence tests.
- **Priority:** lower than Forge project creation/publish and shared end-user auth; this is an optional capacity-saving profile.

## Capabilities

- **Modified:** `runtime-template-renderer`.
- **New:** `external-postgres-runtime-profile`.

## Non-goals

- No deployment of PostgreSQL, host discovery, database/user creation, credential generation, pooling proxy, or Postgres backup management.
- No requirement to share a database across applications or use one database role for multiple projects.
- No Mac-only hostname, path, Docker Desktop, or cloud-provider behavior in the product contract.
- No change to the existing per-project PostgreSQL-container templates or current default.
