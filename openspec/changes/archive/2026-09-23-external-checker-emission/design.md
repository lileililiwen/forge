# Design: Forge as an external DriftWatch checker

## Ownership and boundaries

Driftwatchdog owns the checker protocol and decides what an alert means for
its gate. Forge owns which of its observations become alerts. This change
adds a projection layer only: it reuses `crate::doctor`, `crate::policy`,
`crate::governance` and `crate::readiness` results and must not invent a
parallel assessment.

## Command surface

`forge check [TARGET]` (TARGET defaults to the current project, like
`forge doctor`). Options: `--include-policy` (run the DriftWatch policy
plane first and re-emit its fail/warn findings as alerts — off by default to
avoid a feedback loop where forge drives driftwatch which drives forge) and
`--max-alerts N` (bounded, default 64). Human output prints the same
document; this command exists for machines.

## Envelope contract

```json
{
  "schema": "forge-checker/0.1.0",
  "generated_at": "<rfc3339>",
  "alerts": [
    {"severity": "error|warning", "message": "...",
     "source": "forge.yaml", "symbol": "POLICY-ID|doctor/<finding>"}
  ]
}
```

- `alerts` is always present, possibly empty — an absent key is a protocol
  error under the sibling's Forward-compatible alerts protocol.
- Unknown extra fields (`schema`, `generated_at`) are tolerated by that
  same protocol, which is why they may ship.
- Severity mapping: Forge `fail`→`error`, Forge `warn`→`warning`; passing
  and not-applicable findings are omitted (a checker document that says
  "nothing wrong" must not claim more than the evidence does; `unverified`
  and `unavailable` states become `warning` alerts naming the missing
  evidence, never silently dropped).

## Source and symbol rules

`source` is a project-relative path when the finding is file-scoped, else
the plane name (`governance`, `readiness`, `doctor`). `symbol` is the stable
finding/policy id. Paths are normalized; traversal or absolute paths are
rejected at construction time so the document cannot leak host layout.

## No-mutation rule

A checker run is read-only against the assessed project: forge.yaml bytes,
registry file, `.forge/` state and Git HEAD must be unchanged after the run.
This deliberately bypasses the normal doctor journaling path; the test
asserts byte equality before/after.

## Failure behavior

Unregistered target → existing typed error on stderr, exit 1, no partial
document on stdout. Any internal panic boundary degrades to exit non-zero
with stderr only, because a malformed document on stdout would be ingested
as checker output.

## Verification

- Fixture: captured `driftwatch.toml` checker example run against the CLI
  with a stub binary in tests (contract test mirrors the sibling's
  protocol expectations: empty alerts = success, missing key = error).
- Regression: doctor verdict determinism unchanged; provider matrix rows
  unaffected; `--format json` global flag does not alter `forge check`
  stdout.
