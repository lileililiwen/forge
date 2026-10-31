# Change: in-tree-policy-runtime

## Why

`driftwatchdog` is a Cargo workspace member of this repository, and a normal
`cargo build` produces `target/debug/driftwatchdog`. The policy adapter still
probes only `PATH`, so `forge doctor .` reports the policy plane as
`UNAVAILABLE` on the very checkout that built the runtime:

```text
[UNAVAILABLE] driftwatch-policy
  evidence: adapter did not produce a report: no driftwatch binary found:
  tried 'driftwatchdog', 'driftwatch' (FORGE_DRIFTWATCH_BIN unset)
```

That finding makes Forge's own doctor permanently `not healthy`, and it is
false: the runtime is present, built, and answers
`driftwatchdog check --dry-run --format json` with a valid empty-checker
report. A source checkout should resolve the runtime it built.

## What Changes

- **The resolver gains a source-checkout fallback.** After the ordered `PATH`
  probe, it looks for `target/{debug,release}/driftwatchdog` under
  `CARGO_MANIFEST_DIR` and selects the executable it finds.
- **The refusal names every location tried.** When neither `PATH` nor the
  checkout carries the runtime, the reason lists the candidates, the checkout
  paths and the `FORGE_DRIFTWATCH_BIN` override.
- **The in-tree runtime is tested.** A temp checkout with a fake executable
  proves the fallback; the existing PATH-preference tests stay unchanged.
- **An installed binary is still honest.** The release archive does not ship
  `driftwatchdog`, so an installed Forge on a host with no runtime continues to
  report the policy plane unavailable.
- **The policy plane runs checkers, not the Gate.** `policy_surface_args`
  always selects `check --dry-run --format json`. The Gate has its own evidence
  path; running it inside doctor duplicated a full build/test run and timed out.
- **An empty policy configuration is a warning.** A report with zero configured
  checkers is `NotConfigured` (WARN), not `Unjudged` (UNAVAILABLE); the plane
  ran and had nothing to judge. The release lane records it as a disabled check.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `src/policy/driftwatch.rs` | checkout fallback, checker surface, checkers count |
| `src/policy/model.rs` | `PolicyReport.checkers` |
| `src/doctor/assessment.rs` | `NotConfigured` conclusion |
| `src/release/engine/prepare.rs` | disabled release check for no checkers |
| `src/policy/policy_tests.rs` | fallback test |
| `openspec/specs/quality-policy-integration/spec.md` | one added requirement |
| `README.md`/doctor docs | no change; the finding now reflects the runtime |

Persistence: none.

Failure boundaries: an explicit `FORGE_DRIFTWATCH_BIN` still wins and is never
replaced; a non-executable checkout file is skipped; a checkout without a built
runtime keeps the unavailable outcome.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `quality-policy-integration` | modified | in-tree runtime resolution |
| `doctor-claim-evidence` | read only | the finding becomes pass/warn when the runtime answers |

## Non-Goals

- No vendoring of the runtime into the release archive; the archive remains
  binary + governance/contract assets.
- No change to the checker/gate surface selection or the report parsing.
- No new provider or network access.
