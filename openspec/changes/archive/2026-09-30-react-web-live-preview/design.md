# Design: Runnable react-web live preview with native browser verification

## 1. Implementation boundary

Repository `/home/paul/code/forge`, Rust 2021, MSRV 1.87.

- `src/generate/mod.rs`: replace the `react-web` arm of `template_files`
  (~line 496). New/changed assets: `package.json`, `vite.config.js`,
  `index.html`, `src/main.jsx` (new), `src/greeting.mjs` (new),
  `src/app.test.mjs`, `README.md`. Keep `scripts/build.mjs`, `Dockerfile`,
  `.gitignore` (`node_modules/`, `dist/`).
- `src/studio/preview.rs`: add process-tree ownership. `ChildHandle` gains
  `process_group: bool` and `stderr_thread` fields; `shutdown` signals the
  group before reaping and joins both capture threads. `ProcessRunner::spawn`
  and `FakeRunner::spawn` put the child in its own process group (Unix) and
  drain stderr into the same bounded log buffer so a verbose toolchain cannot
  block the pipe. `ProcessRunner::react_web` additionally inherits `HOME`.
- `Cargo.toml` + `Cargo.lock`: add `libc` (workspace dependency) used only for
  the negative-pid group kill.
- New `tests/react_web_native_preview.rs`: env-gated native integration test.
- New `tests/browser/` Playwright harness (`package.json`, `render-check.mjs`,
  `README.md`).
- Optionally extend `tests/studio_preview_contract.rs` with a process-tree
  kill assertion using a two-level fake runner.

Not changed: `src/studio/{spec,state}.rs`, the `forge-studio-preview/0.1.0`
envelope, Studio routes/CLI verbs, the `operations` schema, `deny.toml`.

## 2. Language and runtime

- Forge: Rust 2021, `cargo build`/`cargo test`. Commands:
  `cargo fmt --all -- --check`, `cargo clippy --all-targets`,
  `cargo test --workspace --all-targets --no-fail-fast`.
- Generated app: Node 20+ (verified on Node 24.18.0) with npm 11. Pinned
  `react@18.3.1`, `react-dom@18.3.1`, `@vitejs/plugin-react@4.3.4`,
  `vite@5.4.11` (all verified to install and bind the reserved port on Node
  24). `npm run dev` = `vite`; `npm run build` = `node ./scripts/build.mjs`;
  `npm test` = `node --test`.
- Browser verification: Playwright 1.63 with cached Chromium under
  `tests/browser/`.

## 3. Ownership and shared code

- The `react-web` assets stay owned by Forge's deterministic generator. The
  generator remains the single place the scaffold shape lives; no second
  template tree is introduced.
- The bounded preview stays owned by `src/studio/preview.rs`. Process
  supervision is added there rather than in `src/process.rs`, because only the
  preview owns a long-lived child that spawns grandchildren; the short-lived
  `spawn_with_timeout` helper is unchanged.
- Playwright is an optional verification tool owned by the test harness, not a
  Forge runtime dependency. It is never linked into the `forge` binary.

## 4. Behavioral model

### Scaffold dev contract

| Property | Value |
|---|---|
| Dev command | `npm run dev` → `vite` |
| Port source | `FORGE_STUDIO_PORT` env, parsed as an integer |
| Bind host | `127.0.0.1` |
| Port fallback | Vite default `5173` when the env is unset or non-numeric |
| Port strictness | `strictPort: true` — never increments to another port |
| Build/test offline | `node ./scripts/build.mjs`, `node --test` (no deps required) |
| Mount marker | `<main data-testid="forge-app">hello from &lt;id&gt;</main>` |

`vite.config.js`:

```js
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const reserved = Number.parseInt(process.env.FORGE_STUDIO_PORT ?? "", 10);

export default defineConfig({
  plugins: [react()],
  server: {
    host: "127.0.0.1",
    port: Number.isNaN(reserved) ? 5173 : reserved,
    strictPort: true,
  },
});
```

### Preview process tree

```
stop()/timeout/drop
  -> ChildHandle::shutdown
       if process_group: libc::kill(-(child.id() as i32), SIGKILL)   // npm + sh + vite + esbuild
       child.kill()                                                   // fallback / direct child
       child.wait()
       join stdout capture thread
       set accept_stop; join accept thread (FakeRunner only)
```

The child is spawned with `CommandExt::process_group(0)`, so its process-group
id equals its pid; a negative-pid `kill` therefore targets exactly the tree the
runner created. `ChildHandle::shutdown` still calls `child.kill()` afterwards so
a platform without process groups (or a runner that did not request one) keeps
the previous behaviour. Both stdout and stderr are drained into the bounded,
redacted log buffer; stderr is prefixed with `[stderr]` so a failed start is
visible in the persisted log tail rather than blocking on an unread pipe.

### Runner environment

`ProcessRunner::react_web` keeps `env_clear()` and re-adds only:
`LC_ALL=C`, the parent `PATH` (toolchain resolution), the parent `HOME` (npm
resolves its cache and user config beneath `$HOME`), and `FORGE_STUDIO_PORT`
(the reserved port). No other parent variable is inherited, and the argv is
never interpolated through a shell.

