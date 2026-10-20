# runtime-kind-shared-host Specification

## Purpose
The publish inventory's contract already carries a runtime vocabulary —
`web|worker|job|library` — but it says what a project is without ever saying
how it is hosted, and it has no owner outside the publish lane. A `library`
project is a module: it runs inside a host another project owns, and several
modules can share one host. The repository's own instance of that shape is
`modules/dotnet-platform-libs` — 80 independently adoptable .NET packages
that all run inside one CLR host inside the consuming application. Forge
cannot name that shape, and the fleet treats every entry the same way, so a
module reads as a service that failed to start: a compose-ready `library`
entry is published as a Mac Docker workload of its own, while the host that
actually owns the module never learns about it. This capability gives Forge
Core one runtime-kind vocabulary that declares a host model for every kind,
moves the publish inventory onto it, and makes the fleet judge a module by
its host model rather than by its compose file.

## ADDED Requirements

### Requirement: One runtime-kind vocabulary with a host model

Forge SHALL own one runtime-kind vocabulary — `web`, `worker`, `job` and
`library` — and every kind SHALL declare how its artifact is hosted. An
own-process kind SHALL run as a process of its own, with its own container
and its own deploy target. A shared-host kind SHALL be a module loaded into
a host another project owns, where several modules can share one host. The
`library` kind SHALL be the shared-host kind. A declared value that is not
one of the four SHALL be refused, and the refusal SHALL name the accepted
values.

#### Scenario: Every kind declares a host model

- **WHEN** a runtime kind is read
- **THEN** it reports its host model, and the shared-host kind is the module
  kind that several modules can share one host under

#### Scenario: An unknown runtime kind is refused

- **WHEN** a runtime value that is not one of the four is read
- **THEN** it is refused, and the refusal names the accepted values

### Requirement: The fleet judges a module by its host model

Forge SHALL publish an inventory entry when it is compose-ready **and** its
runtime kind is an own-process kind. A shared-host entry SHALL be reported
and SHALL NOT be published, whatever compose file it declares, because the
host that adopts the module owns its lifecycle. The report SHALL name the
host model of every entry whose runtime kind was read, and a shared-host
entry's reason SHALL say that it is a module hosted by the project that
adopts it. An entry whose runtime kind could not be read SHALL name no
runtime kind and no host model, and SHALL carry the reason it was refused.

#### Scenario: A module is never started by the fleet

- **WHEN** an inventory entry declares the shared-host kind and also
  declares a Compose file that exists
- **THEN** it is reported with its host model and never published, and its
  reason says that the host that adopts it owns the lifecycle

#### Scenario: An own-process entry is published as before

- **WHEN** a `web`, `worker` or `job` entry is compose-ready
- **THEN** it is published, including the non-web own-process kinds, and only
  a `web` entry may receive a public route

#### Scenario: A module may not expose HTTP

- **WHEN** an inventory entry declares the shared-host kind and declares
  `public_http`
- **THEN** the entry is refused, and the refusal names the host model
