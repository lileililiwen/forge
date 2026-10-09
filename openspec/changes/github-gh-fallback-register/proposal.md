# Proposal: GitHub gh-fallback and create register-if-missing

## Why

Two measured gaps on `main` (2026-10-09):

- **Gap1 — `forge project github observe/propose` returns `github-adapter-unavailable`.** No `forge-github-metadata-adapter` on `PATH`, no `FORGE_GITHUB_BIN` override. Auth/clone/create work via the installed `gh` CLI (`gh auth status` logged in), so read-only metadata stays unreachable while the credential boundary that already works is unused.
- **Gap2 — `forge project github create --push-source` friction for unregistered dev repos.** `forge list` shows 7 registered projects; real dev repos (e.g. sibling checkouts with a valid `forge.yaml`) must be registered first in a separate step before the remote can be created and the source pushed.

## What Changes

- `src/github/gh_fallback.rs` (new): vendored `gh`-backed read path — `gh repo view <owner/repo> --json ...` for read-only observe (topics/description/default-branch/archived/language), `gh repo edit <owner/repo> --add-topic <topic>` for direct-mode single-topic propose with an explicit `--confirm` echo check. Bounded argument arrays, `GITHUB_CLI_TIMEOUT`, credential redaction; PR mode unchanged (still requires the adapter).
- `src/cli/project.rs`: `observe`/`propose` try the adapter first; when no adapter binary is available and `gh` is, they serve the `gh` fallback and label the envelope `adapter_source=gh-cli-fallback`. Token checks are bypassed only on the fallback path (`gh` owns auth); tokens never logged.
- `src/cli/commands_ops.rs` + `src/cli/github.rs`: `create` gains `--register-if-missing` — validate `forge.yaml` via `Manifest::load_from_dir`, register the canonical path, then run the existing `gh repo create` (+ optional `--push-source`). Without the flag behavior is unchanged.
- Portal (`src/portal/sections.rs`): `repositories` controls gain the three github commands; no new section, no route change.
- Web (`frontend/`): one `GitHub metadata` card in Delivery — topics `<ul>` with dev-highlight + text alternative, propose form (repo/field/value/mode radios/confirm token field), create flow (project/repo/visibility radios/push-source + register-if-missing checkboxes). Native controls, labels, focus, keyboard operable; appended CSS block only.

## BFS Impact Map

- **Capabilities:** `github-project-metadata-adapter` (fallback read + direct topic write), `github-cli-project-workflows` (register-if-missing create), `portal-web-ui` (github metadata views).
- **Users / flows:** operators with `gh` auth but no metadata adapter (observe/propose); operators creating remotes for unregistered checkouts (create); browser operators reading topics/proposing/creating from Delivery.
- **Contracts / data / persistence:** `forge-github-metadata/0.1.0` JSON shape unchanged (fallback synthesizes the same `GithubObservation`/`ProposeOutcome`); `forge-github-cli-workflows/0.1.0` gains `register_if_missing` + `registered` booleans (additive); registry writes only via existing `Registry::register`; no journal shape change; no new API route.
- **Integrations:** `gh` CLI only (`repo view`, `repo edit`, `repo create`); adapter path byte-identical when configured.
- **Tests:** new `tests/github_gh_fallback_register_contract.rs` (fallback builders, confirm gate, topic-only gate, redaction, register-if-missing, frontend/portal tokens).
- **Compatibility:** adapter-first ordering; PR-mode propose still requires the adapter; create without the flag behaves as before; frontend adds one card, no endpoint change.

## Capabilities

- GitHub metadata observe via vendored `gh` fallback when the adapter binary is missing.
- Direct-mode single-topic propose via `gh repo edit --add-topic` with `--confirm` echo; PR mode unchanged.
- `create --register-if-missing`: validate manifest, register, then create remote + optional push-source.
- Portal + web GitHub metadata views (topics list with dev-highlight, propose form with confirm field, create flow with visibility radios + push-source checkbox).

## Non-goals

- No new Git provider, no adapter auto-install, no `gh` auth creation, no token persistence/logging.
- No multi-field direct fallback (only single `topic=`; other fields stay adapter-only).
- No PR creation via fallback, no visibility change via fallback, no new JSON API routes.
- No new frontend framework, no chart library, no tag-cloud canvas (plain list + text alternative).
