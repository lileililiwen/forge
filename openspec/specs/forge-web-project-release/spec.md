# forge-web-project-release Specification

## Purpose
Plan and apply project releases from the browser under confirm-and-digest discipline, delegating to the in-process release engine with server-side credentials.
## Requirements
### Requirement: Browser planning of a project release

Forge SHALL expose, on the session-gated global admin surface, a read-only route
that plans a release for exactly one managed project id and one requested semver
version by delegating to the same in-process release engine the CLI uses, and
SHALL resolve the project directory, the release configuration, the git remote
and the adapter binaries only from server-side state. The route SHALL require a
semver `version` and SHALL refuse a missing or malformed version with a typed
error. The plan SHALL NOT write project files, mutate the git working tree,
invoke any adapter, or record an operation, and its response SHALL NOT contain
an absolute filesystem path, an adapter binary path, a git remote credential or
any secret.

#### Scenario: Signed-in operator plans a release

- **WHEN** an authenticated admin session requests the release plan for a managed project id and a semver version
- **THEN** Forge returns a bounded, path-free plan (version, source revision, changelog, checks, stages, readiness) and a `plan_digest`, and performs no write, no git mutation and no adapter invocation

#### Scenario: Unsigned client requests a plan

- **WHEN** a request without a valid admin session cookie calls the release plan route
- **THEN** Forge refuses it the same way it refuses the other admin routes and runs no Core operation

#### Scenario: Project id is unmanaged or path-bearing

- **WHEN** the release plan route is called with an unknown project id, or an id containing a filesystem path
- **THEN** Forge returns a typed 404 or 400 respectively, echoes no path, and runs no Core operation

#### Scenario: Version is missing or not semver

- **WHEN** the release plan route is called without a `version` or with a value that is not a semver triple
- **THEN** Forge returns a typed 400 that does not echo arbitrary input and runs no Core operation

### Requirement: Confirmed digest-bound release application

Forge SHALL require, for the admin release apply route, `confirm` to be true and
a `plan_digest` matching the digest of the exact project and normalized version
the operator previewed, enforced in the admin layer independently of the CLI
release commands. A request that is not confirmed SHALL return a preview and
digest and SHALL NOT write, commit, tag, push or invoke an adapter. A request
whose digest does not match SHALL be refused with a typed error and a fresh
digest and SHALL perform no release side effect. Only on a matching digest SHALL
Forge delegate to the same release engine the equivalent CLI command runs,
record the run in the existing operations journal, and return its typed result.
The release stages SHALL come from the manifest-derived configuration, never from
a browser-supplied list.

#### Scenario: Apply without confirmation previews only

- **WHEN** a signed-in operator submits a release apply without `confirm` true
- **THEN** Forge returns the plan preview and a 64-hex `plan_digest`, writes nothing, mutates no git state, and invokes no adapter

#### Scenario: Apply with a mismatched digest is refused

- **WHEN** a signed-in operator submits a release apply whose `plan_digest` does not match the reviewed project and version
- **THEN** Forge refuses with a typed error and a fresh digest and performs no release side effect

#### Scenario: Confirmed apply runs the CLI's release engine

- **WHEN** a signed-in operator submits a release apply with `confirm` true and a matching `plan_digest`
- **THEN** Forge runs the same release engine the CLI runs with the manifest's stages, journals the operation, and returns its typed per-stage report

### Requirement: Honest release outcomes with server-side credentials

Forge SHALL keep release credentials, the git remote, the adapter binaries and
every filesystem path server-side: no request field SHALL be interpreted as a
shell command, argv vector, stage list, remote, path or credential, and no
response SHALL disclose an absolute path, the release state path, an adapter
binary or a secret. When the release configuration is invalid, a required check
fails, the git remote is missing, or a stage or adapter fails, Forge SHALL return
a typed failure or partial report that is recorded in the journal with the real
per-stage state and never presented as success.

#### Scenario: Browser cannot supply an execution vector

- **WHEN** a caller attempts to pass a stage list, path, remote, branch, binary name, shell string or argv vector to a release route
- **THEN** Forge treats it only as the command's structured `version` field or ignores it, and never spawns a shell or interpolates argv

#### Scenario: Adapter or remote failure is reported honestly

- **WHEN** a confirmed release runs but the configured adapter binary is missing or returns a non-zero result, or the git remote is absent or rejects the ref
- **THEN** Forge returns a typed failure or partial report with the real per-stage outcomes, records it in the journal, and does not represent the release as successful

#### Scenario: A blocked plan is refused before any side effect

- **WHEN** a confirmed release's required check is failing or stale, or the project declares no usable release configuration or changelog
- **THEN** Forge refuses with the engine's typed error, writes no commit or tag, and journals the refusal, never a fake success

