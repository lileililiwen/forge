# Proposal: Runnable react-web live preview with native browser verification

## Why

The archived `site-studio-preview-refinement` change ships a bounded preview
lifecycle, but the `react-web` project it previews is a dependency-free
scaffold with no `dev` script, no React runtime, and an empty `vite.config.js`,
so the "live preview" has nothing real to serve. Its own `tasks.md` §5.4–§5.5
pin the closure: a browser smoke and a native `npm install && npm run dev`
check could not run in the build sandbox. That environment now has Node 24,
npm 11, network access and a cached Chromium, so the gap can be closed
honestly.

## What Changes

- Replace the `react-web` scaffold's client with an ordinary, pinned
  Vite + React + React DOM application. `npm run dev` starts the Vite dev
  server, and `vite.config.js` binds the port supplied through
  `FORGE_STUDIO_PORT` with `host: 127.0.0.1` and `strictPort: true` so it
  serves on exactly the Studio-reserved port and never silently shifts.
  `npm run build` and `npm test` stay dependency-free and run without a
  network install (the existing portable-project contract is preserved).
- Terminate the preview runner's entire process tree on stop, startup timeout
  and session drop. The real runner is `npm` → `sh` → Vite → esbuild; killing
  only the direct `npm` child (today's behaviour) leaks the Vite descendant and
  keeps the reserved port bound. Unix process-group termination fixes that.
  Adds a direct `libc` dependency (already transitive; MIT/Apache-2.0).
- Add a repeatable native verification: an env-gated Rust integration test that
  generates the scaffold, installs the pinned toolchain, runs the bounded
  preview to `ready`, checks the server response, and (when enabled) drives a
  committed Playwright smoke that asserts the React greeting is rendered in a
  real browser. Without the Node toolchain or a browser, the check is reported
  `unverified`, never a pass.
- Modify the `site-studio-preview-refinement` "Bounded live preview"
  requirement to require whole-process-tree termination, with process-tree
  scenarios for stop and startup timeout. Record the §5.4/§5.5 closure.

## Package Boundary and Split Assessment

Single outcome: **the Studio live preview runs the generated `react-web`
application to a verified real-browser render and stops without leaking the
reserved port or a descendant process.**

Included surfaces: the `react-web` generator assets (`src/generate/mod.rs`),
the bounded preview runner and its process ownership (`src/studio/preview.rs`),
the `Cargo.toml`/`Cargo.lock` dependency edge for process-group signalling, and
the native/Playwright verification harness.

Excluded surfaces: the same-origin preview proxy, interactive Studio browser
controls, and the `studio.refine` → `agent::apply_transition` runtime hook stay
deferred exactly as `site-studio-preview-refinement` recorded them. No backend,
auth, database or deployment profile work is added.

Split signals considered: the scaffold change and the process-tree fix are each
independently testable, but they share one owner (Forge), one lifecycle (the
preview session), one contract (`FORGE_STUDIO_PORT` + the bounded envelope), and
one acceptance oracle (a browser renders the app and a stop releases the port).
Splitting them yields an intermediate that cannot demonstrate the requested
outcome, so they stay in one package.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `react-web-live-preview` | Preview the generated `react-web` app in a real browser and stop cleanly | Forge, Rust + generated Node/Vite app | `FORGE_STUDIO_PORT` dev contract, bounded preview process tree | `deterministic-project-generation`, `site-studio-preview-refinement` | native `cargo test` integration + Playwright DOM assertion + port-release check |

Dependency order: `deterministic-project-generation` and
`site-studio-preview-refinement` are already archived and promoted; this
package consumes them without redefining their contracts.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Deterministic generation | `src/generate/mod.rs` `template_files` `react-web` arm; `verify_native` | Pinned profile assets, offline build/test, staged FileLevel ownership | Scaffold has no runtime client or dev script | Forge release | **extend shared owner** |
| Bounded preview | `src/studio/preview.rs` `ProcessRunner`, `ChildHandle::shutdown` | Port reservation, bounded wait/logs, redaction, idempotent stop | Stop only signals the direct child, not its tree | Forge release | **extend shared owner** |
| Node/Vite toolchain | generated `package.json`/`vite.config.js` | Standard Vite dev server honoring an env-selected port | None; external, operator-installed | Node/Vite upstream | **adapt through a generic adapter** |
| Playwright | `tests/browser/` (new), cached Chromium | Browser DOM assertions for client-rendered apps | None; optional verification tool | Playwright upstream | **adapt through a generic adapter** |
| Workspace Governance runtime templates | `workspace-governance/templates/runtime` | Portable runtime templates | Preview/dev-server behaviour is not a deployment-template concern | Workspace Governance | **keep local** |

No sibling project exposes a reusable process-supervision or browser-smoke
capability, so the extension stays local to Forge's existing generation and
preview owners.

## BFS Impact Map

- **Actors:** project owner, Forge Core/CLI/API, the generated Node/Vite
  process tree, the optional Playwright browser runner.
- **State:** generation (`react-web` files) → optional `npm install` → preview
  `starting` → `ready` → `stopped`/`failed`; the persisted session vocabulary is
  unchanged.
- **Persistence:** no new file, schema, table or migration. The generated
  `package.json`, `vite.config.js`, `index.html`, `src/*` change; the session
  record and `operations` journal rows are untouched.
- **Security:** the runner argv stays profile-owned and shell-free; the dev
  server binds `127.0.0.1` only; the process tree is signalled with a bounded
  negative-pid group kill over internal pids (no user input, no shell); logs
  stay capped and credential-redacted. `npm install` is an explicit operator
  step, not an implicit generation side effect.
- **Failure:** missing pinned deps → `npm run dev` exits non-zero and the
  Studio reports `failed` (not `ready`); a busy configured port → Vite refuses
  (`strictPort`) instead of shifting; a startup timeout kills the whole tree
  before persisting `failed`.
- **Compatibility:** `npm run build`/`npm test` remain offline and
  dependency-free, so the `deterministic-project-generation` "portable
  projects" requirement is unchanged; the `forge-studio-preview/0.1.0`
  envelope, routes, typed errors and CLI verbs are unchanged.
- **Verification:** existing generator/studio tests stay green; a new gated
  native integration test and a committed Playwright smoke supply the
  previously missing browser and process-tree evidence. `cargo deny check`
  runs against the added `libc` edge when network is available.

## Capabilities

### New Capabilities

- `react-web-live-preview`: a runnable, pinned Vite+React `react-web` scaffold
  whose dev server binds the Studio-reserved port, plus the native
  browser-render verification for that preview.

### Modified Capabilities

- `site-studio-preview-refinement`: the "Bounded live preview" requirement now
  requires whole-process-tree termination on stop and startup timeout.

### Unchanged consumers

- `deterministic-project-generation`: consumed unchanged; its portable
  build/test scenarios still hold because `build`/`test` stay offline.

## Non-goals

- No same-origin preview proxy, no interactive Studio controls, no
  `studio.refine` agent hook (unchanged deferred work).
- No backend, authentication, database, storage or deployment provisioning.
- No change to the preview wire contract, route set, typed errors or CLI verbs.
- No replacement of the deterministic generator, DriftWatch, or the PTY agent
  manager. `npm install` is never run implicitly by `forge new`.

## Explicit deferred work (recorded in tasks.md)

The previously deferred `site-studio-preview-refinement` items remain deferred:
the same-origin preview iframe proxy, the interactive Studio controls, and the
`studio.refine` → `agent::apply_transition` runtime hook.
