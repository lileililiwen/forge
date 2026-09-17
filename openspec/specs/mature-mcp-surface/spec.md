# mature-mcp-surface Specification

## Purpose
TBD - created by archiving change mature-mcp-surface. Update Purpose after archive.
## Requirements
### Requirement: Shared validated MCP operations

Forge SHALL expose stable mature operations through a single MCP server delegating to the same Core contracts used by CLI.

#### Scenario: Shared validated MCP operations success

- **WHEN** an inspect or doctor request is issued via CLI and MCP against the same state
- **THEN** both return equivalent domain outcomes

#### Scenario: Shared validated MCP operations failure

- **WHEN** a tool request has invalid arguments or targets an unauthorized project
- **THEN** the server returns a structured error before Core mutation

#### Scenario: Shared validated MCP operations boundary

- **WHEN** an internal operation has not met its stability and verification criteria
- **THEN** it is absent from the advertised tool list

### Requirement: Mutating tool boundaries

Forge SHALL distinguish read-only tools from mutating and external-write tools and bind execution to the selected project and approved operation.

#### Scenario: Mutating tool boundaries success

- **WHEN** a create-project tool call has valid scoped intent
- **THEN** Core executes the validated creation request and returns its operation result

#### Scenario: Mutating tool boundaries failure

- **WHEN** a model supplies shell syntax in a project name
- **THEN** it is rejected or treated as literal data and never executed as shell code

#### Scenario: Mutating tool boundaries boundary

- **WHEN** deployment or publishing has not been implemented
- **THEN** no working deploy or publish tool is advertised

