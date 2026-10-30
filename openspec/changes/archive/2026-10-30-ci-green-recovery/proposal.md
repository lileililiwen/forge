# Change: ci-green-recovery

## Why
Forge's CI is the second verification layer, but it has not been green since the
2026-10-15 push. Five jobs failed, and fifty commits have landed since without
CI seeing them. A local `forge gate` can be green while `fmt`, `clippy`, `deny`
and `governance` fail, because the local Gate does not run them.

Three failures are structural and remain in the workflow:

- `governance` refuses because the `openspec` CLI is installed nowhere;
- `surfaces` runs `forge check .` on a fresh checkout, but the checker resolves
  only registered projects;
- `gate` runs with a 600 s bound while `tests` and `declared-verification` have
  600 s each, and never installs the `cargo-audit` binary its `security` check
  invokes.

Two are drift from the unpushed work: the workspace is not rustfmt-clean and
`clippy -D warnings` exits 101. Until these are fixed, later changes are
verified only by a local Gate that does not represent CI.

## What Changes

- **Formatting is clean.** `cargo fmt --all` is applied to the workspace,
  including the vendored tooling crates.
- **Lints are clean.** The clippy warnings are fixed (not suppressed) except
  for the two structural cases: `draw_ui` and `cmd_build` keep a documented
  `too_many_arguments` allow because their argument lists are the CLI/TUI
  surface, not a design.
- **The `governance` job installs its declared validator.** When `openspec` is
  absent, the job installs `@fission-ai/openspec@1.6.0` and then validates; a
  failed install still fails the job.
- **The `surfaces` job exercises the checker against a registered project.**
  It generates a scratch `rust-web` project under `$RUNNER_TEMP`, registers it
  and runs `forge check` on that project. `forge doctor .` still runs against
  this repository. The job no longer depends on Forge being registered.
- **The `gate` job bounds the run to its plan and installs its checker.** It
  passes `--timeout-secs 2400` and installs `cargo-audit` at the version the
  local Gate uses before running `forge gate .`.
- **No product behaviour changes.** The checker, gate, registry and doctor
  contracts are unchanged; this change only makes the CI job graph exercise
  them honestly.
- **The checker parity test is deterministic.** It compared two full documents
  including a wall-clock `generated_at`, so a run that crossed a second
  boundary failed. The test now normalizes that one field and compares the
  rest byte for byte.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `.github/workflows/ci.yml` | `governance`, `surfaces` and `gate` jobs install and bound what they invoke |
| `crates/cargo-audit-boost/`, `crates/cargo-sonar/` | formatting and lint-only edits |
| `src/auth/setup.rs`, `src/cli/auth.rs`, `src/cli/gitops.rs`, `src/contract/mod.rs`, `src/deploy/model.rs`, `src/hosted_ci/store.rs`, `src/lib.rs`, `src/registry/model.rs` | formatting and lint-only edits |
| `tests/*` | formatting-only edits; `checker_contract` timestamp normalization |
| `openspec/specs/artifact-and-ci-baseline/spec.md` | one added requirement for installed prerequisites and a bounded Gate |
| `HANDOFF.md` | cycle evidence |

Persistence: none. No registry, project or evidence schema changes.

Failure boundaries: a missing `openspec` install fails `governance`; a checker
or registration failure fails `surfaces`; a Gate timeout or a missing
`cargo-audit` fails `gate`. No step skips to green.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `artifact-and-ci-baseline` | modified | the CI prerequisite and bound additions |
| `gate-runtime-evidence` | read only | the Gate engine and evidence shape are unchanged |
| `external-checker-emission` | read only | the checker is exercised as shipped |

## Non-Goals

- No change to `forge check`, `forge gate`, `forge register` or `forge doctor`.
  The CI job is adapted to the shipped contracts, not the other way round.
- No dependency upgrade. The advisory set (`crypto-bigint`, `tokio`, `lru`,
  `paste`) is the next change; this change records the pre-existing Gate
  failure and does not hide it.
- No new CI job, no release publication, no tag and no push.
- No refactor of the vendored tooling crates beyond formatting and lint fixes.
