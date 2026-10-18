# runtime-template-renderer Specification

## Purpose

The workspace has many product repositories with different runtimes but no reusable, source-backed way to create Dockerfiles and Compose stacks. Copying a generic sample produces invalid or misleading deployment configuration.
## Requirements
### Requirement: Provide versioned runtime template types

Workspace Governance MUST provide a machine-readable registry of supported
runtime template types, required variables, generated files, and template
versions.

#### Scenario: List supported types

- **WHEN** an operator runs `runtime_templates.py list`
- **THEN** the command returns the supported types with their versions and
  required variable names; the list includes both `container`-backed
  PostgreSQL templates and `external` PostgreSQL templates
  (`*-postgres-external`)

#### Scenario: Template registry is incomplete

- **WHEN** a registry entry references a missing template file or an undeclared
  generated file
- **THEN** validation fails with the type and missing path; no renderer run is
  considered valid

#### Scenario: External template declares `.env.example`

- **WHEN** a registry entry is an `external` PostgreSQL template
- **THEN** its `files` list includes `.env.example`; its `required_variables`
  include `database_name`, `db_user`, and `db_host`; and `db_port` is listed
  in `optional_variables` (default `5432`)

### Requirement: Inspect without mutating

The renderer MUST inspect a target repository's manifests, declaration, and
existing runtime files without writing files.

#### Scenario: Concrete runtime evidence

- **WHEN** inspection finds one supported runtime shape
- **THEN** it returns that type as a suggestion with the evidence paths and
  required variables

#### Scenario: Ambiguous runtime evidence

- **WHEN** inspection finds multiple plausible runtime shapes or insufficient
  evidence
- **THEN** it returns `TYPE_REQUIRED` with explicit choices and writes nothing

### Requirement: Render repository-relative runtime files

The renderer MUST render the selected versioned template into the target root,
including Dockerfile, Compose, `.dockerignore`, and deployment documentation,
using validated project variables. External PostgreSQL templates additionally
render a `.env.example` and amend the root `.gitignore`.

#### Scenario: Render a .NET web project

- **WHEN** an operator selects `dotnet-web-postgres` with API project, port,
  health path, and database variables
- **THEN** the renderer creates valid repository-relative runtime files and
  records the template metadata in `.project.json`

#### Scenario: Render a Rust, Python, Node, static, or worker project

- **WHEN** an operator selects any other supported type with its required
  variables
- **THEN** the corresponding type-specific Dockerfile and Compose contract is
  rendered without requiring source changes outside generated files

#### Scenario: Render an external PostgreSQL template

- **WHEN** an operator selects a `*-postgres-external` template with the
  required `database_name`, `db_user`, `db_host` (and optionally `db_port`)
  variables
- **THEN** the renderer writes a `.env.example` with non-secret placeholders
  and a blank password, amends the root `.gitignore` to ignore `.env`, and
  records `database_source: external` in the project declaration. The
  generated Compose has no `db` service, no PostgreSQL image, no PostgreSQL
  volume, and no database provisioning command

### Requirement: Refuse unsafe or ambiguous writes

The renderer MUST fail closed for unknown types, missing variables, unresolved
placeholders, secrets, absolute paths, path traversal, invalid declarations,
existing target files without explicit per-file replacement, and unsafe
`.gitignore` state for external PostgreSQL templates.

#### Scenario: Existing file

- **WHEN** `Dockerfile` or `compose.yaml` already exists and is not named by
  `--replace`
- **THEN** the renderer returns `TARGET_EXISTS` and performs no partial write

#### Scenario: Secret or absolute-path substitution

- **WHEN** a variable contains a secret-looking key or an absolute/path-traversal
  value
- **THEN** the renderer returns `RENDER_REJECTED` and writes nothing

#### Scenario: External template with conflicting `.gitignore`

- **WHEN** the operator selects an external PostgreSQL template and the root
  `.gitignore` already contains a `!.env*` unignore rule
- **THEN** the renderer returns `SECRET_IGNORE_CONFLICT` and writes nothing
  to the target project

#### Scenario: External template idempotent `.gitignore`

- **WHEN** the operator re-renders an external PostgreSQL template and the
  root `.gitignore` already ignores `.env`
- **THEN** the renderer does not duplicate the entry, does not rewrite
  unrelated bytes, and reports success

### Requirement: Record configuration evidence without promotion

The renderer MUST record template type, version, rendered files, and relative
Compose path in `.project.json` while preserving unrelated declaration fields
and keeping deployment promotion separate. For external PostgreSQL templates,
the renderer additionally records `database_source: external`.

#### Scenario: Existing deployable declaration

- **WHEN** a project already has `deployment.deployable = true`
- **THEN** the renderer preserves that value and updates only template metadata

#### Scenario: Non-deployable declaration

- **WHEN** a non-deployable project is rendered
- **THEN** the renderer records `state = configured` but does not set
  `deployment.deployable = true`

### Requirement: Validate generated contracts

The renderer MUST provide a check operation that validates registry integrity,
rendered placeholders and paths, declaration schema, and Docker Compose syntax
when the Docker Compose executable is available.

#### Scenario: Compose tool unavailable

- **WHEN** `check` cannot execute Docker Compose
- **THEN** it reports `TOOL_UNAVAILABLE` and does not claim verified Compose
  evidence

#### Scenario: Generated Compose is malformed

- **WHEN** Docker Compose rejects the rendered file
- **THEN** the check fails with the command output and the target type

#### Scenario: External template Compose contract

- **WHEN** `check` validates an external PostgreSQL template render
- **THEN** the rendered Compose has exactly one application service, no
  `db` service, no `volumes:` block, and the `PGHOST`/`PGUSER`/`PGDATABASE`
  /`PGPASSWORD` environment values are read from the runtime environment
  (not baked in)
