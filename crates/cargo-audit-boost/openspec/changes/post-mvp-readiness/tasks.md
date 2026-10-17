# Tasks: Post-MVP readiness for cargo-audit-boost

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Confirm the shipped CLI surface from `src/main.rs` (trailing cargo args, `--demo`) and record the exact `--demo` invocation and output paths (`target/build-scan/report.json` / `report.html`); freeze v0.1.0 as the documented version.
- [ ] 1.2 Inventory existing readiness surfaces (`README.md`, `LICENSE`, `.github/workflows/ci.yml`, `.project.json`) and mark which already satisfy readiness and which are missing.
- [ ] 1.3 Record the `ExportReport` field set from `src/exporter.rs` as the source of truth for `docs/profile-output.md`.
- [ ] 1.4 Confirm the non-TTY behavior in `src/main.rs` and identify a PTY recorder for TUI/GIF capture; if none exists, record the blocker now.
- [ ] 1.5 Confirm no source file, sibling repository (`cargo-matrix-boost`, `cargo-sonar`, `rust-platform-libs`, `workspace-governance`, `driftwatchdog`), or shared artifact requires a change; if one does, stop and split it out.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Generate the TUI screenshot(s) and HTML report screenshot(s) from `cargo-audit-boost --demo` and store them under `docs/assets/` with the producing command.
- [ ] 2.2 Record a short `demo.gif` walkthrough with an available PTY recorder; if none exists, record the exact blocker instead of fabricating media.
- [ ] 2.3 Author `docs/profile-output.md` describing `total_duration_secs`, `total_crates`, `max_concurrency`, `success`, `bottlenecks`, and `crates[]`, and how to read the Gantt, cached, bottleneck, and concurrency views.
- [ ] 2.4 Author `docs/ci-integration.md` with a headless profiling recipe and artifact retention of `target/build-scan/report.json` / `report.html`; note the PTY requirement if the TUI needs one.
- [ ] 2.5 Author `docs/error-catalog.md` listing observed failure modes (no Cargo project, invalid cargo arguments, build failure, non-TTY TUI, unwritable target directory), each with exact output and next action.
- [ ] 2.6 Add `license = "MIT"` to `Cargo.toml` so README, `LICENSE`, and package metadata agree.
- [ ] 2.7 Update `README.md` to link the evidence, profile-output doc, CI recipe, and error catalog without changing install/usage accuracy.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Confirm every documented flag and behavior matches `src/main.rs` and shipped v0.1.0; no unshipped capability is advertised.
- [ ] 3.2 Confirm screenshots and the GIF reflect the same fields and statuses as a freshly generated `report.json`.
- [ ] 3.3 Confirm `docs/profile-output.md` matches the current `ExportReport` fields and that all new README links resolve.
- [ ] 3.4 Confirm `.github/workflows/ci.yml` still reflects the documented CI recipe and that `Cargo.toml` differs only by the `license` field.
- [ ] 3.5 Confirm no `src/**` file, sibling repository, or shared artifact was modified beyond this change package, docs, assets, and the metadata line.

## 4. Verification

- [ ] 4.1 Run `cargo check --verbose` and `cargo test --workspace`.
- [ ] 4.2 Run `cargo install --path .` and `cargo-audit-boost --demo`; assert `target/build-scan/report.json` and `report.html` are non-empty.
- [ ] 4.3 Run `openspec validate post-mvp-readiness --strict --no-interactive` and `git diff --check`.
- [ ] 4.4 Record PASS, FAIL, or BLOCKED per command with exact output; do not archive while a mandatory check is failing or a blocker is unresolved.
- [ ] 4.5 Record any unavailable GUI/PTY-recorder/platform evidence as BLOCKED with the exact command and output; never fabricate screenshots or claims.
