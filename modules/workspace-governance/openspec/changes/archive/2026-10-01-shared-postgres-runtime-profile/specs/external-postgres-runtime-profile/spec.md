## ADDED Requirements

### Requirement: Render an external PostgreSQL runtime profile

Workspace Governance SHALL provide versioned runtime templates that connect an application to a supplied PostgreSQL service without starting a database container.

#### Scenario: External service selected

- **WHEN** an operator renders a supported `*-postgres-external` template with non-secret endpoint/database/user values
- **THEN** the generated Compose file contains the application service and no PostgreSQL service, data volume, or database provisioning command

#### Scenario: Existing local template selected

- **WHEN** an operator selects an existing `*-postgres` template
- **THEN** its current per-project PostgreSQL container and volume behavior remains unchanged

### Requirement: Keep database credentials out of generated source

The runtime renderer SHALL keep external PostgreSQL passwords out of template arguments, generated metadata, committed examples, and source-controlled files.

#### Scenario: Render external profile

- **WHEN** the renderer creates an external PostgreSQL profile
- **THEN** it emits a blank `.env.example`, ignores local `.env`, and references runtime environment values for the password

#### Scenario: Ignore conflict

- **WHEN** `.gitignore` cannot safely ignore `.env` or has a conflicting unignore rule
- **THEN** rendering refuses before partially writing the runtime template

### Requirement: Preserve per-project database isolation

External PostgreSQL templates SHALL document and validate the use of a distinct database and least-privilege database role for each project.

#### Scenario: Shared host

- **WHEN** multiple projects connect to one PostgreSQL server
- **THEN** each rendered configuration identifies its own database/user and does not share credentials by default

#### Scenario: Endpoint unavailable

- **WHEN** the external service cannot be reached at runtime
- **THEN** the application reports its existing readiness failure and the template does not start a fallback database or claim provisioning succeeded
