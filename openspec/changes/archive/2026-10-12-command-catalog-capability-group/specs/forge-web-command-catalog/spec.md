# forge-web-command-catalog Specification (delta)

## ADDED Requirements

### Requirement: Capability grouping facade coverage

Forge SHALL represent the `forge cap` capability-grouping facade once per
Clap path — the `cap` group and its `add`, `inspect`, `list` and `run`
subcommands — with a per-command summary, category, scope, risk,
availability and reason taken from what that command actually does. Because
no implemented, session-gated admin route serves the facade, all five rows
SHALL be reported `cli_only` with `route` absent and a plain-language reason
and next step. A row SHALL NOT be reported `web` merely because the underlying
plugin list it composes is itself a `web` command, and a print-only facade verb
SHALL NOT be given a mutating risk or a planned-but-unimplemented web
disposition.

#### Scenario: Capability facade is covered once per path

- **WHEN** the command-catalog coverage check compares the catalog against the
  live Clap tree
- **THEN** `cap`, `cap.add`, `cap.inspect`, `cap.list` and `cap.run` each
  appear exactly once, no other row is invented, and the check reports no
  missing or extra path

#### Scenario: Read-only capability reads

- **WHEN** the catalog reports `cap list` or `cap inspect`
- **THEN** each is a `read`-risk, `project`-scoped `cli_only` row with no
  route, carrying the `local_filesystem` capability token and a reason naming
  the local project-tree read and the terminal next step

#### Scenario: Print-only capability verbs

- **WHEN** the catalog reports `cap add` or `cap run`
- **THEN** each is a `read`-risk, `forge`-scoped `cli_only` row with no route
  and no capability token, carrying a reason that states the command only
  prints the descriptor snippet or the flat `forge …` command, never executes
  it and never writes the manifest, and neither row is labelled as mutating
  state or as a planned web workflow

#### Scenario: Facade has no browser route

- **WHEN** an operator reads the catalog for a `forge cap` command
- **THEN** the row names no route, explains why no typed JSON route performs
  the work, and does not point at the plugin-list route that only the
  underlying `plugins list` command resolves to