current_spec: fleet-live-rollout

# Forge handoff

## Current state

`react-web-live-preview` implemented, verified and archived on 2026-09-30 as
`2026-09-30-react-web-live-preview`. Its requirement was promoted into
[openspec/specs/react-web-live-preview/spec.md](openspec/specs/react-web-live-preview/spec.md),
and the "Bounded live preview" requirement in
[openspec/specs/site-studio-preview-refinement/spec.md](openspec/specs/site-studio-preview-refinement/spec.md)
now terminates the whole preview process tree.

- `src/generate/mod.rs` `react-web` emits a pinned Vite+React client
  (react/react-dom 18.3.1, @vitejs/plugin-react 4.3.4, vite 5.4.11).
  `npm run dev` serves on `FORGE_STUDIO_PORT` (`127.0.0.1`, `strictPort`);
  `npm run build` / `npm test` stay offline and dependency-free.
- `src/studio/preview.rs` spawns the runner in its own Unix process group and
  kills the group on stop, so `npm` → `sh` → Vite → esbuild does not survive;
  runner stderr drains into the bounded log. `libc` added as a dependency.
- Verification adds `tests/react_web_native_preview.rs` (env-gated via
  `FORGE_NATIVE_REACT_WEB_PREVIEW=1`) and the committed Playwright DOM oracle
  `tests/browser/render-check.mjs`; an absent toolchain or browser reports
  `UNVERIFIED`, never a pass.

## Verification (2026-09-30)

- `cargo fmt --all -- --check` (changed files clean; rustfmt 1.9.0 still wants
  pre-existing edits in untouched `src/gate`, `src/github`, `src/portfolio` and
  `src/publish` files), `cargo clippy --all-targets` (no findings in changed
  files), `cargo deny check` (advisories/bans/licenses/sources ok),
  `git diff --check`, and `openspec validate --all --strict --no-interactive`
  (62 passed): clean.
- Tests: `--lib -- generate` incl. `react_web_renders_with_index_and_test`;
  `tests/studio_preview_contract` 7 (incl. the process-tree kill);
  `tests/studio_api_contract` 2; `tests/studio_cli_contract` 13;
  `tests/workspace_metadata_contract` 12 (react-web opt-out digest re-captured
  for the new scaffold).
- Native: `FORGE_NATIVE_REACT_WEB_PREVIEW=1 cargo test --test
  react_web_native_preview` → browser rendered `hello from native-react-preview`
  from `http://127.0.0.1:48200/`. Boundaries: an occupied port is refused
  (`Port ... already in use`, exit 1); a tree without installed dependencies
  fails (`vite: not found`, exit 127).
- `cargo test --workspace --all-targets --no-fail-fast`: 103 targets pass; only
  the pre-existing sandbox failure
  `fleet_online_routes_to_local_listener_when_alethefy_is_up` (DOWN vs ONLINE
  listener restriction) remains.
- Pointer: `react-web-live-preview` archived. The only remaining active change
  is **`fleet-live-rollout`** (10/11, blocked on Mac Docker engine recovery).
  No shared Gate Runtime is configured; no Gate pass is claimed.
