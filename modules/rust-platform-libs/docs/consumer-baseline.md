# Consumer baseline for the Rust platform.
#
# This document is the adoption recipe. A Rust consumer adopts the
# platform by copying the baseline files verbatim, declaring the MSRV
# the workspace declares, and depending on the platform crates through
# one of the two forms below. `scripts/conformance.sh` proves the
# recipe works; CI runs it on every push.

## 1. Baseline files

| File | Purpose |
|---|---|
| `rustfmt.toml` | Stable-only rustfmt settings (`cargo fmt --check` is warning-free on the pinned toolchain) |
| `clippy.toml` | Clippy settings; `msrv` matches the declared MSRV so lints never suggest newer features |
| `deny.toml` | `cargo deny` policy: permissive-license allowlist, RustSec advisories, no wildcard versions, crates.io only |
| `rust-toolchain.toml` | Pins the exact toolchain CI and developers share |
| `[workspace.lints]` in `Cargo.toml` | `unsafe_code = "forbid"` plus the workspace clippy level |

Copy all four files verbatim. Do not fork them: a forked baseline is
an untested baseline.

## 2. MSRV policy

- Declared MSRV: `rust-version = "1.75"` in the workspace `Cargo.toml`.
- Pinned toolchain: `channel = "1.95.0"` in `rust-toolchain.toml` —
  the lowest installed stable toolchain that builds the workspace
  today.
- Effective floor: the transitive `time 0.3.55` dependency requires
  rustc >= 1.88, so the workspace does **not** build on a 1.75–1.87
  compiler today even though the declared MSRV says 1.75.
- Raising the declared MSRV is a **separate OpenSpec change with a
  migration note**, never a silent bump. Lowering the effective floor
  (e.g. downgrading `time`) is likewise its own change.

A consumer below the declared MSRV gets a build failure naming the
required version (`package ... requires rustc ...`); the conformance
harness proves the mechanism with a deliberate `rust-version = "1.99"`
fixture.

## 3. Adoption recipe

### In-workspace development (path dependency)

A crate that lives in this workspace depends on the platform by path,
exactly as `tests/consumer-fixture/Cargo.toml` does:

```toml
[dependencies]
platform-core = { path = "../../crates/platform-core" }
platform-identity = { path = "../../crates/platform-identity" }
platform-audit = { path = "../../crates/platform-audit" }
```

The hand-written `path = "../rust-platform-libs/crates/platform-core"`
form that `trailCrew` uses predates this recipe and refers to the old sibling
checkout. It is a legacy consumer migration item, not a supported path for new
consumers; migrate it before retiring that origin. New in-workspace consumers
SHOULD use the workspace-member form above.

### Out-of-workspace adoption (versioned git dependency)

The crates are publishable but the first crates.io release is blocked by the
owner gates in `docs/publication.md`. Until publication lands, an external
consumer can depend on an explicitly released, versioned Forge git tag, never
a hard-coded sibling path:

```toml
[dependencies]
platform-core = { git = "https://github.com/lileililiwen/forge", tag = "rust-platform-libs-v0.1.0", package = "platform-core" }
```

The tag is created only by the owner after the release preconditions are met.
Consumer-side migration is each consumer's own OpenSpec change; this module
never edits a consumer.

> Offline note: `scripts/conformance.sh` cannot reach a git remote,
> so its out-of-workspace probe depends on the workspace through an
> absolute `path` instead. The script says so explicitly where it
> does it. The documented contract is the versioned git form above.

## 4. CI enforcement

CI (`.github/workflows/ci.yml`) uses the pinned toolchain (no
toolchain input — `rust-toolchain.toml` governs) and runs, in order:

| Check | Command | Failure names |
|---|---|---|
| Formatting | `cargo fmt --all -- --check` | the misformatted file |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | the lint |
| Dependency policy | `cargo deny check` | the advisory / license / banned crate |
| Tests | `cargo test --workspace` | the failing test |
| Conformance | `./scripts/conformance.sh` | the fixture and the cause |

A lint regression, a denied dependency or license, a consumer below
the MSRV, or a fixture that cannot resolve a platform crate fails CI
with a named cause. `cargo deny check` passes today with one standing
warning (duplicate `getrandom` 0.3.4/0.4.3 via `jsonschema` vs `uuid`);
duplicates are `warn`, not `deny`, so the check stays green while the
duplication is visible.

## 5. Conformance harness

Run `./scripts/conformance.sh` from the repository root. It proves five
things and fails loudly on any of them:

1. **Pinned toolchain** — `rustc --version` inside the workspace
   resolves to the `rust-toolchain.toml` channel, and the channel
   satisfies the declared MSRV.
2. **In-workspace consumer** — `tests/consumer-fixture` (the
   identity → permission → audit path) builds and tests green, and
   its manifest uses only workspace-relative `path` dependencies.
3. **Out-of-workspace consumer** — a scratch crate outside the
   workspace copies the four baseline files verbatim, resolves
   `platform-core`, and builds, tests, formats, and lints clean
   under the copied pin.
4. **Lint regression fails loudly** — a scratch crate with an unused
   variable fails `cargo clippy -- -D warnings` naming
   `unused-variables`.
5. **MSRV enforcement fails loudly** — a scratch crate with
   `rust-version = "1.99"` fails to build naming the required
   version.
6. **Denied dependency fails loudly** — `cargo deny check bans`
   against a probe config banning `serde` fails naming the crate.

(Yes, six proofs — the toolchain assertion plus the five fixtures the
change tasks require.)

## 6. What this baseline does not do

- No new foundation crate; no change to the provider-neutral boundary
  (`platform-arch-tests` still forbids Axum, SQLx, cookie/session,
  OIDC/JWT, and product crates in production crates).
- No consumer repository edits; migration is documented, not performed.
- No registry publication decision (`publish = false` unchanged).
- No contract-family parity expansion (owned by
  `contract-consumer-parity` in `platform-contracts`).
