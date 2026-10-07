# forge-web-project-actions Specification

## Purpose
TBD - created by archiving change forge-web-project-actions. Update Purpose after archive.
## Requirements
### Requirement: Browser execution of Core-backed project lifecycle writes

Forge SHALL expose, on the session-gated global admin surface, typed endpoints
that execute the project lifecycle commands whose Core operation is a pure
in-process handler keyed by a validated project id — `feature remove`
(`remove_feature`), `feature upgrade` (`upgrade_feature`) and `spec apply`
(`apply_routing`) — by delegating, on a matching confirmation digest, to the same
handlers the CLI runs. Each endpoint SHALL accept only that command's structured
scalar fields (a feature id, an optional version for upgrade, a non-empty set of
finding ids and an optional reason for spec apply) and SHALL NOT interpret any
request field as a shell command, argv vector, filesystem path, or arbitrary CLI
text. A request SHALL be refused before any Core call unless it is a JSON body
from a valid admin session, and an unknown or observed-only project id SHALL
return a typed not-found that never serializes the stored absolute path.

<!-- traceability: outcome=R1 execution; design=§1,§4,§6; boundary=src/api/admin.rs
     +src/api/mod.rs (Route::AdminProjectFeatureRemove/Upgrade, AdminProjectSpecApply)
     → feature::remove_feature/upgrade_feature, spec::apply_routing;
     success=confirmed run equals a direct Core call; failure=401/415/404/400 typed
     no-echo; boundary=digest mismatch 409 no-write; tasks=2.2,2.3,2.4;
     oracle=tests/forge_web_project_actions_contract.rs §7.1,3,4,5,6 -->

#### Scenario: Signed-in operator previews then confirms a lifecycle write

- **WHEN** an authenticated admin session submits the structured fields for `feature remove`, `feature upgrade` or `spec apply` without confirmation
- **THEN** Forge returns a path-free preview descriptor and a `plan_digest` and writes nothing until the operator confirms with that exact digest

#### Scenario: Confirmed lifecycle write matches the CLI

- **WHEN** a signed-in operator confirms with a matching `plan_digest`
- **THEN** Forge runs the same Core handler the equivalent CLI command runs and returns its typed result, mutating the project exactly as the CLI would

#### Scenario: Mutation without a valid session or JSON body

- **WHEN** a request without a valid admin session cookie, or without a JSON content type, calls any of the three routes
- **THEN** Forge refuses it as the other admin routes do (401 / 415) and runs no Core operation

#### Scenario: No generic text execution exists

- **WHEN** a caller attempts to pass CLI text, a shell string, an argv vector or a filesystem path to one of these routes
- **THEN** Forge treats it only as that command's structured field or rejects it, and never spawns a shell or interpolates argv

### Requirement: Confirmation-gated execution rendered from the catalog

Forge SHALL render, for every executable catalog row that applies to the current
project, a runnable preview→confirm→apply control generated from that row's
`execution` block — its route, method, typed `parameters`, and confirmation
requirement — with a control per declared typed parameter and no free-text field
that accepts a command, path or argv. The rendered control SHALL request the row's
route and SHALL disable the apply step until the preview `plan_digest` is
confirmed, so the inventory itself is actionable rather than descriptive. A row
without an `execution` block SHALL never be rendered as runnable.

<!-- traceability: outcome=R2 frontend execution; design=§4,§5;
     boundary=frontend/index.html + frontend/app.js consuming CommandRow.execution;
     success=feature remove/upgrade/spec apply actionable from catalog;
     failure=row with null execution not rendered runnable;
     boundary=apply gated on confirmed digest; tasks=2.7,3.2;
     oracle=Playwright temp-registry round trip + catalog serialization test §7 -->

#### Scenario: Executable project action is actionable from the list

- **WHEN** an operator opens a project whose catalog rows include `feature remove`, `feature upgrade` or `spec apply`
- **THEN** each renders a parameter form and a confirm-gated apply that calls its declared admin route

#### Scenario: A non-executable row is never runnable

- **WHEN** a catalog row reports a CLI-only, provider-required, project-capability-required, disabled or not-yet-web disposition
- **THEN** the portal shows its honest disposition and next step and renders no apply control for it

### Requirement: Lifecycle execution stays inside the Core-only boundary

Forge SHALL keep this executable set limited to pure in-process Core operations on
a registered project id, and SHALL NOT expose subprocess execution (a build or
test command), external-provider operations (deploy, publish, release write),
interactive PTY sessions (`agent`), transport servers, or hidden-input commands as
browser-executable actions. The commands excluded from execution SHALL retain
their truthful CLI-only or provider-required disposition so the catalog never
claims a capability the portal does not safely provide.

<!-- traceability: outcome=R3 scope integrity; design=§6,§8;
     boundary=command_catalog rows for build/test/deploy/publish/agent/serve keep
     non-web disposition; success=excluded commands keep honest reason;
     failure=no route that shells out or hits a provider is registered on the admin
     lifecycle surface; tasks=3.4; oracle=catalog problems() empty + row assertions -->

#### Scenario: Subprocess and provider commands stay CLI-only

- **WHEN** the catalog is read after this change
- **THEN** `build`, `test`, `deploy`, `publish`, release writes, `agent` and `serve` commands remain non-executable with their existing reasons, and no admin lifecycle route invokes a subprocess or provider

