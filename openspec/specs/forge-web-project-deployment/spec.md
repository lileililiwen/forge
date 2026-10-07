# forge-web-project-deployment Specification

## Purpose
TBD - created by archiving change forge-web-project-deployment. Update Purpose after archive.
## Requirements
### Requirement: Browser planning of a project deploy

Forge SHALL expose, on the session-gated global admin surface, a read-only
route that plans a deploy for exactly one managed project id by delegating to
the same in-process deploy engine the CLI uses, and SHALL resolve the project
directory, target and adapter binary only from server-side state. The plan SHALL
NOT write project files, invoke the deploy adapter, or record an operation, and
its response SHALL NOT contain an absolute filesystem path, the adapter binary
path, or any credential.

#### Scenario: Signed-in operator plans a deploy

- **WHEN** an authenticated admin session requests the deploy plan for a managed project id
- **THEN** Forge returns a bounded, path-free plan (target, source revision, artifact identity, health check, readiness) and performs no write and no adapter invocation

#### Scenario: Unsigned client requests a plan

- **WHEN** a request without a valid admin session cookie calls the deploy plan route
- **THEN** Forge refuses it the same way it refuses the other admin routes and runs no Core operation

#### Scenario: Project id is unmanaged or path-bearing

- **WHEN** the deploy plan route is called with an unknown project id, or an id containing a filesystem path
- **THEN** Forge returns a typed 404 or 400 respectively, echoes no path, and runs no Core operation

### Requirement: Confirmed digest-bound deploy application

Forge SHALL require, for the admin deploy apply route, `confirm` to be true and
a `plan_digest` matching the digest of the exact project and target the operator
previewed, enforced in the admin layer independently of any bearer route. A
request that is not confirmed SHALL return a preview and digest and SHALL NOT
write or invoke the adapter. A request whose digest does not match SHALL be
refused with a typed error and a fresh digest and SHALL invoke no adapter. Only
on a matching digest SHALL Forge delegate to the same deploy engine the
equivalent CLI command runs, record the run in the existing operations journal,
and return its typed result.

#### Scenario: Apply without confirmation previews only

- **WHEN** a signed-in operator submits a deploy apply without `confirm` true
- **THEN** Forge returns the plan preview and a 64-hex `plan_digest`, writes nothing, and invokes no adapter

#### Scenario: Apply with a mismatched digest is refused

- **WHEN** a signed-in operator submits a deploy apply whose `plan_digest` does not match the reviewed project and target
- **THEN** Forge refuses with a typed error and a fresh digest and invokes no adapter

#### Scenario: Confirmed apply runs the CLI's deploy engine

- **WHEN** a signed-in operator submits a deploy apply with `confirm` true and a matching `plan_digest`
- **THEN** Forge runs the same deploy engine the CLI runs, journals the operation, and returns its typed report

### Requirement: Honest deploy outcomes with server-side credentials

Forge SHALL keep deploy credentials, the adapter binary and every filesystem
path server-side: no request field SHALL be interpreted as a shell command,
argv vector, host, path or credential, and no response SHALL disclose an
absolute path, the adapter binary or a secret. When the deploy adapter is not
configured or a stage fails, Forge SHALL return a typed failure or partial
report that is recorded in the journal and never presented as success.

#### Scenario: Browser cannot supply an execution vector

- **WHEN** a caller attempts to pass a path, host, binary name, shell string or argv vector to a deploy route
- **THEN** Forge treats it only as the command's structured `target` field or ignores it, and never spawns a shell or interpolates argv

#### Scenario: Adapter unavailable or failing is reported honestly

- **WHEN** a confirmed deploy runs but the configured adapter binary is missing or returns a non-zero or unhealthy result
- **THEN** Forge returns a typed failure or partial report with the stage outcomes, records it in the journal, and does not represent the deploy as successful

