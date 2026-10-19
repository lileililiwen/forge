# Design: A consumer baseline for the Rust platform

## 1. Implementation boundary

**Repository:** `/home/paul/code/rust-platform-libs` — Rust, edition as pinned by
the workspace, MSRV declared by this change.

**Files to change:** workspace `Cargo.toml` (`[workspace.package]`,
`[workspace.lints]`), new `rustfmt.toml`, `clippy.toml`, `deny.toml`,
`rust-toolchain.toml`, `.github/workflows/ci.yml`, a new
`scripts/conformance.sh` (or equivalent) plus a consumer fixture crate under
`tests/`, `docs/consumer-baseline.md`, and `ROADMAP.md`/`HANDOFF.md` updates.

**Must not change:** crate public APIs, the provider-neutral boundary enforced by
`platform-arch-tests`, or `publish = false`.

## 2. Language and runtime

Rust. The MSRV is declared in `[workspace.package] rust-version` and pinned by
`rust-toolchain.toml`; stable is the default channel.

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo deny check`
- `cargo test --workspace`
- `./scripts/conformance.sh`
- `openspec validate --changes --strict --no-interactive`

## 3. Ownership and shared code

Repository-local. Consumers adopt the baseline verbatim; they do not fork it.

- **Dependency direction:** platform crates never depend on a consumer; a
  consumer depends on `platform-*` by path (in-workspace development) or by a
  versioned git dependency (out-of-workspace), never by a hard-coded sibling
  path.
- **Smallest adoptable boundary:** four configuration files plus one toolchain
  pin that a consumer can copy.
- **Release boundary:** ships with the repository; `publish = false` is
  unchanged.

## 4. Behavioral model

| Consumer state | Result |
|---|---|
| toolchain older than the MSRV | `rust-toolchain.toml` selects the pinned toolchain; a build on an older compiler fails with the MSRV message |
| formatting drift | `cargo fmt --check` fails |
| lint regression | `clippy -D warnings` fails |
| denied dependency or license | `cargo deny check` fails naming the advisory/licence |
| in-workspace consumer | path dependency, as `trailCrew` does |
| out-of-workspace consumer | versioned git dependency per the documented recipe |
| `publish = false` | unchanged; no registry publication |

## 5. Contract and compatibility

- The baseline is the exported contract: the lint set, the MSRV value, and the
  consumer recipe.
- **Compatibility:** existing crates build unchanged under the pinned toolchain;
  `[workspace.lints]` is additive. If the MSRV must rise, that is a separate
  change with a migration note, not a silent bump in this one.
- Documentation replaces the hand-written path recipe for consumers; the
  in-workspace path form stays supported for development.

## 6. Failure and boundary policy

| Case | Result |
|---|---|
| Missing `rust-toolchain.toml` | CI installs stable and fails the MSRV check |
| New lint warning | CI fails; no allow without a recorded justification |
| Denied dependency | CI fails naming advisory/licence and the crate path |
| Conformance fixture cannot resolve the crate | conformance fails naming the version/path |
| Registry publication attempted | out of scope; no workflow publishes |

## 7. Verification oracle

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo deny check`, `cargo test --workspace` all pass on the pinned
  toolchain.
- `./scripts/conformance.sh` builds and tests a consumer fixture against this
  workspace and asserts the fixture resolves platform crates without a hard-coded
  path outside the workspace.
- A deliberately lint-breaking, MSRV-breaking, and deny-breaking fixture each
  fail with the expected named cause.
- `openspec validate --changes --strict --no-interactive` passes.

## 8. Decision ledger

**Assumptions:** stable toolchain; MSRV is a policy the maintainer chooses (not
inferred); consumers migrate on their own schedule; publication is undecided.

**Resolved alternatives:**

- (a) Copy the dotnet bootstrap pattern vs import it → chose the **pattern**;
  the ecosystems share no configuration.
- (b) Pick the lowest existing MSRV (1.74) vs a new policy → chose an **explicit
  policy** recorded in the design and pinned by `rust-toolchain.toml`, with a
  separate change required to raise it.
- (c) Publish to crates.io vs keep `publish = false` → kept **unpublished**;
  publication is a separate decision with its own change.
- (d) Extend the parity walker here vs in platform-contracts → chose
  **platform-contracts** (`contract-consumer-parity`), to keep one contract
  owner.

**Deferred:** consumer-side migration; registry publication; parity expansion.

**Blockers:** none.
