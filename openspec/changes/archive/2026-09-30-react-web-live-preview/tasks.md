# Tasks: react-web-live-preview

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Re-read the `react-web` generator arm (`src/generate/mod.rs`
  `template_files`), the bounded preview (`src/studio/preview.rs`
  `ProcessRunner`, `ChildHandle`, `PreviewSession`), and the promoted
  `site-studio-preview-refinement` spec; map each new requirement and scenario
  to the exact file, symbol and test that will evidence it.
- [x] 1.2 Confirm the implementation-handoff gate: project Forge (Rust 2021,
  MSRV 1.87), generated Node/Vite app, `libc` dependency edge, and the
  `tests/react_web_native_preview.rs` + `tests/browser/` harness paths.
- [x] 1.3 Capture the pre-change baseline: `cargo test --lib -- generate`,
  `cargo test --lib -- studio`, `tests/studio_{preview,api,cli}_contract`, and
  record the current `npm run build`/`npm test` offline behaviour of the
  existing scaffold.
- [x] 1.4 Add failing/skeleton coverage first: the generator asset assertions
  for the new files, the process-tree kill assertion in
  `tests/studio_preview_contract.rs`, and the gated
  `tests/react_web_native_preview.rs` skeleton.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement the runnable scaffold in the `react-web` generator arm:
  pinned `package.json` (`dev`/`build`/`test`, react + vite deps),
  `FORGE_STUDIO_PORT`-aware `vite.config.js`, `index.html` loading
  `/src/main.jsx`, `src/main.jsx` React mount with the `data-testid="forge-app"`
  marker, `src/greeting.mjs`, the updated `src/app.test.mjs`, and the README
  (`npm install` then `npm run dev`; build/test offline).
- [x] 2.2 Implement process-tree termination in `src/studio/preview.rs`:
  `ChildHandle.process_group`, `CommandExt::process_group(0)` in both runners,
  `libc::kill(-pid, SIGKILL)` in `shutdown` behind `#[cfg(unix)]`, stderr
  drained into the bounded log buffer, and the `libc` dependency edge in
  `Cargo.toml`/`Cargo.lock`.
- [x] 2.3 Inherit `HOME` in `ProcessRunner::react_web` alongside `PATH`,
  `LC_ALL` and `FORGE_STUDIO_PORT`; keep `env_clear()` and the shell-free argv.
- [x] 2.4 Implement `tests/react_web_native_preview.rs`: generate → install →
  start preview → `ready` → HTTP 200 + mount-point assertion → stop →
  port-rebindable + no-descendant assertion, with `UNVERIFIED` short-circuits
  when `npm` is absent.
- [x] 2.5 Implement the committed Playwright smoke under `tests/browser/`
  (`package.json`, `render-check.mjs`, `README.md`) asserting the rendered
  greeting, and invoke it from the Rust test with the live preview URL.
- [x] 2.6 Update the generator unit tests (`src/generate/mod.rs`) for the new
  asset set and confirm `verify_native("react-web", …)` still does not need a
  network install.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run the generator, studio and cross-surface suites; confirm the
  preview envelope, routes, CLI verbs, typed errors and the `operations`
  journal are unchanged.
- [x] 3.2 Prove the process boundary on the native path: after a real preview
  stop no descendant of the runner remains and the reserved port is immediately
  rebindable; prove the same for the startup-timeout path.
- [x] 3.3 Confirm `npm run build`/`npm test` still pass offline (no
  `node_modules`) so `deterministic-project-generation` is unchanged, and that
  a missing-dependency `npm run dev` yields `failed`, not `ready`.
- [x] 3.4 Confirm no unrelated project files, routes, manifests or registry
  rows are touched, and that `npm install` never runs implicitly during
  generation.

## 4. Verification

- [x] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`,
  `git diff --check`, and the focused suites; record exact results.
- [x] 4.2 Run the gated native path (`FORGE_NATIVE_REACT_WEB_PREVIEW=1 cargo
  test --test react_web_native_preview -- --nocapture`) and the Playwright
  browser smoke; record the rendered-DOM and port-release evidence, and record
  `cargo deny check` with its network status.
- [x] 4.3 Run `node scripts/check-openspec-change-names.mjs` and
  `openspec validate --all --strict --no-interactive`.
- [x] 4.4 Record the deferred items (same-origin proxy, interactive controls,
  `studio.refine` agent hook) as still open, archive the verified change
  without `--skip-specs`, and update HANDOFF with the next eligible change.
