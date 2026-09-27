# ADR 0002: MSRV and toolchain floor

Date: 2026-09-27
Scope: `artifact-and-ci-baseline`
Status: accepted

## Context

`Cargo.toml` declares no `rust-version`. The toolchain floor lives only as
prose ("Rust stable, validated 1.98.1") in ADR 0001 and
`docs/release-readiness.md`. Nothing enforces it, and the `msrv` CI job does
not exist. The sibling runtime this repository drives (`driftwatchdog`)
pins `rust-version = "1.74"` with the rationale in a comment; this ADR
derives Forge's own floor from its real dependency graph rather than copying
that number.

## Derivation

The floor was derived from the real graph on 2026-09-27, and live
verification disproved the proposal's first guess (`clap 4` at 1.74,
mirroring the sibling): two stronger requirements bind instead.

| Requirement | Source | Minimum |
| --- | --- | --- |
| `OsStr::display` (`os_str_display`) used in `src/agent/mod.rs` | Forge's own code | 1.87 (stabilized 1.87.0) |
| Edition-2024 transitive manifests (`clap_lex 1.1.1` via `clap 4.6.7`) | dependency graph | cargo 1.85 (lockfile v3 keeps the manifest parseable; v4 would need 1.78+) |
| `clap 4`, `rusqlite 0.32`, `thiserror 2`, `serde 1`, others | direct floors | ≤ 1.74 |

Proven locally: `cargo +1.74 check` fails (edition-2024 manifest
unparseable), `cargo +1.85.0 check` fails (`E0658 os_str_display` in
`src/agent/mod.rs:1723,1748`), `cargo +1.87 check --workspace
--all-targets` passes, as does stable 1.98.1. The declared floor is
therefore **`rust-version = "1.87"`**, binding on Forge's own
`OsStr::display` use.

## Decisions

1. Declare `rust-version = "1.87"` in `[package]`.
2. Enforce it with an `msrv` CI job running
   `cargo check --workspace --all-targets` on a toolchain exactly at the
   floor (mirrors the sibling's approach).
3. Raise the floor only together with a dependency-floor bump (or a new
   stabilized-feature use in Forge's own code) and a green `msrv` job;
   record the previous floor (none before this ADR) and the binding
   requirement in the bump commit.
4. No `rust-toolchain.toml` pin: forcing a toolchain on contributors would
   silently change the local build the docs describe. The floor plus the CI
   job is the enforceable form, matching the sibling (no `rust-toolchain`
   file).
5. Keep `Cargo.lock` at `version = 3`: the v4 encoding is unreadable below
   1.78, which would make the floor claim false for lockfile parsing
   before compilation even starts. Stable toolchains preserve the v3
   version on rebuild (verified: `cargo build` leaves `version = 3`).

## Consequences

- A toolchain below 1.74 fails `msrv` instead of the claim surviving in docs.
- ADR 0001's recorded commands and the README quickstart point at the same
  entry points (`scripts/release-check.sh`, `.github/workflows/ci.yml`).
- Previous floor: none. Binding requirement: `OsStr::display` in
  `src/agent` (stabilized 1.87.0); dependency-graph minimum is cargo 1.85
  (edition-2024 transitive manifests).

## Verification

`cargo check --workspace --all-targets` on Rust 1.74; the CI `msrv` job;
`node scripts/check-openspec-change-names.mjs`;
`openspec validate --all --strict --no-interactive`; `git diff --check`.
