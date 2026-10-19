# Tasks: Post-MVP readiness (Rust platform crates)

## 1. BFS — Baseline and impact coverage

- [x] Confirm the boundary: no `Cargo.toml`/`Cargo.lock`/`src/` edit; workspace config and the MSRV/lint/CI baseline are owned by the active `rust-platform-consumer-baseline` change — do not race it.
- [x] Record the README baseline (61 lines) and inventory all six crates from `Cargo.toml` and `crates/*/Cargo.toml` (version, license, publish, rust-version, edition).
- [x] Confirm `[workspace.package].license = "Apache-2.0"` with no root LICENSE file on disk.
- [x] Map each crate's public surface to its owning spec (`rust-platform-foundation`, `rust-identity-audit-permission`) and confirm no API change is needed.
- [x] Record which crates are production vs test-only; test-only crates are excluded from examples.
- [x] Confirm no forbidden dependency (`axum`, `sqlx`, `cookie`, `jsonwebtoken`, `openidconnect`, `oauth2`, `reqwest`, `tokio`, product crates) enters a production crate.

## 2. DFS — Requirement-by-requirement implementation

- [x] REQ-1: add the per-crate matrix to `README.md` (purpose, production/test-only, version, license, publish, MSRV, edition).
- [x] REQ-2: write `docs/usage.md` with one minimal usage example per production crate.
- [x] REQ-3: add compile-checked `crates/platform-{core,contracts,identity,audit}/examples/*.rs` matching the usage snippets.
- [x] REQ-4: add the root Apache-2.0 `LICENSE` and document per-crate inheritance in the matrix.
- [x] REQ-5: add the consumer adoption guide section documenting the path-dependency route and linking `rust-platform-consumer-baseline` for the shared recipe.
- [x] REQ-6: document versioning (`0.1.0`, workspace-inherited) and the `publish = false` posture without asserting a registry decision.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-check both archived specs: cited, not edited (`git status` shows no `Cargo.toml`/`src`/spec edits).
- [x] Confirm `cargo test --workspace` compiles and passes all examples and the existing suite.
- [x] Confirm `platform-arch-tests` still passes (no new dependency direction).
- [x] Confirm every README matrix value matches the manifests; refresh any stale counts.
- [x] Confirm `docs/usage.md` snippets are the same code as the compiling examples.
- [x] Confirm the change does not duplicate the baseline change's recipe or archive order.

## 4. Verification

- [x] `cargo test --workspace`
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `node scripts/check-openspec-change-names.mjs`
- [x] `openspec validate --changes --strict --no-interactive`
- [x] `openspec validate --specs --strict --no-interactive`
- [x] `git diff --check`
- [x] Record outputs in the change notes; do not commit and do not archive.
