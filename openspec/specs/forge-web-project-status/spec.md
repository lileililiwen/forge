# forge-web-project-status Specification

## Purpose
Surface read-only project status and a fleet readiness summary from the in-process doctor, checker, and readiness projection with honest sub-check states.
## Requirements
### Requirement: Read-only project status projection

Forge SHALL expose, on the session-gated global admin surface, a read-only
route `GET /v1/admin/projects/{id}/status` that reports the status of one
**managed** project by reusing the crate's in-process doctor, checker and
readiness functions. It SHALL NOT mutate project files, registry rows or the
operation journal, and it SHALL NOT run a subprocess that writes or a native
build/test toolchain. The route SHALL address a project only by a validated
opaque `id` resolved server-side from the registry; no request SHALL supply a
filesystem path, binary, host, argv vector or shell string.

#### Scenario: Signed-in operator reads a managed project's status

- **WHEN** an authenticated admin session requests the status of a managed project
- **THEN** Forge returns a bounded JSON projection whose overall `state` is one of `healthy`, `issues`, `stale` or `unavailable`, with the three sub-checks `doctor`, `check` and `readiness`

#### Scenario: Unmanaged project

- **WHEN** the id is a valid identifier but is not a project managed by this Forge registry
- **THEN** Forge returns a typed `404` whose body echoes no part of the id and performs no read of a browser-named path

#### Scenario: Hostile or path-bearing id

- **WHEN** the id contains shell metacharacters, a path separator, a traversal segment or a non-identifier
- **THEN** Forge returns a typed `400` that echoes no part of the offending input and reaches no filesystem layer

#### Scenario: Anonymous request

- **WHEN** a request without a valid admin session calls the status route
- **THEN** Forge returns `401` with no project status data

### Requirement: Honest sub-check states and safe reasons

Forge SHALL report each status sub-check (`doctor`, `check`, `readiness`) with
one of the states `healthy`, `issues`, `stale` or `unavailable`. A sub-check
that cannot run SHALL be reported `unavailable` with a safe, path-free reason
and SHALL never be reported as a pass. When any sub-check reports `issues` the
overall state SHALL be `issues`; otherwise when any sub-check is `unavailable`
the overall state SHALL be `unavailable`; otherwise when the `doctor` check is
`stale` the overall state SHALL be `stale`; otherwise the overall state SHALL
be `healthy`. The `readiness` sub-check SHALL be computed from the read-only
profile/readiness projection and SHALL NOT invoke the native readiness matrix,
generate fixtures, or run any build/test subprocess. The `check` sub-check
SHALL reuse the in-process checker assembly over the doctor report and the
read-only governance observation.

#### Scenario: Healthy project

- **WHEN** every sub-check can run and reports no failing or missing evidence
- **THEN** the overall state is `healthy`

#### Scenario: Real finding reported honestly

- **WHEN** the in-process doctor or checker reports failing or warning evidence for the project
- **THEN** that sub-check and the overall state report `issues`, never `healthy` or a pass

#### Scenario: Sub-check cannot run

- **WHEN** a sub-check cannot read its inputs (for example the project directory is not readable)
- **THEN** it is reported `unavailable` with a safe reason that names no absolute filesystem path and no credential

#### Scenario: Readiness is the read-only projection

- **WHEN** the status route reports readiness for a project
- **THEN** it reports the profile's catalog support state only and performs no native build, test or fixture generation

### Requirement: Fleet readiness summary

Forge SHALL expose, on the same session-gated admin surface, a read-only fleet
summary route `GET /v1/admin/status` that counts the registered managed
projects by overall status into the states `healthy`, `issues`, `stale` and
`unavailable`, so the portal can render a fleet-wide readiness tile. The
summary SHALL be non-live, SHALL reuse the same read-only per-project
projection, and SHALL disclose no absolute filesystem path or credential.

#### Scenario: Fleet readiness counts

- **WHEN** a signed-in operator reads the fleet summary
- **THEN** Forge returns a per-state count and a total over the registered managed projects, each counted state matching the project's own status projection

#### Scenario: Fleet summary is anonymous-safe

- **WHEN** a request without a valid admin session calls the fleet summary route
- **THEN** Forge returns `401` with no counts

### Requirement: Status privacy and catalog agreement

Forge SHALL keep every filesystem location server-side: no status or fleet
summary response SHALL serialize the registered project path, a credential or
an adapter binary. The command catalog SHALL report `check` as a `web` row
pointing at the project status route and `fleet status` as a `web` row
pointing at the fleet summary route, so the catalog's disposition agrees with
the routes the browser can actually call.

#### Scenario: No path or credential leaks

- **WHEN** a status or fleet summary route returns success or a refusal
- **THEN** the serialized body contains no absolute filesystem path, no credential and no echoed hostile input

#### Scenario: Catalog names the status routes

- **WHEN** the command catalog is fetched
- **THEN** `check` appears `web` with the project status route and `fleet status` appears `web` with the fleet summary route, and neither keeps a CLI-only next step