## 5. Contract and compatibility

- Wire contract unchanged: `forge-studio-preview/0.1.0`, the same
  `PreviewEnvelope`, states, typed errors (`studio-start-timeout`,
  `studio-port-unavailable`, …), routes and CLI verbs.
- Generated-file contract: new files (`src/main.jsx`, `src/greeting.mjs`) and
  changed files (`package.json`, `vite.config.js`, `index.html`,
  `src/app.test.mjs`, `README.md`). Determinism is preserved: every new asset
  is a pure function of the project id.
- Offline compatibility: `npm run build` and `npm test` remain dependency-free,
  so `verify_native("react-web", …)` and the `deterministic-project-generation`
  "portable projects" scenarios keep passing without network.
- Dependency change: `libc` is added as a direct dependency. It is already in
  the resolved graph (transitive) and is MIT/Apache-2.0, within the
  `deny.toml` allow-list; `[bans] wildcards = "deny"` is not triggered by a
  `"0.2"` requirement.

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| `FORGE_STUDIO_PORT` unset/non-numeric | Dev server uses `5173`; Studio always sets the var, so this only affects manual runs |
| `FORGE_STUDIO_PORT` occupied | Vite exits non-zero (`strictPort`); Studio's bounded wait records `studio-start-timeout`/`failed`; no port shifted |
| Pinned deps missing | `npm run dev` exits non-zero before binding; Studio records `failed`, never `ready` |
| Startup timeout | Whole process tree killed before `failed` is persisted |
| Explicit stop / drop | Whole process tree killed; port freed; one `studio.preview.stop` row |
| Port allocator exhausted | Unchanged: typed `studio-port-unavailable`, unrelated listeners untouched |
| Non-Unix platform | Falls back to `child.kill()` (direct child only); the native verification is Linux-only and reports `unverified` elsewhere |

## 7. Verification oracle

- Unit/contract (always run):
  - `cargo test --lib -- generate` — generator asset set, offline
    `npm run build`/`npm test` via `verify_native`, manifest/README assertions.
  - `cargo test --lib -- studio` and `tests/studio_preview_contract.rs`,
    `tests/studio_api_contract.rs`, `tests/studio_cli_contract.rs` — bounded
    lifecycle, idempotent stop, typed refusals, transport parity.
  - New process-tree assertion in `tests/studio_preview_contract.rs`: a fake
    runner that spawns a grandchild which holds the reserved port; after `stop`
    the port must be bindable and the grandchild gone.
- Native (opt-in, host-dependent):
  - `FORGE_NATIVE_REACT_WEB_PREVIEW=1 cargo test --test react_web_native_preview -- --nocapture`
    generates the scaffold, runs `npm install --no-audit --no-fund`, starts the
    preview through `studio::start_preview(..., ProcessRunner::react_web())`,
    asserts `ready`, asserts an HTTP 200 with the `<div id="root">` mount point,
    stops, and asserts the reserved port is immediately rebindable and no
    descendant remains. Missing `npm` ⇒ prints `UNVERIFIED` and does not fail.
  - The same run drives the committed Playwright smoke
    (`tests/browser/render-check.mjs`) against the live preview URL; exit `2`
    (Playwright or browser unavailable) is reported `UNVERIFIED`, never a pass.
  - Standalone: `cd tests/browser && npm install && node render-check.mjs "<preview-url>" "<greeting>"`.
- Strict artifacts: `node scripts/check-openspec-change-names.mjs`,
  `openspec validate --all --strict --no-interactive`, `git diff --check`.
- `cargo deny check` is run when the advisory database is reachable; if not, it
  is recorded as network-blocked, not as a pass.

## 8. Decision ledger

- **Runnable client, offline build/test.** The scaffold gains a real React
  client and Vite dev server, but `build`/`test` keep the dependency-free node
  scripts so the existing portable-project contract and `verify_native` are
  untouched. Alternative rejected: making `build` = `vite build`, which would
  break the offline scenario and require an implicit install.
- **Pinned old-but-stable versions.** react 18.3.1 / vite 5.4.11 were verified
  end-to-end on Node 24; newer majors add churn without changing the outcome.
- **Process-group kill with `libc`.** Chosen over (a) hardcoding
  `node_modules/.bin/vite`, which couples the runner to Vite's layout, and
  (b) `/proc` descendant walking, which is racier and more code. `libc` is
  already transitive and license-allowed.
- **`HOME` inheritance.** `env_clear()` previously dropped `HOME`; with a real
  npm run that can break cache/user-config resolution, so `HOME` is inherited
  explicitly (not a secret, not arbitrary env passthrough).
- **Optional browser tool.** Playwright lives in `tests/browser/` behind an env
  gate so the default Rust test suite stays offline and no Forge runtime
  dependency is added.
- **Deferred, unchanged:** same-origin preview proxy, interactive Studio
  controls, `studio.refine` agent hook.
