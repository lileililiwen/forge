# Runtime template library and renderer

## ADDED Requirements

### Requirement: Provide versioned runtime template types

Workspace Governance MUST provide a machine-readable registry of supported
runtime template types, required variables, generated files, and template
versions.

#### Scenario: List supported types

- **WHEN** an operator runs `runtime_templates.py list`
- **THEN** the command returns the six supported types with their versions and
  required variable names

#### Scenario: Template registry is incomplete

- **WHEN** a registry entry references a missing template file or an undeclared
  generated file
- **THEN** validation fails with the type and missing path; no renderer run is
  considered valid

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
using validated project variables.

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

### Requirement: Refuse unsafe or ambiguous writes

The renderer MUST fail closed for unknown types, missing variables, unresolved
placeholders, secrets, absolute paths, path traversal, invalid declarations,
and existing target files without explicit per-file replacement.

#### Scenario: Existing file

- **WHEN** `Dockerfile` or `compose.yaml` already exists and is not named by
  `--replace`
- **THEN** the renderer returns `TARGET_EXISTS` and performs no partial write

#### Scenario: Secret or absolute-path substitution

- **WHEN** a variable contains a secret-looking key or an absolute/path-traversal
  value
- **THEN** the renderer returns `RENDER_REJECTED` and writes nothing

### Requirement: Record configuration evidence without promotion

The renderer MUST record template type, version, rendered files, and relative
Compose path in `.project.json` while preserving unrelated declaration fields
and keeping deployment promotion separate.

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
