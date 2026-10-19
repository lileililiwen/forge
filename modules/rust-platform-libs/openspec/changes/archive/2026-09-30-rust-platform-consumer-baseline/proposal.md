# Proposal: A consumer baseline for the Rust platform

## Why

The Rust platform has crates but no adoptable baseline, so the portfolio has 20
repositories with a `Cargo.toml` and exactly one consumer (`trailCrew`, by a
hand-written `path = "../rust-platform-libs/crates/platform-core"`).

- No shared `rustfmt.toml`, `clippy.toml`, `deny.toml` or `.cargo/config.toml`
  exists; the three that do disagree (three formatting styles, two clippy
  profiles).
- No MSRV policy: declared `rust-version` ranges from 1.74 to 1.85, and four
  repositories declare none.
- Editions are mixed (2021 and 2024) even among platform-adjacent repositories.
- This repository's CI runs `cargo test --workspace` only — no formatting,
  clippy or dependency-policy gate.
- `publish = false`; there is no consumer recipe and no conformance harness.

## What Changes

- Ship a committed lint/format/dependency-policy baseline that a consumer adopts
  verbatim.
- Declare an MSRV policy, pin it with `rust-toolchain.toml`, and enforce it in
  CI.
- Document the consumer recipe that replaces hand-written paths.
- Extend CI to formatting, clippy (`-D warnings`), dependency policy, and tests.
- Add a conformance harness that packs and consumes a real consumer crate.

## BFS Impact Map

- **Capabilities:** workspace metadata, lint baseline, toolchain policy, CI,
  consumer adoption.
- **Consumers:** `trailCrew` and the future Rust consumers named in
  `HANDOFF.md`; no application behavior change.
- **Configuration:** workspace `Cargo.toml` (`[workspace.lints]`,
  `[workspace.package]`), new `rustfmt.toml`/`clippy.toml`/`deny.toml`,
  `rust-toolchain.toml`, CI workflow, `docs/`, and a conformance script.
- **Failure:** a consumer outside the MSRV, a lint regression, or a denied
  dependency must fail CI with a named cause.
- **Compatibility:** existing crates keep their public API; `publish = false`
  stays until a registry decision is made.

## Capabilities

- `rust-platform-consumer-baseline`.

## Non-goals

- No new foundation crate and no change to the provider-neutral boundary
  (no Axum, SQLx, cookie/session, OIDC/JWT, or product crate dependency).
- No consumer repository edits; migration is documented, not performed.
- No registry publication decision.
- No expansion of the contract fixture walker from 4 to 9 families — that is
  `contract-consumer-parity` in `platform-contracts`.
- No RBAC DSL, persistence, or business behavior.

## Package Boundary and Split Assessment

**Single outcome:** a Rust consumer can adopt the platform through one documented
baseline that CI enforces.

**Included:** workspace metadata and lints, toolchain pin, CI, the consumer
recipe document, and the conformance harness.

**Excluded:** the contract family parity work (platform-contracts), consumer
repositories, registry publication, and any new crate.

**Split signals considered:** lints, MSRV and conformance share one owner, one
lifecycle (the consumer build), and one oracle (conformance passes under the
pinned toolchain). Publishing to a registry is the only genuinely separable
concern and is explicitly deferred.

**Dependencies:** `rust-platform-foundation` (shipped, defines the crates).
Blocks the Rust side of `contract-consumer-parity`.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository | `openspec/specs/rust-platform-foundation/spec.md`, `[workspace.lints]` in `Cargo.toml` | foundation crates, lints block | no consumer-facing baseline; lints not shared | rust-platform-libs | **extend shared owner** |
| this repository | `crates/platform-arch-tests` | dependency-direction enforcement | already covers crate boundaries | rust-platform-libs | **adopt** |
| `trailCrew` | `Cargo.toml:38` hand-written path dependency | proof of adoption | not a baseline | trailCrew | **keep local** (consumer-side migration) |
| dotnet-platform-libs | `build/Platform.Consumer.props`, `scripts/conformance.sh` | the consumer-bootstrap and conformance *pattern* | different ecosystem; pattern only | dotnet-platform-libs | **adapt through a generic adapter** (mirror the pattern, not the code) |
| workspace-governance | `.ai-rules`, `templates/` | governance rules, not ecosystem baseline | out of scope | workspace-governance | **not applicable** |
| platform-contracts | `openspec/specs/contract-foundation/spec.md` | the schemas the parity walk consumes | owned by `contract-consumer-parity` | platform-contracts | **not applicable** |

## Evidence boundary

This change can prove that the baseline exists, that CI enforces it, and that the
conformance harness consumes a crate under the pinned toolchain. It cannot prove
that any consumer has adopted the platform in production.
