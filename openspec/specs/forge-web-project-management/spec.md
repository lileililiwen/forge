# forge-web-project-management Specification

## Purpose
TBD - created by archiving change forge-web-project-management. Update Purpose after archive.
## Requirements
### Requirement: Browser creation and adoption of projects from structured fields

Forge SHALL expose, on the session-gated global admin surface, typed routes
that create (`forge new`), import (`forge import`) and register
(`forge register`) a project by delegating to the same in-process Core
functions the CLI runs (`generate`, `adopt_import` / `inspect_import`,
`Registry::register`). Each route SHALL accept only that command's structured
fields, and SHALL resolve the project directory only from a server-side
configured root joined with a validated single-segment project name. No
request field SHALL be interpreted, now or later, as a filesystem path, binary
path, host, argv vector or shell string. The read-only preview SHALL NOT write
project files, registry rows or an operation journal entry.

#### Scenario: Signed-in operator previews a project creation

- **WHEN** an authenticated admin session submits a project name and its typed command fields
- **THEN** Forge returns a bounded, path-free preview and a `plan_digest` and performs no write

#### Scenario: Browser cannot supply a filesystem location

- **WHEN** a caller passes a path, host, binary name, shell string or argv vector instead of the command's structured `project` field
- **THEN** Forge treats it only as that typed field or refuses it, joins the validated name to its own configured root, and never opens a browser-named path

#### Scenario: Server root is not configured

- **WHEN** the server-side project root is unset, blank or not a directory
- **THEN** Forge refuses with a typed prerequisite error that names the required server configuration and never echoes the root value, and performs no write

#### Scenario: Project name is hostile or path-bearing

- **WHEN** the `project` field contains a path separator, a traversal segment, a percent-encoded path or a non-identifier
- **THEN** Forge returns a typed refusal, echoes no part of the offending input, and runs no Core operation

#### Scenario: Unsigned client attempts a creation

- **WHEN** a request without a valid admin session cookie calls a project-management route
- **THEN** Forge refuses it the same way it refuses the other admin routes and runs no Core operation

### Requirement: Confirm and digest binding on project-management mutations

Forge SHALL require, for every project-management mutation on the admin
surface, `confirm` to be true and a `plan_digest` matching the digest of the
exact structured field set the operator previewed, enforced in the admin layer
independently of any bearer or CLI route. A mutation that is not confirmed, or
whose digest does not match, SHALL be refused with a typed safe error and SHALL
NOT invoke the Core handler's write. Only on a matching digest SHALL Forge
delegate to the same Core function the equivalent CLI command runs. The digest
SHALL be computed over a path-free descriptor so it binds the reviewed fields
and never serializes a machine location.

#### Scenario: Mutation without confirmation

- **WHEN** a signed-in operator submits a creation, import or registration without `confirm` true
- **THEN** Forge returns the preview and digest, performs no write, and does not run the Core mutation

#### Scenario: Mutation with a mismatched digest

- **WHEN** a signed-in operator submits a project-management mutation whose `plan_digest` does not match the reviewed field set
- **THEN** Forge refuses the request, returns a fresh digest, and invokes no Core write

#### Scenario: Confirmed mutation matches the CLI

- **WHEN** a signed-in operator submits a project-management mutation with `confirm` true and a matching `plan_digest`
- **THEN** Forge runs the same Core function the equivalent CLI command runs, journals the operation, and returns its typed result

### Requirement: Honest outcomes and server-side path privacy

Forge SHALL keep every filesystem location server-side: no response from a
project-management route SHALL disclose the configured root, the resolved
destination, a credential or any other absolute filesystem path. When a Core
function fails — an unknown profile, an absent or ambiguous directory, or an
identity collision — Forge SHALL return a typed error, journal a failed
operation, and never present the outcome as success.

#### Scenario: Failed mutation is reported honestly

- **WHEN** a confirmed project-management mutation fails inside its Core function
- **THEN** Forge returns a typed error, records the operation as failed, and does not report success

#### Scenario: No path or credential leaks

- **WHEN** any project-management route returns a success, preview or error
- **THEN** the serialized body contains no absolute filesystem path, no credential and no browser-supplied value echoed back

