# Forge as an external DriftWatch checker

Forge can act as a spec checker for any Driftwatchdog-monitored project.
`forge check` projects the existing read-only assessment planes (doctor
findings, the optional DriftWatch policy plane, the selected governance
observation and the profile readiness state) into the Driftwatchdog
external-checker protocol. It never runs a second assessment, never
writes anything, and never journals the run.

## Command surface

```sh
forge check [TARGET] [--include-policy] [--max-alerts N]
```

- `TARGET` is a registered project id or path (default: the current
  directory). An unregistered target fails with the typed
  `error[unknown-project]` on stderr, exit 1, and an empty stdout.
- `--include-policy` runs the DriftWatch policy plane first and re-emits
  its findings as alerts. It is **off by default** to avoid a feedback
  loop where Forge drives Driftwatchdog which drives Forge back through
  this same registration.
- `--max-alerts N` bounds the document (default 64, valid range 1..=10000,
  the sibling protocol cap). Truncation is explicit: the document keeps
  its bound and ends with a `check/truncated` summary alert naming the
  dropped count.

## Document contract

stdout carries exactly one compact JSON document, and nothing else:

```json
{
  "schema": "forge-checker/0.1.0",
  "generated_at": "2026-09-24T10:00:00Z",
  "alerts": [
    {"severity": "warning", "message": "repository: required inspector ...",
     "source": "doctor", "symbol": "doctor/repository"}
  ]
}
```

- `alerts` is always present, possibly empty. `{}` without the key would
  be a protocol error under the sibling's forward-compatible alerts
  protocol, so it is never emitted.
- Severities stay inside the protocol vocabulary: Forge `fail` becomes
  `error`; `warn`, `unavailable` and `unverified` evidence becomes
  `warning` naming the gap (missing evidence is never silently dropped).
  Passing and not-applicable findings are omitted — the document claims
  no more than the evidence supports.
- `source` is a project-relative path when the finding is file-scoped,
  otherwise the plane name (`doctor`, `governance`, `readiness`).
  Absolute paths and `..` traversal are rejected at construction time,
  so a checker document can never leak the host layout.
- `symbol` is the stable finding or policy id: `doctor/<finding>`, a
  `driftwatch-<RULE>` policy id, `governance/<provider>`,
  `readiness/<profile>`, or `check/truncated`.
- Every field passes the shared credential redactor
  (`policy::redact_credentials`), messages are bounded, and the assessed
  project's absolute path is replaced with `<project>`.
- Findings never change the exit code: a document with `error` alerts
  still exits 0 (that is the checker's job to gate, not Forge's). Only
  Forge-side operational failures exit non-zero, with stderr only.
- `--format json` and the human format print the identical document;
  this command exists for machines.

## Read-only guarantee

A `forge check` run must not mutate the assessed project: the manifest
bytes, registry file, `.forge/` state, Git HEAD and the operation
journal are byte-identical before and after. Repeated runs emit the same
document except for `generated_at`. The governance plane is evaluated
through the read-only path (`governance::inspect`), not the persisting
`check_project` path, and no transport (MCP/API/portal) exposes the
checker command.

## Registration in a monitored project

Add one checker block to the project's `driftwatch.toml`. `command` is
the program (no shell splitting) and `args` the argument array; the
working directory defaults to the project root, so `.` resolves to the
monitored project. The project still needs `forge.yaml` and one
`forge register` for the data planes it exercises; Forge is not a build
dependency of the monitored project.

```toml
[[checkers]]
name = "forge"
command = "forge"
args = ["check", "."]
timeout_ms = 30000
```

Then run `driftwatch check` as usual. Forge findings appear as just
another configured checker in the gate evidence.

## Non-goals

- No network endpoint, watcher or daemon for pushing findings.
- No MCP/API/portal exposure of `forge check`, and no journaling of its
  runs.
- No change to what doctor computes: the checker is a projection.
- No requirement that monitored projects install Forge; checker
  registration is opt-in per project.

Contract version: `forge-checker/0.1.0`. Source: requirement.md §23,
§24, §32, §34; the sibling protocol is the archived
`config-checker-protocol` spec in the driftwatchdog repository.
