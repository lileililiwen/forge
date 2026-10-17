# Tasks: Post-MVP readiness for cargo-sonar-boost

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Confirm the shipped CLI surface from `src/main.rs` (`--demo`, `--no-open`, `--output-dir`, `--project-dir`) and record the exact demo invocation and output paths; freeze v0.1.1 as the documented version.
- [ ] 1.2 Inventory existing readiness surfaces (`README.md`, `Cargo.toml` MIT metadata, `LICENSE`, `.github/workflows/ci.yml`, `docs/publishing_guide.md`) and mark which already satisfy readiness.
- [ ] 1.3 Record the exact tool probes and skip messages from `src/analyzer.rs` as the source of truth for `docs/tool-availability.md`.
- [ ] 1.4 Record the failure modes in `src/main.rs` and `src/analyzer.rs` (missing `Cargo.toml`, clippy failure, optional tool missing, browser-open failure, export failure) as the source of truth for `docs/error-catalog.md`.
- [ ] 1.5 Confirm no source file, sibling repository (`cargo-audit-boost`, `cargo-matrix-boost`, `rust-platform-libs`, `workspace-governance`, `driftwatchdog`), or shared artifact requires a change; if one does, stop and split it out.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Generate the HTML dashboard screenshot(s) from `cargo sonar-boost --demo --no-open --output-dir target/sonar-demo` and store them under `docs/assets/` with the producing command.
- [ ] 2.2 Record a short `demo.gif` walkthrough with an available recorder; if none exists, record the exact blocker instead of fabricating media.
- [ ] 2.3 Author `docs/tool-availability.md` covering required `cargo clippy` and optional `cargo-deny`, `cargo-geiger`, `cargo-tarpaulin`, the probe each uses, the exact skip message, and the dashboard dimension left empty when absent.
- [ ] 2.4 Author `docs/ci-integration.md` with a copy-paste headless recipe built only on existing flags (`--no-open --output-dir`) and artifact retention of `report.json` / `report.html`.
- [ ] 2.5 Author `docs/error-catalog.md` listing observed failure modes, exact output, and next action for each.
- [ ] 2.6 Update `README.md` to link the evidence, tool matrix, CI recipe, and error catalog without changing its install/usage accuracy.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Confirm every documented flag and behavior matches `src/main.rs` and shipped v0.1.1; no unshipped capability is advertised.
- [ ] 3.2 Confirm screenshots match the fields and grades in a freshly generated `report.json`.
- [ ] 3.3 Confirm the tool-availability matrix matches `src/analyzer.rs` probes and skip messages exactly.
- [ ] 3.4 Confirm all new README links resolve and that `.github/workflows/ci.yml` still reflects the documented CI recipe.
- [ ] 3.5 Confirm no `src/**` file, sibling repository, or shared artifact was modified beyond this change package, documentation, and assets.

## 4. Verification

- [ ] 4.1 Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --verbose`.
- [ ] 4.2 Run `cargo install --path .` and `cargo sonar-boost --demo --no-open --output-dir target/sonar-demo`; assert `report.json` and `report.html` are non-empty.
- [ ] 4.3 Run `openspec validate post-mvp-readiness --strict --no-interactive` and `git diff --check`.
- [ ] 4.4 Record PASS, FAIL, or BLOCKED per command with exact output; do not archive while a mandatory check is failing or a blocker is unresolved.
- [ ] 4.5 Record any unavailable GUI/browser/recorder/platform evidence as BLOCKED with the exact command and output; never fabricate screenshots or claims.
