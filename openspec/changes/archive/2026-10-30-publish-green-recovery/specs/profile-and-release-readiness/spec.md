## ADDED Requirements

### Requirement: Readiness fixtures carry every rendered asset

A readiness fixture SHALL contain every file a generated project of that
profile would receive, including binary feed packages, so the native build
proves the fixture rather than a warm package cache.

#### Scenario: A profile declares a committed feed

- **WHEN** readiness renders a profile whose kit descriptor declares feed
  packages
- **THEN** each package is written at its project-relative path before the
  native build runs

#### Scenario: Cold package cache

- **WHEN** the host has no previously restored copy of a declared package
- **THEN** the fixture's native restore resolves it from the fixture's own feed

#### Scenario: Text-only rendering

- **WHEN** a caller renders the profile's text files and omits the binary feed
- **THEN** the readiness regression test fails naming the missing
  `packages/<feed>/*.nupkg` path
