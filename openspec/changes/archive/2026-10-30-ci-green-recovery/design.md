## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The CI job graph and step commands | `forge` | Yes. `.github/workflows/ci.yml`. |
| Rust formatting and lint state | `forge` | Yes. Workspace sources and vendored crates. |
| The checker, gate, registry and doctor behaviour | `forge` | No. Unchanged. |
| The advisory decision and dependency upgrades | `forge`, next change | No. Recorded as pre-existing. |
| Release publication | the user | No. Nothing is tagged or pushed. |

## What Makes CI Honest

Each failing job is repaired at the layer that owns the failure, not by
weakening the check.

| Job | Failure | Repair | What still fails |
|---|---|---|---|
| `governance` | `openspec` absent | install `@fission-ai/openspec@1.6.0` when the binary is absent, then validate | a failed install or a validation error |
| `surfaces` | `forge check .` on an unregistered checkout | generate and register a scratch project, run the checker on it | generation, registration or checker failure |
| `gate` | 600 s wrapper < 600 s × 2 checks; `cargo-audit` absent | `--timeout-secs 2400`; install `cargo-audit@0.22.2` | a real blocked verdict, a timeout, or a missing tool |
| `fmt` | 14 files differ | `cargo fmt --all` | any later drift |
| `clippy` | 21 warnings under `-D warnings` | fix the lints; two structural allows | any later warning |

The `surfaces` decision is deliberate. `forge check` is specified for a
*registered* project, and a fresh checkout has no registry row. Registering
Forge itself would require deciding what a native profile is, which is a
separate capability. A generated project is a real project with a real
manifest, so the checker surface is exercised without inventing that decision
or mutating this repository.

## The Two Allows

`draw_ui` has nine parameters: the TUI frame, view state, selection, two
scroll offsets, search state and three filters. Splitting it into a struct
would move the same values behind a constructor with no reduction in coupling.
`cmd_build` has nine parameters for the same reason: it is the `BuildArgs`
projection. Both carry `#[allow(clippy::too_many_arguments)]` with the reason
next to the signature; every other warning is fixed.

## Failure Boundaries

- A runner without network cannot install `openspec` or `cargo-audit`; the job
  fails naming the tool rather than skipping.
- `forge gate` may still return `blocked` because of the pre-existing
  `crypto-bigint` advisory. The job mirrors that verdict; it does not add an
  audit ignore to turn the job green.
- The scratch project is created under `$RUNNER_TEMP` and never committed.
- No step writes to this repository outside `target/`.

## The Flaky Parity Test

`format_json_does_not_alter_checker_stdout` compared two complete documents
after removing only the `"generated_at":"` marker, leaving the timestamp value
in place. Two runs one second apart differed. The test now replaces the whole
`generated_at` value with `normalized` and compares the rest byte for byte, so
it still detects a real `--format json` difference and no longer depends on
wall-clock timing.

## Verification Strategy

1. `cargo fmt --all -- --check` exits 0.
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   exits 0.
3. The CI YAML is parsed by the workflow runner's schema through a local
   `python3 -c 'yaml.safe_load(...)'` check and the three changed jobs are read
   back.
4. The checker path is reproduced locally: generate, register and check a
   scratch `rust-web` project, asserting the document carries `alerts`.
5. The repository suite, OpenSpec validation and whitespace checks run as
   usual.
6. The repository Gate runs; the advisory failure is recorded as pre-existing
   with its remediation path and the next change.
