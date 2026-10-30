# Change: dependency-advisory-remediation

## Why
`cargo audit` reported one vulnerability and five warnings, and the repository's
own Gate had been blocked by the same advisory for four consecutive changes: a
`security` check that every cycle records and moves past is no longer a signal.

| Advisory | Crate | Fixed release |
|---|---|---|
| RUSTSEC-2026-0387 | `crypto-bigint 0.5.5` | none; `openidconnect 4.0.1` is latest |
| RUSTSEC-2026-0386 | `tokio 1.53.2` | `1.54.1` |
| RUSTSEC-2026-0253 / -0002 | `lru 0.12.5` | `ratatui 0.30.2` |
| RUSTSEC-2024-0436 | `paste` | removed by `ratatui 0.30.2` |

Three have a direct upgrade. The fourth has no upstream fix: it is reached via
`openidconnect -> elliptic-curve -> p256/p384`, and Forge only verifies
public-key signatures. Upgrade what can be upgraded, and record the unreachable
path as a reviewed exception in both policy files, as the `rsa` exception
already does. The `bans` check also failed on a path-only wildcard dependency,
fixed in the same pass.

## What Changes

- **`tokio` moves to `1.54.1`**, which fixes the `AsyncFd` unsoundness. The
  update is pinned to the single crate so the rest of the lockfile does not
  move in the same change.
- **`ratatui` moves from `0.26` to `0.30.2`** in `cargo-audit-boost` and
  `cargo-matrix-boost`. The upgrade removes `lru 0.12.5` and `paste` from the
  graph; the two TUI surfaces are adapted to the current API.
- **`RUSTSEC-2026-0387` is excepted, with its reachability argument**, in
  `deny.toml` and `.cargo/audit.toml`. The exception names the transitive path,
  the fact that Forge performs public-key signature verification only, and the
  upstream trigger that ends the exception.
- **The `driftwatchdog` path dependency declares its version.** The path-only
  entry was a wildcard to `cargo deny`, so the `bans` check failed before any
  advisory did. The version now matches the crate manifest.
- **A policy contract test keeps the two files in agreement**: every
  vulnerability-class ignore is present in both, and every `deny.toml` ignore
  carries a reason.
- **No product behaviour changes.** The identity, audit and matrix behaviour is
  unchanged; the new TUI code renders the same state.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `Cargo.toml`, `Cargo.lock` | `tokio` pin; `ratatui` feature/version bump; versioned `driftwatchdog` path |
| `crates/cargo-audit-boost/src/ui.rs` | ratatui 0.30 API |
| `crates/cargo-matrix-boost/src/ui.rs` | ratatui 0.30 API |
| `deny.toml` | the advisory exception with its reason |
| `.cargo/audit.toml` | the same exception for `cargo audit` |
| `tests/dependency_policy_contract.rs` | policy parity and reason checks |
| `openspec/specs/artifact-and-ci-baseline/spec.md` | one added requirement |

Persistence: none.

Failure boundaries: an upgrade that changes a TUI verdict fails the crate's
tests; a policy file that loses an exception makes `cargo audit` or
`cargo deny` fail; the contract test fails when the two files disagree.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `artifact-and-ci-baseline` | modified | exception reachability and policy-file parity |
| `quality-policy-integration` | read only | the Gate's `security` check keeps its command |
| TUI crates | behaviour-preserving | versions and API adaptation only |

## Non-Goals

- No replacement of `openidconnect`. A different OIDC implementation is a
  product change with its own risk; the advisory's affected path is unreachable
  here and the upstream release ends the exception.
- No forked or patched `elliptic-curve`, and no `[patch]` section that hides the
  advisory from the graph.
- No change to the Gate's `security` command, the baseline, or the evidence
  shape.
- No new lint, feature or policy beyond the exception plumbing.
