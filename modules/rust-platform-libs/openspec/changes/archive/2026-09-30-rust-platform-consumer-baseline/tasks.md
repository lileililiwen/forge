# Tasks: A consumer baseline for the Rust platform

## 0. Caveats and confirmations

- [x] **The MSRV is a policy decision, not a measurement.** Maintainer
  confirmed: declared MSRV stays `1.75`; the pin is `1.95.0` (lowest
  installed stable toolchain that builds — `time 0.3.55` requires
  rustc >= 1.88, so the effective floor is 1.88; recorded in
  `docs/consumer-baseline.md` §2). Raising the declared MSRV later
  needs a separate change and a migration note.
- [x] **Record environment-blocked checks.** None blocked: `cargo-deny
  0.20.2`, `openspec 1.6.0`, and toolchains 1.74/1.85.0/1.87/1.95.0 +
  stable were all present. Every check below ran for real.
- [x] **The 4→9 contract-family parity is out of scope.** Untouched;
  still owned by `contracts-consumer-parity` in `platform-contracts`.
- [x] **Do not edit consumers.** No sibling repository touched; the
  only new consumer-like crate is the in-repo `tests/consumer-fixture`.
- [x] **Keep `publish = false`.** Unchanged in every manifest.
- [x] **No silent lint allowances.** `[workspace.lints]` untouched; no
  `allow` added anywhere for this change.
- [x] **Preserve provider neutrality.** `platform-arch-tests` passes
  unchanged; no Axum, SQLx, OIDC/JWT or product dependency entered a
  production crate.
- [x] **Stage only this change directory.** `.project.json` was already
  modified and `.github/` untracked before this change; the change's
  commits stage only the baseline files listed in §2 plus the archive
  move.

## 1. BFS — Baseline and impact coverage

- [x] Inventory the workspace `Cargo.toml`, the existing `[workspace.lints]`,
  `platform-arch-tests`, and the current CI workflow.
- [x] Record the chosen MSRV and confirm every crate builds under the pinned
  toolchain before changing CI.
- [x] Add conformance fixtures: an in-workspace consumer, an out-of-workspace
  consumer, a lint-breaking case, an MSRV-breaking case, and a denied-dependency
  case.
- [x] Confirm `publish = false` and the provider-neutral boundary are untouched.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add `rustfmt.toml`, `clippy.toml`, `deny.toml` and the `[workspace.lints]`
  baseline.
- [x] Add `rust-toolchain.toml` and the MSRV policy in
  `[workspace.package].rust-version`.
- [x] Extend CI to formatting, clippy `-D warnings`, `cargo deny check`, and
  tests.
- [x] Write `docs/consumer-baseline.md` with the adoption recipe (in-workspace
  path and out-of-workspace versioned dependency).
- [x] Implement `scripts/conformance.sh` and the consumer fixture.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-run the full crate test suite and `platform-arch-tests` under the pinned
  toolchain.
- [x] Verify the lint baseline does not require allow-listing existing code
  silently; record any justified allow.
- [x] Verify a consumer following the documented recipe resolves the platform
  without any hard-coded sibling path.
- [x] Update `ROADMAP.md` and `HANDOFF.md`.

## 4. Verification

- [x] `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D
  warnings && cargo deny check && cargo test --workspace`.
- [x] `./scripts/conformance.sh`.
- [x] `node scripts/check-openspec-change-names.mjs` (if present) and
  `openspec validate --changes --strict --no-interactive`.
- [x] Record actual outputs; report any environment-blocked check (for example a
  missing `cargo-deny`) with its exact next action rather than as passing.
