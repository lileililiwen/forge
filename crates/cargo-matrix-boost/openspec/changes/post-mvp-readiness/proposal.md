# Proposal: Post-MVP readiness — visual evidence, toolchain matrix, CI recipe, error catalog

## Why

`cargo-matrix-boost` is a usable cargo plugin: it installs with `cargo install --path .`, runs a
feature × toolchain matrix, renders a TUI grid and a self-contained HTML dashboard, and offers a
`--demo` simulation mode. It is MIT-licensed and has a CI workflow. What it lacks is *evaluability*:
a stranger reading the repository cannot see what the TUI or HTML report look like before installing,
cannot learn which toolchain/feature combinations are actually exercised, has no copy-paste CI recipe,
and has no catalog of what failure looks like. Post-MVP readiness means the repository alone is enough
to evaluate, trust, and integrate the tool.

## What Changes

- Add reproducible visual evidence: TUI screenshot(s), HTML dashboard screenshot(s), and a short
  `--demo` walkthrough GIF, all generated from the repository's own `--demo` mode.
- Add a toolchain × feature matrix document describing stable/beta/nightly support, feature-powerset
  resolution, defaults, and how to read the result grid.
- Add a CI integration recipe for headless matrix runs using the existing `--no-open --output-dir`
  flags, including artifact retention of `report.json` / `report.html`.
- Add an error-case catalog of observed failure modes (missing rustup toolchain, no resolvable
  features, `cargo metadata` failure, browser-open failure) with the exact output and next action.
- Cross-link the above from `README.md` without changing CLI behavior, report schema, or licensing.

This is a documentation-and-evidence change. No runtime behavior is modified.

## Package Boundary and Split Assessment

This change is **one package** (`post-mvp-readiness`), not a split.

- All four foci (screenshots/GIF, toolchain matrix, CI recipe, error catalog) are repository-local
  documentation and evidence that share a single entry point (`README.md`) and a single evidence
  pipeline (`cargo matrix-boost --demo` → `target/matrix/report.json` + `report.html`).
- None of them changes runtime behavior, so none has an independent verification surface that would
  justify a separate change. Splitting would produce three or four near-empty changes that duplicate
  the same BFS context and the same oracle command.
- Candidate split considered and rejected: extracting a shared "cargo plugin docs/evidence" convention
  across `cargo-audit-boost` and `cargo-sonar`. That is a cross-repository architecture decision and is
  a Non-goal here.
- Candidate future split recorded: if the error-case catalog requires *new machine-readable exit codes*
  or a `--json` error output, that is a behavior change and MUST move to a separate feature change
  (`matrix-diagnostics` or equivalent), not be smuggled into this documentation change. This is tracked
  in `design.md` → Ledger.

## Sibling and Shared Architecture Reconnaissance

| Sibling / shared surface | Relationship to this repo | Decision | Reason |
| --- | --- | --- | --- |
| `cargo-audit-boost` | Sibling Rust cargo plugin in the same workspace conventions | Keep-local | Each plugin owns its own README, CI, and release cadence; no shared crate is imported. |
| `cargo-sonar` | Sibling Rust cargo plugin in the same workspace conventions | Keep-local | Same reason; extracting shared reporting/doc crates would be an architecture change. |
| `rust-platform-libs` | Workspace Rust library area | Keep-local | This repo has no compile-time dependency on it; adding one is out of scope. |
| `platform-contracts` (`platform.readiness`) | Language-neutral readiness schema owned by per-service runtimes | Keep-local | `cargo-matrix-boost` is a one-shot CLI, not a long-running service, and exposes no `/healthz` or `/readyz`; adopting the contract is out of scope. |
| `.ai-rules` / `.ai-gate` | Workflow conventions copied per repository | Keep-local | Conventions are copied, not imported; this change does not alter them. |
| Workspace root `Directory.Build.props` / `nuget.config` | .NET-only workspace configuration | Not applicable | Non-MSBuild projects ignore these files; this repo is pure Cargo. |

Default is **keep-local** for every row: the readiness work is repository-scoped documentation and
evidence, and any shared-artifact adoption would require its own approved change.

## BFS Impact Map

| Surface | Impact | Kind |
| --- | --- | --- |
| `README.md` | Add evidence links, matrix/CI/error-catalog links; keep the existing 69-line structure | Documentation |
| `docs/toolchain-matrix.md` (new) | Document toolchains, feature powersets, defaults, result semantics | Documentation |
| `docs/ci-integration.md` (new) | Document headless `--no-open --output-dir` recipe and artifact retention | Documentation |
| `docs/error-catalog.md` (new) | Document observed failure modes with exact output | Documentation |
| `docs/assets/` or `assets/` (new) | TUI + HTML screenshots and `demo.gif` generated from `--demo` | Evidence asset |
| `src/lib.rs`/`main.rs`/`demo.rs`/`exporter.rs`/`ui.rs` | Read-only; used to regenerate evidence; no code change expected | Verification-only |
| `.github/workflows/ci.yml` | Read-only reference for the CI recipe; no workflow behavior change | Verification-only |
| `Cargo.toml` | Possibly `homepage`/`documentation` metadata for asset location only; no dependency change | Metadata (optional) |
| `openspec/` | New `post-mvp-readiness` change and new `readiness` capability | Process |
| Sibling repositories | None | None |

## Capabilities

### New Capabilities

- `readiness`: post-MVP evaluability of a cargo plugin — a runnable clean-clone quickstart, reproducible
  visual evidence, a documented toolchain × feature matrix, a headless CI integration recipe, and an
  error-case catalog.

### Modified Capabilities

- None. No existing spec-level behavior changes; this change adds documentation and evidence only.

## Non-goals

- No new CLI flags, subcommands, or changes to matrix resolution, execution, or report schema.
- No crates.io publication, release tagging, or hosted demo site.
- No shared crate extraction from sibling cargo plugins or adoption of `platform-contracts`.
- No relicense; the project remains MIT.
- No mandatory Windows/macOS screenshot parity; platforms that cannot be captured are recorded as
  blocked, not invented.