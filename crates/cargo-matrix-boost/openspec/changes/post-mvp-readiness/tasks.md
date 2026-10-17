# Tasks: Post-MVP readiness for cargo-matrix-boost

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Confirm the shipped CLI surface from `src/main.rs` and record the exact `--demo` invocation and output paths; freeze v0.1.0 as the documented version.
- [ ] 1.2 Inventory existing documentation (`README.md`, `Cargo.toml` metadata, `LICENSE`, `.github/workflows/ci.yml`) and mark which already satisfy readiness.
- [ ] 1.3 Map every surface in the BFS Impact Map and confirm none requires a runtime code change; if one does, stop and split it into a separate change.
- [ ] 1.4 Confirm repository-local boundary: no sibling `cargo-audit-boost`, `cargo-sonar`, `rust-platform-libs`, or `platform-contracts` artifact is modified.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Generate the TUI screenshot(s) and HTML dashboard screenshot(s) from `cargo matrix-boost --demo` and store them under the documented asset directory with the producing command.
- [ ] 2.2 Record a short `demo.gif` walkthrough using an available TTY recorder; if no recorder exists, record the exact blocker instead of fabricating media.
- [ ] 2.3 Author `docs/toolchain-matrix.md` covering stable/beta/nightly, feature powerset vs explicit sets, defaults, exclusions, `--max-combinations`, and how to read the result grid.
- [ ] 2.4 Author `docs/ci-integration.md` with a copy-paste headless recipe built only on existing flags (`--no-open --output-dir`) and artifact retention of `report.json`/`report.html`.
- [ ] 2.5 Author `docs/error-catalog.md` listing observed failure modes, exact output, and next action for each.
- [ ] 2.6 Update `README.md` to link the evidence, matrix, CI recipe, and error catalog without changing its install/usage accuracy.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Confirm every documented flag and behavior matches `src/main.rs` and the shipped v0.1.0 behavior; no unshipped capability is advertised.
- [ ] 3.2 Confirm screenshots and the GIF reflect the same fields and statuses as a freshly generated `report.json`.
- [ ] 3.3 Confirm all new README links resolve and that `.github/workflows/ci.yml` still reflects the documented CI recipe.
- [ ] 3.4 Confirm no source file, sibling repository, or shared artifact was modified beyond this change package and documentation/assets.

## 4. Verification

- [ ] 4.1 Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
- [ ] 4.2 Run `cargo install --path .` and `cargo matrix-boost --demo --no-open --output-dir target/matrix-demo`; assert `report.json` and `report.html` are non-empty.
- [ ] 4.3 Run `openspec validate post-mvp-readiness --strict --no-interactive` and `git diff --check`.
- [ ] 4.4 Record PASS, FAIL, or BLOCKED per command with exact output; do not archive while a mandatory check is failing or a blocker is unresolved.
- [ ] 4.5 Record any unavailable GUI/recorder/platform evidence as BLOCKED with the exact command and output; never fabricate screenshots or claims.