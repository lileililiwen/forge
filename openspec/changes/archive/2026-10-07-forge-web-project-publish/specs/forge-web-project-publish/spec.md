# forge-web-project-publish (delta)

## ADDED Requirements

### Requirement: Browser planning of a provider publish

Forge SHALL expose, on the session-gated global admin surface, a read-only route
that plans a provider publish for exactly one managed project id by resolving the
project directory, the provider id, the provider configuration and the committed
git revision only from server-side state. The route SHALL refuse an unset
`FORGE_PUBLISH_PROVIDER`, a missing or invalid provider configuration, an unknown
or disabled provider, or a project with no resolvable 40-character revision with a
typed prerequisite error that names the variable or configuration and never a
credential value, a provider id value or an absolute path. The plan SHALL NOT
invoke any provider, write any project file, or record an operation, and its
response SHALL NOT contain an absolute filesystem path, the provider executable,
the provider configuration path, an SSH target or any secret.

#### Scenario: Signed-in operator plans a publish

- **WHEN** an authenticated admin session requests the publish plan for a managed project id with a configured provider and a committed revision
- **THEN** Forge returns a bounded, path-free plan (provider, operation, 40-hex revision) and a 64-hex `plan_digest`, and invokes no provider and writes no journal row

#### Scenario: Unsigned client requests a plan

- **WHEN** a request without a valid admin session cookie calls the publish plan route
- **THEN** Forge refuses it the same way it refuses the other admin routes and runs no Core operation

#### Scenario: Project id is unmanaged or path-bearing

- **WHEN** the publish plan route is called with an unknown project id, or an id containing a filesystem path
- **THEN** Forge returns a typed 404 or 400 respectively, echoes no path, and runs no Core operation

#### Scenario: Publish prerequisite is unset or unusable

- **WHEN** `FORGE_PUBLISH_PROVIDER` is unset or blank, the provider configuration file is missing or invalid, the named provider is unknown or disabled, or the project has no resolvable commit
- **THEN** Forge returns a typed `409 admin-prerequisite` that names the variable or configuration, echoes no provider id or path, and invokes no provider

### Requirement: Confirmed digest-bound provider publish

Forge SHALL require, for the admin publish apply route, `confirm` to be true and a
`plan_digest` matching the digest of the exact project, provider id and resolved
revision the operator previewed, enforced in the admin layer independently of the
CLI publish commands. A request that is not confirmed SHALL return a preview and
digest and SHALL NOT invoke a provider or record an operation. A request whose
digest does not match SHALL be refused with a typed error and a fresh digest and
SHALL perform no provider call. Only on a matching digest SHALL Forge build the
fixed `forge-publish-provider/0.1.0` request and invoke the same in-process
provider function the equivalent CLI command runs, then record the run in the
existing operations journal with the provider-reported state and return its typed
result. The apply body SHALL read only `confirm` and `plan_digest`; no request
field SHALL be interpreted as a provider, revision, path, argv vector, host, SSH
target, credential or shell text.

#### Scenario: Apply without confirmation previews only

- **WHEN** a signed-in operator submits a publish apply without `confirm` true
- **THEN** Forge returns the plan preview and a 64-hex `plan_digest`, invokes no provider, and records no operation

#### Scenario: Apply with a mismatched digest is refused

- **WHEN** a signed-in operator submits a publish apply whose `plan_digest` does not match the reviewed project, provider and revision
- **THEN** Forge refuses with a typed `409 admin-digest-mismatch` and a fresh digest and performs no provider call and no journal write

#### Scenario: Confirmed apply runs the CLI's publish path

- **WHEN** a signed-in operator submits a publish apply with `confirm` true and a matching `plan_digest`
- **THEN** Forge invokes the same provider function the CLI runs with the fixed contract and server-resolved fields, journals one `publish` operation carrying the provider-reported state and phase evidence, and returns its typed result

### Requirement: Honest publish outcomes with server-side provider resolution

Forge SHALL keep the provider executable, the provider id, the project directory,
the revision and every credential server-side: no request field SHALL be
interpreted as a provider, revision, path, argv vector, remote, SSH target,
credential or free-text command, and no response SHALL disclose an absolute path,
the provider executable, the provider configuration path, an SSH target or a
secret. When the provider cannot be spawned, times out, exits non-zero, returns an
invalid contract response or leaks secret-like evidence, Forge SHALL record a
`failed` operation with the real reason and return a typed non-success. When the
provider returns a non-healthy status, Forge SHALL return the provider's real
status, health and evidence with `healthy:false`, never presented as success.

#### Scenario: Browser cannot supply an execution vector

- **WHEN** a caller attempts to pass a provider, revision, path, remote, SSH target, binary name, shell string or argv vector to a publish route
- **THEN** Forge ignores it or rejects it, and never spawns a shell or interpolates argv, and the provider is chosen only from server-side state

#### Scenario: Provider failure is reported honestly

- **WHEN** a confirmed publish runs but the provider executable is missing or non-zero, times out, or returns an invalid response
- **THEN** Forge records a `failed` operation for the project and returns a typed non-success with no path or secret, and never a fake success

#### Scenario: Unhealthy provider response is not success

- **WHEN** a confirmed publish's provider returns a valid response whose status is not `done` or whose health is not `healthy`
- **THEN** Forge journals that real state and returns the provider's real status and evidence with `healthy:false`, not a success claim
