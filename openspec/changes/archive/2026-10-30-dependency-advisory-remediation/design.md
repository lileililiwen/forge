## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| Dependency versions and the lockfile | `forge` | Yes |
| `cargo-audit-boost` / `cargo-matrix-boost` TUI code | `forge` | Yes, API adaptation only |
| The advisory exception and its reasoning | `forge` | Yes |
| The Gate `security` command and evidence | `forge` | No |
| An `openidconnect` replacement | future product decision | No |

## Why the Exception Is Reachable-Unreachable

`RUSTSEC-2026-0387` describes timing variability in `crypto-bigint` operations
"intended to be constant-time". The exploitable direction is a secret-dependent
operation: ECDSA signing with a private nonce, or private-key arithmetic.

Forge's only use of this graph is OIDC ID-token verification
(`src/identity/oidc.rs`, `browser_auth.rs`): it fetches the provider's public
keys and verifies a signature. No private key enters the process, no signing
operation is invoked, and the operands are public. The same reasoning already
justifies the `RUSTSEC-2023-0071` exception for `rsa`, which the repository has
carried since browser sign-in shipped.

The exception is not permanent. Its reason names the trigger that ends it: an
`openidconnect` release on `elliptic-curve 0.14` / `crypto-bigint ≥ 0.7.6`, or
a future advisory that makes the verification path itself unsafe.

## Alternatives Considered

| Option | Why not |
|---|---|
| Replace `openidconnect` | A new OIDC implementation is a product change: discovery, JWKS caching, nonce/state, claim validation and the browser flow all move. It is not a dependency bump. |
| `[patch.crates-io] elliptic-curve` | A patched crypto stack would hide the advisory from the graph while shipping an unreviewed fork. |
| Ignore in `deny.toml` only | CI runs both `cargo deny check` and `cargo audit`; one file would leave the other job red. |
| Leave the Gate blocked | The security check would stay red until an upstream release, which is how it was already being normalised. |

## The Upgrade Path

1. `cargo update -p tokio --precise 1.54.1` — one crate, so the update is
   attributable.
2. Set `ratatui = "0.30"` in both TUI crates; adapt imports and calls to the
   re-exported API. The facade re-exports the types both crates use, so the
   only source changes are `Frame::size()` → `Frame::area()` and
   `Table::highlight_style` → `Table::row_highlight_style`.
3. `cargo tree -i lru` and `cargo tree -i paste` must report no package.
4. `cargo audit` must report no vulnerability and no failing warning.
5. `cargo deny check` must pass with the exception recorded.
6. The `driftwatchdog` path dependency keeps its path and declares its version;
   a path-only entry is a wildcard to the `bans` check.

## Policy Parity

`deny.toml` and `.cargo/audit.toml` are read by different tools and used to
drift. The new contract test reads both:

- every vulnerability-class id ignored in one file appears in the other;
- every `deny.toml` ignore carries a non-empty `reason`;
- the `crypto-bigint` reason names `openidconnect` and the public-key-only
  boundary.

The test does not execute the scanners (they fetch an advisory database); it
keeps the hand-maintained policy honest.

## Failure Boundaries

- A `ratatui` API mismatch fails compilation in the affected crate, and its TUI
  tests fail.
- A lockfile resolution that reinstates `lru 0.12` or `paste` fails the audit
  run.
- A policy file edited without the other fails the parity test.
- The Gate still runs `cargo audit`; the exception is passed to that tool by
  its own configuration file, not by weakening the check.

## Verification Strategy

1. `cargo tree -i lru` and `cargo tree -i paste` return no package.
2. `cargo audit` exits 0 and still names the two excepted advisories as
   allowed.
3. `cargo test -p cargo-audit-boost -p cargo-matrix-boost`.
4. `cargo test --test dependency_policy_contract`.
5. The repository suite, formatting, clippy, OpenSpec validation and Gate.
6. `cargo deny check` when the tool is available; otherwise record the exact
   missing prerequisite and run `cargo audit`, which shares the advisory set.
