# Proposal: Post-MVP readiness — visual evidence, profile interpretation, CI recipe, error catalog

## Why

`cargo-audit-boost` is a usable cargo build profiler: it installs with
`cargo install --path .`, runs a real build under an interactive TUI, renders a
self-contained HTML report (Gantt timeline, concurrency graph, diagnostics
browser), offers a `--demo` simulation mode, and is MIT-licensed. What it lacks
is *evaluability*: a stranger reading the repository cannot see the TUI or the
HTML report before installing, cannot interpret `report.json` without reading
`exporter.rs`, has no copy-paste CI recipe for headless runs, and has no catalog
of what failure looks like. Post-MVP readiness means the repository alone is
enough to evaluate, trust, and integrate the tool.

## What Changes

- Add reproducible visual evidence: TUI screenshot(s), HTML report
  screenshot(s), and a short `--demo` walkthrough GIF, generated from the
  repository's own `--demo` mode and stored with the command that produced them.
- Add `docs/profile-output.md` describing the `report.json` shape
  (`total_duration_secs`, `total_crates`, `max_concurrency`, `success`,
  `bottlenecks`, `crates[]`) and how to read the Gantt, cached, bottleneck, and
  concurrency views in `report.html`.
- Add `docs/ci-integration.md` with a headless recipe for profiling a real build
  in CI and retaining `target/build-scan/report.json` / `report.html`.
- Add `docs/error-catalog.md` of observed failure modes (no Cargo project,
  invalid cargo arguments, build failure, TUI without a TTY, unwritable target
  directory) with exact output and next action.
- Align package metadata with the shipped MIT license (`Cargo.toml` currently
  declares no `license` field while `LICENSE` and the README say MIT).
- Cross-link the above from `README.md` without changing CLI behavior, report
  schema, or licensing.

This is a documentation-and-evidence change. No runtime behavior is modified.

## Package Boundary and Split Assessment

This change is **one package** (`post-mvp-readiness`), not a split.

- All foci (screenshots/GIF, profile-output interpretation, CI recipe, error
  catalog, metadata alignment) are repository-local documentation, evidence, and
  a one-line metadata fix sharing a single entry point (`README.md`) and a
  single evidence pipeline (`cargo-audit-boost --demo` → `target/build-scan/`).
- None changes runtime behavior, so none has an independent verification surface
  that would justify a separate change.
- Candidate split considered and rejected: extracting a shared "cargo plugin
  docs/evidence" convention with `cargo-matrix-boost` / `cargo-sonar`. That is a
  cross-repository architecture decision and is a Non-goal here.
- Candidate future split recorded: if the error catalog requires *new exit
  codes* or a `--json` error output, that is a behavior change and MUST move to
  a separate feature change. Tracked in `design.md` → Ledger.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository | `src/main.rs`, `src/demo.rs`, `src/exporter.rs`, `src/ui.rs`, `README.md`, `LICENSE` | `--demo`, TUI, `target/build-scan/report.{json,html}`, MIT | no screenshots, no interpretation doc, no CI recipe, no error catalog, `Cargo.toml` lacks `license` | cargo-audit-boost | **extend shared owner** |
| cargo-matrix-boost | `openspec/changes/post-mvp-readiness/` | a sibling post-MVP-readiness package (structure precedent only) | different CLI, output dir, and report schema; no shared code | cargo-matrix-boost | **keep local** — precedent, not a dependency |
| cargo-sonar | `src/exporter.rs`, `README.md` | sibling cargo plugin with a `--demo` HTML dashboard | different tool set and report model | cargo-sonar | **keep local** — no shared crate is imported |
| rust-platform-libs | workspace crates | platform libraries | this repo has no compile-time dependency on it; adding one is out of scope | rust-platform-libs | **keep local** |
| workspace-governance | `projects.json`, `templates/project/**` | registry, shared rules templates | conventions are copied per repository; governance does not own plugin docs | workspace-governance | **keep local** |
| driftwatchdog | gate runtime | CI gate for downstream products | not consumed by this repo's `cargo test` verification; no readiness contract | driftwatchdog | **keep local** |
| platform-contracts (`platform.readiness`) | `schemas/readiness.schema.json` | service `/healthz` + `/readyz` envelope | this is a one-shot CLI, not a long-running service; exposes no health endpoint | platform-contracts | **keep local** — adopting the contract is out of scope |

Default is **keep-local** for every row: the readiness work is repository-scoped
documentation and evidence, and any shared-artifact adoption would require its own
approved change.

## BFS Impact Map

| Surface | Impact | Kind |
|---|---|---|
| `README.md` | Add evidence, profile-output, CI, and error-catalog links; keep install/usage accurate | Documentation |
| `docs/profile-output.md` (new) | Interpret `report.json` and the HTML views | Documentation |
| `docs/ci-integration.md` (new) | Headless profiling recipe and artifact retention | Documentation |
| `docs/error-catalog.md` (new) | Observed failure modes with exact output | Documentation |
| `docs/assets/` (new) | TUI + HTML screenshots and `demo.gif` from `--demo` | Evidence asset |
| `Cargo.toml` | Add `license = "MIT"` to match `LICENSE`/README; no dependency change | Metadata |
| `src/**`, `.github/workflows/ci.yml` | Read-only; used to regenerate evidence and cite the existing CI | Verification-only |
| `openspec/` | New `post-mvp-readiness` change, new `readiness` capability, `config.yaml` | Process |
| Sibling repositories | None | None |

## Capabilities

### New Capabilities

- `readiness`: post-MVP evaluability of a cargo plugin — a runnable clean-clone
  quickstart, reproducible visual evidence, a documented report interpretation, a
  headless CI recipe, an error-case catalog, and consistent MIT metadata.

### Modified Capabilities

- None. No existing spec-level behavior changes; this change adds documentation,
  evidence, and one metadata field.

## Non-goals

- No new CLI flags, subcommands, or changes to cargo invocation, TUI rendering,
  or the `report.json` / `report.html` contract.
- No crates.io publication, release tagging, or hosted demo site.
- No shared crate extraction from sibling cargo plugins or adoption of
  `platform-contracts`.
- No relicense; the project remains MIT.
- No mandatory Windows/macOS screenshot parity; platforms that cannot be captured
  are recorded as blocked, not invented.
