# Tasks: Maintainer plugin platform

## 1. BFS — Baseline and impact coverage

- [x] Survey the workspace: 96 siblings in `~/code`, 7 registered.
- [x] Read every candidate: `src/import/mod.rs` detectors,
  `src/catalog/source.rs` `local_record`,
  `src/catalog/query.rs` predicates, `src/semantic/`,
  `src/github/`, `src/publish/providers.rs`,
  `src/delivery/handlers.rs`, `frontend/`.
- [x] Measure the actual state: `forge project list --format json`
  returns no `compose`, `ci`, `topics` or `description` on any of
  the 7 records; `classify suggest` requires `--suggested-value`;
  `semantic::approve` writes nothing by design.
- [x] Confirm the plugin transport to reuse: `{id, command,
  enabled}` over JSON stdin/stdout, bounded argv, per-run timeout,
  secret-leak rejection.
- [x] Add the change package with proposal, design and a
  `maintainer-plugin-platform` delta spec.

## 2. DFS — Evidence

- [x] Make `detect_compose` / `detect_ci` reachable outside adopt
  time (visibility only, no logic change).
- [x] Populate `compose` and `ci` in `local_record`; report an
  unreadable directory as `none` + evidence unavailable rather than
  a false `none`.
- [x] Test: register a project with `compose.yaml` and a
  `Dockerfile`, assert `--compose docker+compose` returns it;
  assert a Dockerfile-only project is `docker` and is not returned
  by `--compose docker+compose`.

## 3. DFS — Derivation

- [x] Add `forge classify derive` reading only local evidence:
  manifest `profile`/`maturity`/`target_maturity`, observed GitHub
  `topics`/`language`, README's first heading.
- [x] Record each proposal's evidence and a bounded confidence;
  no model, no network.
- [x] Test: derive twice on an unchanged project records the same
  proposals; derive writes no project field.

## 4. DFS — Plugin registry

- [x] Add `src/plugins/` — read descriptors, validate capabilities
  against the closed set, report unknown capability as `invalid`.
- [x] Treat a descriptor-less plugin as `delivery` so existing
  `providers.yaml` needs no edit.
- [x] Add `forge plugins` listing id, kind, enabled, capabilities.
- [x] Add the `forge-metadata-propose/0.1.0` request kind routed
  to a plugin advertising `kind: metadata`; an unrecognised kind
  answers `unsupported` as a capability gap.
- [x] Test: a delivery-only install lists delivery plugins; an
  invalid capability is refused; a missing command reports
  unavailable without hiding the others.

## 5. DFS — Apply

- [x] Add `forge classify apply --confirm`: refuse any proposal not
  already `Approved`, by name.
- [x] Project approved values onto the four permitted fields;
  refuse anything outside them rather than dropping silently.
- [x] Send one PR-mode `forge-metadata-propose/0.1.0` request via
  `invoke_provider`; record a journal row naming plugin, fields
  and the PR reference. No direct mutation path.
- [x] Test: unapproved refused by name; no metadata plugin is a
  refusal naming the capability; an out-of-scope field is refused.

## 6. DFS — Browser

- [x] Add the fleet filter row (language, lifecycle, profile,
  compose, CI, tag) wired to `GET /v1/projects/catalog`, composing
  with the existing free-text search. No new endpoint.
- [x] Add the per-project Maintain card: observed GitHub metadata
  with freshness, derived proposals with per-field approve/reject,
  and one apply action. Reuse `buildActionControl`.
- [x] Render an unreachable remote as `unavailable` with its reason,
  never as empty fields.
- [x] Test: filter row and `forge project list` agree for the same
  predicate.

## 7. BFS — Regression and completeness

- [x] Every existing CLI command and flag unchanged.
- [x] The plugin transport, the GitHub adapter's PR-by-default
  mutation and the semantic proposal contract are unchanged.
- [x] Registry and journal schemas unchanged.
- [x] Every surviving test target green at its previous count.

## 8. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] The new contract tests green.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and
  `openspec validate --all --strict --no-interactive` 0 failures.
- [x] `git diff --check` clean.
- [x] Manual: `forge plugins`; `forge project list --compose
  docker+compose`; `forge classify derive`; the browser filter row
  and Maintain card.
- [x] `forge gate` local run recorded.
- [x] Archive with `openspec archive maintainer-plugin-platform
  --yes`; promote the canonical spec; update HANDOFF.