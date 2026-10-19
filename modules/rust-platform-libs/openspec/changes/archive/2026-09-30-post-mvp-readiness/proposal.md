# Proposal: Post-MVP readiness for the Rust platform crates

## Why

The 61-line README names the six crates in prose but gives a consumer no per-crate matrix (version, license, publish posture, MSRV, edition), no usage examples, and no license file even though `Cargo.toml` declares `license = "Apache-2.0"` and every crate inherits `license.workspace = true`. A reader cannot tell which crates are runnable, what `publish = false` means, how to call the public API, or how to adopt the platform beyond a hand-written `path =` dependency. The only proven consumer is `trailCrew`, and the consumer-facing recipe, lint baseline, MSRV pin, and conformance harness are owned by the already-active `rust-platform-consumer-baseline` change, so this package must complement rather than duplicate that work.

## What Changes

- Add a per-crate matrix to `README.md`: crate, purpose, whether it is production or test-only, `version = 0.1.0` (workspace), `license = "Apache-2.0"` (workspace), `publish = false`, `rust-version = "1.75"` (workspace), `edition = "2021"`.
- Add `docs/usage.md` with a minimal, compile-checked usage example per crate, plus a note on how the examples are verified.
- Add compile-checked examples under `crates/*/examples/` where a crate can be exercised without a consumer (test-only crates excluded); `cargo test --workspace` MUST stay green.
- Add a root `LICENSE` with the Apache-2.0 text that the manifest already declares, and document per-crate inheritance.
- Add a consumer adoption guide section that documents the current path-dependency adoption (as in `trailCrew`) and points to `rust-platform-consumer-baseline` for the shared lint/MSRV/CI recipe — this change does not re-declare it.
- No workspace `Cargo.toml`, crate public API, spec, or consumer change.

## Package Boundary and Split Assessment

Single-package change: `README.md`, `docs/usage.md`, a root `LICENSE`, and lightweight `crates/*/examples/*.rs` files. No workspace-manifest edit, no crate public-API edit, no consumer edit.

Assessed and rejected splits:

- **README matrix vs usage examples vs license:** one readiness narrative; the matrix names the crates, the examples prove the API, and the license makes the manifest claim true. Splitting triples review for coupled docs.
- **Compile-checked examples as a separate change:** rejected. Examples are non-production, non-API artifacts; keeping them in this package lets `cargo test --workspace` verify them directly.
- **Consumer adoption guide as a separate change:** rejected. The *recipe* (lint/MSRV/CI/conformance) is owned by `rust-platform-consumer-baseline`; this package documents only the currently-proven path-dependency adoption and links the recipe. Re-declaring it here would duplicate a single source of truth.
- **License per crate vs root license:** rejected. All crates inherit `license.workspace = true`; one root Apache-2.0 LICENSE plus the matrix is the correct shape.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository — active change | `openspec/changes/rust-platform-consumer-baseline/` (`[workspace.lints]`, MSRV, CI, recipe, conformance) | the consumer recipe, lint/MSRV/CI baseline | already owns the shared baseline; not yet shipped | rust-platform-libs — consumer-baseline change | **keep-local + point** (reason: do not duplicate or pre-empt an active change; document path-dependency adoption only) |
| this repository — specs | `openspec/specs/rust-platform-foundation`, `rust-identity-audit-permission` | the public surface to document | README lists crates but not a matrix or examples | rust-platform-libs | **extend** (cite, do not edit) |
| this repository — `crates/platform-arch-tests` | lockfile dependency-direction tests | boundary enforcement the examples must not break | examples must not pull forbidden deps | rust-platform-libs | **adopt** (run it, change nothing) |
| `trailCrew` | `Cargo.toml` hand-written `path = "../rust-platform-libs/crates/platform-core"` | the only real adopter | not a baseline; consumer-side migration | trailCrew | **document only** (no consumer edit) |
| `dotnet-platform-libs` | per-crate README/versioning + consumer-bootstrap pattern | the matrix/versioning *pattern* | different ecosystem | dotnet-platform-libs | **adapt the pattern, not the code** |
| `workspace-governance` | `.project.json` `kind: shared`, `profile: rust-platform`; change-name rule | the declaration and vocabulary | no README/license contract to extend | workspace-governance | **consume only** |
| `driftwatchdog` | gate runtime | failure-memory gate | no gate wiring change | driftwatchdog | **not applicable** |
| `platform-contracts` | schema/parity repository | the schemas the crates mirror | parity expansion is another change | platform-contracts | **not applicable** |

Keep-local default: every artifact lives in `rust-platform-libs`. No consumer, sibling, or workspace-manifest change.

## BFS Impact Map

| Surface | Impact |
|---|---|
| `README.md` | Add per-crate matrix (purpose/version/license/publish/MSRV/edition), usage pointer, license pointer, adoption guide pointing at the baseline change |
| `docs/usage.md` (new) | Minimal usage example per crate; stated verification method |
| `crates/platform-core/examples/*.rs` (new, illustrative) | Compile-checked bounded ID/clock/error example |
| `crates/platform-contracts/examples/*.rs` (new) | Compile-checked envelope/validation example |
| `crates/platform-identity/examples/*.rs` (new) | Compile-checked subject/permission/tenant example |
| `crates/platform-audit/examples/*.rs` (new) | Compile-checked audit-event/redaction example |
| `LICENSE` (new) | Apache-2.0 text making the workspace license claim true |
| `Cargo.toml`, `crates/*/src/**`, `crates/platform-testing`, `crates/platform-arch-tests` | Untouched (test-only crates get no example; baseline change owns workspace config) |

## Capabilities

- A consumer can read one matrix and know each crate's purpose, stability posture, version, license, and MSRV.
- Each production crate has a minimal, compile-checked usage example.
- The Apache-2.0 license claim is true on disk and documented per crate.
- Adoption is documented for the one proven consumer without pre-empting the baseline change's recipe.

## Non-goals

- No `Cargo.toml`/`Cargo.lock` edits, no new crate, no public-API or behavior change.
- No lint/clippy/deny/MSRV/CI/toolchain baseline (owned by `rust-platform-consumer-baseline`).
- No registry publication decision; `publish = false` stays.
- No Axum/SQLx/cookie/OIDC/JWT/HTTP or product dependency in any production crate.
- No contract-family parity expansion (owned by `platform-contracts`).
- No consumer or sibling repository edits.
