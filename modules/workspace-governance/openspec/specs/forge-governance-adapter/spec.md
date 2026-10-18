# forge-governance-adapter Specification

## Purpose

Forge exposes a versioned governance-provider contract (v0.1.0: a JSON request on stdin, a normalized observation on stdout) and advertises `workspace-governance` as an optional provider id — but no adapter exists anywhere, so the portfolio's real governance facts (`workspace_check.py --audit --json`: ERROR/WARN findings, adoption gaps, per-project codes) never reach Forge's doctor, portal or release surfaces. Workspace Governance owns those facts; the missing piece is a small executable shim in this repository that translates one audit result into one contract observation. This gives the audit a machine consumer without this repo gaining any UI, and gives Forge real portfolio evidence.
## Requirements
### Requirement: Contract-shaped check response

The adapter MUST accept one `0.1.0` check request on stdin naming a
project, MUST evaluate exactly that project's audit findings, and MUST
print exactly one normalized observation to stdout with provider identity,
protocol version, echoed project id, mapped status, bounded
secret-scrubbed evidence and a summary detail.

#### Scenario: Clean adopted project

- **WHEN** the requested project has no ERROR findings and is adopted
- **THEN** the observation status is `pass` and WARN findings appear only
  as evidence

#### Scenario: Failing project

- **WHEN** the audit attributes an ERROR to the requested project
- **THEN** the status is `fail` with that finding's code first in evidence

#### Scenario: Adoption gap

- **WHEN** the project is discovered but not adopted
- **THEN** the status is `blocked` and the detail names the adoption state

#### Scenario: Unregistered project

- **WHEN** no registry entry matches the requested id
- **THEN** the status is `unknown` and the observation exits zero so the
  consumer distinguishes "answered unknown" from "could not answer"

### Requirement: Failure-class exit semantics

The adapter MUST exit zero for every answerable evaluation (including
`fail`), and MUST exit non-zero with no stdout only when it cannot
produce a trustworthy observation (malformed request, internal audit
failure), so consumers never map adapter failure to a governance status.

#### Scenario: Malformed request

- **WHEN** stdin is not a valid `0.1.0` check request
- **THEN** the adapter exits non-zero without printing an observation

#### Scenario: Audit crash

- **WHEN** the underlying audit raises
- **THEN** the adapter exits non-zero and Forge-side classification is
  `unavailable`, not `fail`

### Requirement: Read-only evidence surface

The adapter MUST NOT write, register, adopt or mutate anything in this
repository, and MUST scrub secret-shaped substrings from all evidence
lines.

#### Scenario: Tree unchanged

- **WHEN** the adapter runs against a fixture workspace
- **THEN** a recursive hash of the repository tree is identical before and
  after
