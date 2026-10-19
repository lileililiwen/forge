# Design: Post-MVP readiness (Rust platform crates)

## Boundary paths

| Path | Change |
|---|---|
| `README.md` | Add per-crate matrix + versioning/license/publish/MSRV columns, usage pointer, license pointer, adoption guide |
| `docs/usage.md` (new) | Minimal usage example per crate; how examples are verified; test-only crates excluded |
| `crates/platform-core/examples/*.rs` (new) | Compile-checked example (bounded IDs, clock, safe error) |
| `crates/platform-contracts/examples/*.rs` (new) | Compile-checked example (envelope + compatibility validation) |
| `crates/platform-identity/examples/*.rs` (new) | Compile-checked example (subject, permission evaluator, tenant resolver) |
| `crates/platform-audit/examples/*.rs` (new) | Compile-checked example (audit event builder + redaction) |
| `LICENSE` (new) | Apache-2.0 text matching `[workspace.package].license` |
| `Cargo.toml`, `crates/*/src/**`, `openspec/specs/**`, `crates/platform-testing/**`, `crates/platform-arch-tests/**`, `Cargo.lock` | Untouched |

## Language / runtime + commands

- Language/runtime: Rust, workspace `edition = "2021"`, `rust-version = "1.75"`, `version = "0.1.0"`, `license = "Apache-2.0"`, `publish = false`; `unsafe_code = "forbid"`.
- Primary verification: `cargo test --workspace` (compiles and runs examples as part of the workspace).
- Recorded full verification surface from `HANDOFF.md`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `openspec validate --specs --strict --no-interactive`, plus the cross-repository fixture parity walk in `crates/platform-contracts/tests/fixture_parity.rs` (needs `platform-contracts` resolved).
- Shared lint/MSRV/CI/toolchain baseline is out of scope and owned by `rust-platform-consumer-baseline`; docs point to it rather than re-declaring it.

## Ownership

Owner: Rust platform maintainers. Workspace configuration is owned by the active consumer-baseline change; this package must not race it. The single proven consumer (`trailCrew`) is documented, never edited. No shared-owner transfer.

## Behavioral table

| # | Behavior | Documented state | Proof rule |
|---|---|---|---|
| B1 | README matrix lists every crate with purpose, production/test-only posture, version, license, publish, MSRV, edition | Implemented | Matrix matches `Cargo.toml` + each crate manifest |
| B2 | `docs/usage.md` gives one minimal example per production crate | Implemented | Embedded snippet matches a compiling `examples/*.rs` |
| B3 | Examples compile and stay dependency-clean | Implemented | `cargo test --workspace` green; `platform-arch-tests` green |
| B4 | Root LICENSE is Apache-2.0, matching the workspace declaration | Implemented | File present; identifier matches `Cargo.toml` |
| B5 | Adoption guide documents the path-dependency route and links the baseline recipe | Implemented | No duplicated recipe; link resolves |
| B6 | No workspace/config/API change; no consumer edit | Enforced | Diff limited to README, `docs/`, LICENSE, `examples/`, this change |

## Contract / compatibility

No public API, serialization, or trait surface changes; examples only consume existing APIs. `publish = false` and version `0.1.0` are restated, not changed. The dependency-direction guarantee (`platform-arch-tests`) is preserved: examples MUST NOT introduce `axum`, `sqlx`, `cookie`, `jsonwebtoken`, `openidconnect`, `oauth2`, `reqwest`, `tokio`, or a product crate into any production crate.

## Failure policy

- A non-compiling example fails review; it is fixed or removed, never documented as working.
- A README claim that disagrees with a manifest (version/license/publish/MSRV) fails review and is corrected from the manifest.
- Any `Cargo.toml`, `Cargo.lock`, or `src/` edit fails review (out of boundary; belongs to the baseline change).
- Any forbidden dependency in a production crate fails `platform-arch-tests` and blocks the change.
- Registry publication MUST NOT be asserted; `publish = false` is stated as-is.
- Stale test counts from `HANDOFF.md` are refreshed or restated as "last recorded", never presented as current without a run.

## Verification oracle (exact commands)

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
node scripts/check-openspec-change-names.mjs
openspec validate --changes --strict --no-interactive
openspec validate --specs --strict --no-interactive
git diff --check
```

Manual oracle: README matrix read against `Cargo.toml` and each `crates/*/Cargo.toml`; every `docs/usage.md` snippet exists as a compiling example; `LICENSE` exists with the Apache-2.0 identifier.

## Decision ledger

| Decision | Status | Blocker / next action |
|---|---|---|
| Registry publication | **Undecided — do not guess** | `publish = false` stays; a registry decision is a separate change |
| MSRV value and pin (`rust-toolchain.toml`) | **Owned by `rust-platform-consumer-baseline`** | Read and point; do not decide here. Record current value `1.75` from the manifest |
| Example location (per-crate `examples/` vs doctests) | Decided | Per-crate `examples/` (non-production, workspace-verified); doctests deferred |
| Whether to document the baseline recipe here | Decided | No — point to the active change to keep one source of truth |
| Test-only crates get examples | Decided | No; `platform-testing`/`platform-arch-tests` are excluded |
| Concurrent active changes | **Noted** | `rust-platform-consumer-baseline` remains active; this package must not edit workspace config or archive order |
