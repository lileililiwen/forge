# Proposal: Flywheel plugin cap grouping

## Why

Three measured gaps on `main` at `f435f3b` after `lifecycle-series-rail`:

- **Gap1 — flywheel breaks at the ends.** The 8-step rail renders but step 0 (Idea) has no entry (no graduation preview/import, no studio spec link) and a published delivery ends without a maintain loop (no Next-idea prompt, no maintain refresh shortcut). Operators cannot walk idea→scaffold→gate→publish→maintain in the browser.
- **Gap2 — plugin vocabulary too narrow.** `src/plugins/mod.rs` `CAPABILITIES` holds 6 metadata/delivery tokens and `PluginKind` holds `Metadata|Delivery` only. Real integrations (`driftwatchdog`, `cargo-*`, `sisyphusfy`/`ariadex`/`mnemora`, `platform-contracts`, `labrys`/`openpanel`/`jenkins-local`, `argoscope`/`devloom`) have no kind to resolve to, and there is no `gate_plugins()` routing beside `metadata_plugins()`.
- **Gap3 — flat CLI with no grouping.** `gate`/`check`/`doctor`/`readiness`, `agent`/`studio`/`intent`, `contract`/`component`/`standard`, `delivery`/`deploy`/`publish` are flat top-level commands with no `forge cap list|inspect|add|run` facade, no portal/web cap filter or rail badge, and no demo script tying the loop together.

## What Changes

- **Flywheel web (`frontend/`).** Workbench rail step 0 gains `#wb-idea-entry` (graduation preview/import CLI preview + studio spec entry link, `?project=`+`?step=` preserving). Delivery publish outcome renders `#delivery-next-idea` (Next-idea prompt + workbench `&step=idea` link + maintain refresh shortcut firing `#wb-maintain-refresh`). One canonical doc `docs/flywheel-demo.md` walks `hookit` through idea→scaffold→gate→publish→maintain with exact CLI + web URLs.
- **Plugin reinforcement (`src/plugins/mod.rs`).** `CAPABILITIES` grows to 11 (`topic,description,homepage,language,publish,delivery,gate,quality,agent,contract,analytics`); `PluginKind` gains `Gate,Quality,Agent,Contract` (serde lowercase, `as_str`/`parse` extended; `Metadata|Delivery` behavior unchanged, unknown stays `Invalid`). Builtin table `builtin_descriptor(id)` resolves `driftwatchdog=gate`, `cargo-*=quality`, `sisyphusfy|ariadex|mnemora=agent`, `platform-contracts=contract`, `labrys|openpanel|jenkins-local|jenkins=delivery`, `argoscope|devloom=metadata+analytics`, consulting `kits/manifest.json` `plugins` array first when present; `providers.yaml` descriptor overrides builtin. New `gate_plugins()` (+ `quality/agent/contract_plugins()`) beside `metadata_plugins()`.
- **Cap grouping CLI (`forge cap`).** New `Commands::Cap` facade (`src/cli/commands.rs` `CapCommands`, `src/cli/cap.rs` `cmd_cap`, dispatch in `src/main.rs`): `cap list` (from `CAPABILITIES` + live `plugins list` states), `cap inspect <cap>` (plugins + mapped flat commands), `cap add` (descriptor guidance), `cap run <cap>` (underlying command hint). All old flat commands stay as working aliases. Portal `projects` controls gain `forge cap list|inspect`; entries gain `cap_group` attribute. Web projects view gains `#cap-filter` group filter; workbench rail gains `#cap-badge`.
- **Tests/browser.** Extend `tests/portal_ui_contract.rs` (idea entry, next-idea loop, cap filter/badge, no new dep) and `tests/browser/lifecycle-rail-check.mjs` + `tests/lifecycle_rail_browser.rs` (rail step0, `forge cap list` JSON, demo URLs). Reuse pinned `tests/browser` playwright 1.63.0, no new frontend dep.

## BFS Impact Map

- **Capabilities:** `portal-web-ui` (idea entry, publish→maintain loop, cap filter/badge), `forge-publish-plugin-orchestration` (extended vocabulary, builtin resolution, gate routing, cap facade).
- **Users/flows:** browser operators walking idea→maintain; CLI operators grouping gate/agent/contract/delivery work through `forge cap`; portal readers filtering projects by cap group.
- **Contracts/data/persistence:** no registry/journal/API schema change; `plugins list` JSON shape additive (`kind` gains 4 values, `capabilities` gains 5 tokens); `cap list` JSON is new (`contract: forge-cap/0.1.0`); portal `controls_available` additive; frontend adds elements only.
- **Integrations:** `kits/manifest.json` optional `plugins` array (absent → hardcoded builtin table); `providers.yaml` `plugins:` block overrides builtin; `gh`/adapters untouched.
- **Tests:** `portal_ui_contract` (+5), `lifecycle_rail_browser` harness (+3 notes), `cargo test --lib plugins::` (builtin + gate routing).
- **Compatibility:** old flat commands byte-identical; unknown kind/capability stays `Invalid`; no-config `plugins list`/`cap list` stays honest-empty; no new endpoint/framework.

## Capabilities

- Idea entry in rail step 0 with graduation + studio links.
- Publish→maintain loop (Next-idea prompt + maintain refresh shortcut).
- Canonical `docs/flywheel-demo.md` hookit 5-step demo with web URLs.
- Extended plugin vocabulary + builtin resolution + gate routing.
- `forge cap list|inspect|add|run` facade grouping flat commands.
- Portal cap controls + `cap_group` attribute; web cap filter + rail badge.

## Non-goals

- No new API route, no new portal section, no registry/journal migration.
- No new plugin transport, privilege, or invocation path (listing only).
- No new frontend framework, chart, or browser dependency.
- No breaking CLI rename or removal; no auto-install of adapters.
