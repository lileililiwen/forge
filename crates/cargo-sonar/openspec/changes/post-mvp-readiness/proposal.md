# Proposal: Post-MVP readiness — dashboard evidence, tool-availability matrix, CI recipe, error catalog

## Why

`cargo-sonar-boost` is a usable zero-config Rust quality dashboard: it installs
with `cargo install --path .`, aggregates `cargo clippy`, `cargo-deny`,
`cargo-geiger`, and `cargo-tarpaulin` into a self-contained interactive HTML
dashboard, offers a `--demo` simulation mode, and is MIT-licensed with complete
crate metadata. What it lacks is *evaluability*: a stranger cannot see the HTML
dashboard before installing, cannot tell which of the four underlying tools are
required versus optional and what a skip looks like, has no copy-paste CI recipe
for headless runs, and has no catalog of what failure looks like. Post-MVP
readiness means the repository alone is enough to evaluate, trust, and integrate
the tool.

## What Changes

- Add reproducible visual evidence: HTML dashboard screenshot(s) (the health
  gauge, clippy browser, coverage view, and light/dark) and a short `--demo`
  walkthrough GIF, generated from the repository's own `--demo` mode and stored
  with the command that produced them.
- Add `docs/tool-availability.md`: a matrix of required (`cargo clippy`) versus
  optional (`cargo-deny`, `cargo-geiger`, `cargo-tarpaulin`) tools, the probe
  each uses, the exact skip message, and what each dimension is left showing when
  a tool is absent.
- Add `docs/ci-integration.md` with a headless recipe using
  `--no-open --output-dir` and artifact retention of `report.json` /
  `report.html`.
- Add `docs/error-catalog.md` of observed failure modes (no `Cargo.toml`, clippy
  failure, each optional tool missing, browser-open failure, unwritable output
  directory) with exact output and next action.
- Cross-link the above from `README.md` without changing CLI behavior, report
  schema, or licensing.

This is a documentation-and-evidence change. No runtime behavior is modified.

## Package Boundary and Split Assessment

This change is **one package** (`post-mvp-readiness`), not a split.

- All foci (screenshots/GIF, tool-availability matrix, CI recipe, error catalog)
  are repository-local documentation and evidence sharing a single entry point
  (`README.md`) and a single evidence pipeline (`cargo sonar-boost --demo` →
  `target/sonar/report.json` + `report.html`).
- None changes runtime behavior, so none has an independent verification surface
  that would justify a separate change.
- Candidate split considered and rejected: extracting a shared "cargo plugin
  docs/evidence" convention with `cargo-audit-boost` / `cargo-matrix-boost`. That
  is a cross-repository architecture decision and is a Non-goal here.
- Candidate future split recorded: if the error catalog requires *new exit
  codes*, a `--json` error output, or a machine-readable tool-availability
  manifest, that is a behavior change and MUST move to a separate feature change.
  Tracked in `design.md` → Ledger.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository | `src/main.rs`, `src/analyzer.rs`, `src/demo.rs`, `src/exporter.rs`, `README.md`, `Cargo.toml` (MIT metadata) | `--demo`, `--no-open`, `--output-dir`, four-tool aggregation, HTML dashboard | no screenshots, no tool matrix, no CI recipe, no error catalog | cargo-sonar-boost | **extend shared owner** |
| cargo-audit-boost | `src/exporter.rs`, `README.md` | sibling cargo plugin with a `--demo` HTML report | different tool and report model; no shared code | cargo-audit-boost | **keep local** |
| cargo-matrix-boost | `openspec/changes/post-mvp-readiness/` | sibling post-MVP-readiness package (structure precedent only) | different CLI, output dir, and report schema | cargo-matrix-boost | **keep local** — precedent, not a dependency |
| rust-platform-libs | workspace crates | platform libraries | no compile-time dependency; adding one is out of scope | rust-platform-libs | **keep local** |
| workspace-governance | `projects.json`, `templates/project/**` | registry, shared rules templates | conventions are copied per repository; governance does not own plugin docs | workspace-governance | **keep local** |
| driftwatchdog | gate runtime | CI gate for downstream products | not consumed by this repo's `cargo test` verification; no readiness contract | driftwatchdog | **keep local** |
| platform-contracts (`platform.readiness`) | `schemas/readiness.schema.json` | service `/healthz` + `/readyz` envelope | this is a one-shot CLI, not a long-running service; exposes no health endpoint | platform-contracts | **keep local** |
| `docs/publishing_guide.md` | existing crates.io publishing guide | release procedure | unrelated to evaluability; unchanged | this repository | **adopt** — referenced, not duplicated |

Default is **keep-local** for every row: the readiness work is repository-scoped
documentation and evidence, and any shared-artifact adoption would require its
own approved change.

## BFS Impact Map

| Surface | Impact | Kind |
|---|---|---|
| `README.md` | Add evidence, tool-matrix, CI, and error-catalog links; keep install/usage accurate | Documentation |
| `docs/tool-availability.md` (new) | Required vs optional tools, probes, skip messages, dimension impact | Documentation |
| `docs/ci-integration.md` (new) | Headless `--no-open --output-dir` recipe and artifact retention | Documentation |
| `docs/error-catalog.md` (new) | Observed failure modes with exact output | Documentation |
| `docs/assets/` (new) | HTML dashboard screenshots and `demo.gif` from `--demo` | Evidence asset |
| `src/**`, `.github/workflows/ci.yml`, `docs/publishing_guide.md` | Read-only; used to regenerate evidence and cite existing behavior | Verification-only |
| `Cargo.toml` | Unchanged (already `license = "MIT"`, readme, repository); verified for consistency | Verification-only |
| `openspec/` | New `post-mvp-readiness` change, new `readiness` capability, `config.yaml` | Process |
| Sibling repositories | None | None |

## Capabilities

### New Capabilities

- `readiness`: post-MVP evaluability of a cargo plugin — a runnable clean-clone
  quickstart, reproducible visual evidence, a tool-availability matrix, a
  headless CI recipe, an error-case catalog, and consistent MIT metadata.

### Modified Capabilities

- None. No existing spec-level behavior changes; this change adds documentation
  and evidence only.

## Non-goals

- No new CLI flags, subcommands, or changes to tool invocation, health scoring,
  or the `report.json` / `report.html` contract.
- No crates.io publication, release tagging, or hosted demo site.
- No new scanner or replacement of `clippy`/`deny`/`geiger`/`tarpaulin`.
- No shared crate extraction from sibling cargo plugins or adoption of
  `platform-contracts`.
- No relicense; the project remains MIT.
- No mandatory Windows/macOS screenshot parity; platforms that cannot be captured
  are recorded as blocked, not invented.
