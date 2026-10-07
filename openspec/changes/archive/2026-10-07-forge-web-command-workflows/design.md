# Design: Task-first dashboard with one-confirm bulk onboarding

## 1. Implementation boundary

- **Repository / project:** this Forge repository, `frontend/` only. No
  Rust, catalog, Core or spec-surface change.
- **Modules changed:**
  - `frontend/index.html` — move the `#commands` section after `#delivery`;
    reorder sidebar nav to fleet → workbench → management → portfolio →
    delivery → commands; relabel the commands heading copy as reference.
  - `frontend/app.js` — auto-run `wsDiscover()` at the end of a successful
    dashboard load; render the unonboarded count line under the fleet from
    the discovery payload; chunk `wsPreview`/`wsRun` at 25 items with one
    combined plan view, per-chunk digests and sequential apply.
  - `tests/forge_portal_frontend_contract.rs` — pin section order, the
    count-line element, auto-discover and chunking markers.
  - `tests/browser/workspace-onboarding-check.mjs` +
    `tests/forge_web_workspace_onboarding_browser.rs` — 27-dir fixture
    (25 + 2 across two chunks), no manual Discover click, chunked confirm,
    fleet-appears, order/keyboard/contrast/path assertions.
- **Must NOT change:** any API route, catalog row, Core function, journal
  shape, heading hierarchy (`h1`→`h2`→`h3` order preserved by moving whole
  sections), or the single-item management controls.

## 2. Language and runtime

- Browser HTML/JavaScript served by `forge web serve`; no bundler, no new
  dependency. Rust 2021 toolchain only to run the existing test harness.
- Verify with the focused contract/browser suites plus `portal_ui_contract`
  and the pinned Playwright a11y page check if touched (it is not — the
  legacy `/ui` pages are untouched).

## 3. Ownership and shared code

- `frontend/` owns presentation and flow orchestration; the API contract
  (`candidates`/`onboard` shapes, 25-item cap, digest discipline) is
  consumed unchanged.
- Chunking lives entirely in the panel: each chunk is a complete,
  independent preview → digest → confirm cycle against the shipped route,
  so the server-side safety properties are inherited, not reimplemented.

## 4. User experience and interface

Actor: authenticated global admin. Entry: dashboard load. The first screen
shows fleet totals, readiness and the unonboarded-count line; scrolling
reaches workbench, then management with the onboarding table already
populated. The count line (“N workspace directories are not yet
onboarded”) links to `#management`.

States: loading (“Reading the workspace…”); empty (no directories);
unconfigured root (prerequisite notice + README step, manual Discover kept);
discovery error (manual retry); preview (combined per-chunk plans with one
digest line per chunk); running (per-chunk progress in the result box);
partial (completed chunks kept, failed chunk reported, re-preview prompt);
success (totals + Reload fleet). All text via `textContent`; existing
live regions, labelled controls, table keyboard region and focus styles
reused. Narrow viewports reuse the existing scroll regions; no new CSS
beyond what exists (verify: reuse `.notice`, `.wb-plan-*`, table styles).

## 5. Behavioral model

- On dashboard load (authenticated): fleet + commands load as today, then
  `wsDiscover()` runs automatically. Failure leaves the panel exactly as
  today (manual Discover + empty table).
- Unonboarded count = candidates with `selectable: true` in the discovery
  payload; rendered as “N of M workspace directories are not yet
  onboarded.” plus an anchor to `#management`. Hidden when root
  unconfigured.
- Preview: split selection into 25-item chunks preserving table order;
  POST each chunk (no confirm); concatenate `preview` arrays in order;
  store `[{items, digest}]`. Render every plan line plus one digest line
  per chunk. Any chunk refusal renders its error and aborts (no digest
  stored).
- Run: requires the confirmation tick; applies chunks in order with each
  chunk's own `{confirm: true, plan_digest}`; appends each chunk's result
  lines as they land; a refused chunk stops the loop, keeps prior results
  visible, clears stored digests and prompts re-preview. Success shows
  grand totals + Reload fleet button.
- Reload: `window.location.reload()` (unchanged).

## 6. Contract and compatibility

- No wire change. Chunk size constant `WS_CHUNK = 25` mirrors the server
  cap; if the server ever rejects a chunk, its typed error renders as
  today.
- DOM ids added: one count-line element under the fleet heading. No id
  removed or renamed. Heading levels unchanged.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| Auto-discovery fails | manual Discover path unchanged; error notice with retry |
| Root unconfigured | prerequisite notice, no count line |
| Empty selection preview | refused client-side before any request |
| Chunk preview refused | error rendered, no digest stored, nothing written |
| Chunk apply refused mid-run | completed chunks kept visible; digests cleared; re-preview prompt |
| Digest mismatch on a chunk | server 409 with fresh preview (existing behavior per chunk) |
| Reload | existing full-page reload |

## 8. Verification oracle

- **Extended browser drive**: 27-dir fixture (25 importable cargo dirs +
  2 manifest dirs); assert `#commands` comes after `#management` in DOM
  order; do NOT click Discover (auto-run); select all; preview shows 27
  plan lines + 2 digest lines; confirm + run applies chunk 1 then chunk 2;
  results total 27; reload; fleet lists a sample of new ids; keyboard
  entry, contrast sample, no-absolute-path assertions retained.
- **Frontend source contract**: section order (`#management` before
  `#commands`), count-line id, auto-discover call, chunk-size marker.
- **Existing suites unchanged and green**: portal UI, catalog, management,
  workbench, delivery browser (order-agnostic), a11y page harness.
- A task box is checked only with the command output for its assertion.

## 9. Decision ledger

- **Resolved:** reorder by moving whole sections (heading hierarchy
  untouched) rather than rebuilding navigation.
- **Resolved:** auto-discover is read-only and failure-safe, so it can run
  unattended on load; writes still need explicit preview + tick.
- **Resolved:** chunking in the panel, not the server: each chunk keeps its
  own digest cycle, so no gate is weakened.
- **Resolved:** catalog stays complete and last — demotion by position,
  not deletion.
- **Blockers:** none.
