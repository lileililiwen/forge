# Design: Flywheel plugin cap grouping

## 1. Decisions

- **One change, three gaps (chosen).** The flywheel ends, the plugin vocabulary, and the cap grouping are one operator loop (idea→publish→maintain through named capabilities), so one `flywheel-plugin-cap` change with two delta specs (`portal-web-ui`, `forge-publish-plugin-orchestration`) keeps the rail, registry, and CLI consistent. Alternative (three changes) rejected: triple Gate/validate/archive for one loop wastes the bounded timeout and risks rail/badge drift.
- **Rail step 0 is an entry, not a derivation change (chosen).** `railStepStates`/`railCurrentStep` stay byte-identical; `#wb-idea-entry` is a render-only block inside the lifecycle card naming `forge graduation preview|import` + `forge studio spec` + a `?step=spec` link. Alternative (re-deriving idea from graduation rows) rejected: no graduation journal projection exists in the workbench payloads; a link + CLI preview is honest.
- **Publish loop is outcome-rendered, not a new verb (chosen).** `deliveryPublish()` after `loadDelivery()` calls `renderDeliveryNextIdea(body)` writing `#delivery-next-idea` (Next-idea prompt + `/workbench?project=<id>&step=idea` + button firing `#wb-maintain-refresh` via `maintainRefreshShortcut()`). No new endpoint, no journal write. Alternative (new delivery verb) rejected: the loop is guidance, not state.
- **Builtin first, descriptor override (chosen).** `builtin_descriptor(id)` checks `kits/manifest.json` `plugins` array (when the file exists and parses) then the hardcoded table (`driftwatchdog=gate[gate]`, `cargo-*=quality[quality]`, `sisyphusfy|ariadex|mnemora=agent[agent]`, `platform-contracts=contract[contract]`, `labrys|openpanel|jenkins-local|jenkins=delivery[delivery]`, `argoscope|devloom=metadata[analytics]`); `record_for` prefers the `providers.yaml` descriptor when present. Unknown kind/capability stays `Invalid` with the same `unknown plugin kind` / `closed set` reasons. Alternative (descriptor-only) rejected: existing configs carry no descriptor and must resolve to a useful kind.
- **`gate_plugins()` mirrors `metadata_plugins()` (chosen).** Same Ready+enabled filter, kind-gated (`Gate` for gate; sibling `quality/agent/contract_plugins()` same shape). Alternative (generic `plugins_for(kind)`) rejected: the existing caller shape is `metadata_plugins(records, fields)`; mirroring keeps call sites obvious.
- **Cap facade is guidance + live states, not a re-dispatch (chosen).** `forge cap list` enumerates `CAPABILITIES` with Ready counts from live `plugins::list`; `inspect <cap>` names providing plugins + mapped flat commands (`gate: gate/check/doctor/readiness`, `agent: agent/studio/intent`, `contract: contract/component/standard`, `delivery: delivery/deploy/publish`, `quality: check/doctor`, `analytics: analytics`, `metadata: classify/describe`); `add` prints the descriptor snippet; `run` prints the exact underlying `forge …` string. Old commands are untouched aliases. Alternative (real re-exec) rejected: re-dispatch duplicates arg parsing and risks breaking the flat surface.
- **Portal + web are additive tokens (chosen).** Portal projects `controls_for` gains `forge cap list` + `forge cap inspect <cap>`; `build_projects_section` gains `cap_group` (profile-routed: default `delivery`). Web projects panel-tools gains `<select id="cap-filter">` (All + 7 groups) filtering via `projectCapGroup()`; rail gains `<span id="cap-badge">`. Alternative (new section/view) rejected: twelve-section contract and route allowlist stay stable.

## 2. Implementation boundary

- **Files changed:**
  - `src/plugins/mod.rs` — extended `CAPABILITIES`, `PluginKind{Gate,Quality,Agent,Contract}`, builtin table + `kits/manifest.json` first-read, `gate/quality/agent/contract_plugins()`, override ordering.
  - `src/cli/commands.rs` — `CapCommands{List,Inspect,Add,Run}`.
  - `src/cli/cap.rs` (new) — `cmd_cap`, `CAP_GROUPS`, live-states list.
  - `src/cli/mod.rs` — `pub mod cap;`.
  - `src/main.rs` — `Commands::Cap` + dispatch.
  - `src/portal/sections.rs` — projects controls + `cap_group` attribute.
  - `frontend/index.html` — `#cap-filter` select, `#cap-badge`, `#wb-idea-entry`, `#delivery-next-idea` shells.
  - `frontend/app.js` — `projectCapGroup`, `applyCapFilter`, `renderCapBadge`, `renderIdeaEntry`, `renderDeliveryNextIdea`, `maintainRefreshShortcut`.
  - `frontend/styles.css` — appended flywheel-cap block only.
  - `docs/flywheel-demo.md` (new) — hookit 5-step demo.
  - `tests/portal_ui_contract.rs` — +5 token tests.
  - `tests/browser/lifecycle-rail-check.mjs`, `tests/lifecycle_rail_browser.rs` — step0 + cap list + demo URLs.
  - OpenSpec package deltas.
- **Modules reused unchanged:** provider transport, registry, journal, API routes, portal roll-up, frontend request/error-summary/clipboard helpers, gate runtime.
- **Must NOT change:** flat CLI behavior, `plugins list` envelope (additive only), portal section ids, contrast/touch/motion tokens, `kits/manifest.json` feed bytes.

## 3. Language and runtime

Rust (stable, `cargo fmt`/`cargo build`/`cargo test`) for plugins/CLI/portal; vanilla JS ES2021 + vanilla CSS for `frontend/`; Markdown for the demo doc. Verification: `cargo test --test portal_ui_contract --test lifecycle_rail_browser`, `cargo test --lib plugins::`, `openspec validate --all --strict --no-interactive`, `git diff --check`, `forge gate --dry-run` + `forge gate --timeout-secs 500`.

## 4. Failure modes

- Unknown kind/capability → `Invalid` with `unknown plugin kind` / `closed set`, advertises nothing.
- Missing `kits/manifest.json` or unparsable `plugins` key → hardcoded builtin table (never a failure).
- `providers.yaml` descriptor present → overrides builtin even when builtin names the id.
- No providers file → `plugins list` honest-empty; `cap list` shows vocabulary with zero Ready rows.
- `cap inspect <unknown>` → typed `cap-invalid` listing the closed `CAPABILITIES`.
- `?step=bogus` still ignored; rail `aria-current` never moves on step navigation.
- Publish failure → no Next-idea block (loop renders on success only).

## 5. Accessibility / responsive

Labels on `#cap-filter` (`Filter by capability group`), rail badge is `<span role="status">` text (never color-only), idea entry uses native links/buttons with 44px minima via existing `.button`, error summaries keep `tabindex="-1"` + focus move, `#delivery-next-idea` result uses `role="status"`, appended CSS stacks under existing grid and respects `prefers-reduced-motion`.
