# Proposal: web-portfolio-completion

## Why

The Portfolio view (`frontend/index.html` portfolio section) wires only
two of the Forge-owned metadata verbs the CLI already owns: `portfolio
tag add` (`POST …/portfolio/{id}/tags`) and `review set`
(`POST …/portfolio/{id}/reviews`). Everything else the registry already
persists — tag remove/list, relation add/remove/list, goal
add/link/list, evidence import/list, review list, and the
single-project `show` read — has a CLI but no browser surface. Worse,
the two wired writes apply immediately on click: no preview, no
confirmation tick, unlike every other mutating browser surface
(workbench plan→apply, maintain classify approve/reject/apply,
delivery allowlist/approve/publish/reconcile), all of which refuse
implicit mutation with `confirm: true` (+ digest binding where a
manifest exists to hash).

An operator managing cross-project metadata from the dashboard today
must drop to the terminal for removals, lists, goals, relations and
evidence — and the two clicks that do exist bypass the
preview→confirm→apply contract the rest of the dashboard honors.

## What Changes

- **Reads (reuse + extend, never probe):** `GET /v1/admin/portfolio`
  (fleet), `GET /v1/admin/portfolio/evidence` (bundle) and
  `GET /v1/admin/portfolio/{id}` (show) are reused unchanged. The
  read sub-resource `GET /v1/admin/portfolio/{id}/{kind}` grows from
  `evidence`-only to `evidence|tags|relations|reviews|goals`, each
  served from the existing typed registry reads
  (`portfolio_current_snapshots`, `portfolio_tags_for`,
  `portfolio_relations_for`, `portfolio_reviews_for`,
  `portfolio_goals` filtered to the project). Unknown kinds stay an
  honest `404`. No provider, readiness matrix, analytics adapter or
  native build runs on any of these reads.
- **Writes (confirm-gated, Forge-owned only):** every portfolio write
  requires `confirm: true`, else `409 portfolio-confirm-required`
  with `effect: "none"` plus the current-state preview of the exact
  sub-resource the operator reviewed. Covered: existing `tags`,
  `relations`, `reviews`, `goals` actions plus three new typed routes
  following the delivery `allowlist/{id}/remove` precedent:
  `POST …/portfolio/{id}/tags/remove` (`{name, confirm}`),
  `POST …/portfolio/{id}/relations/remove` (`{to, type, confirm}`),
  `POST …/portfolio/{id}/evidence/import` (append-only snapshot
  `{source_system, source_revision, status, observed_at?,
  stale_after?, evidence?, confirm}` via `portfolio_import_snapshot`).
  `POST …/portfolio/{id}/evidence` stays the honest `403
  portfolio-source-owned` edit refusal, unchanged.
- **No digest binding** (deliberate, documented): portfolio has no
  manifest to hash — delivery binds to `manifest_sha256`, workbench to
  the plan hash. The confirm binds to the reviewed project id plus the
  echoed current-state preview instead; the design records why a hash
  would be theater here.
- **No journal rows** (deliberate, documented): the portfolio store has
  no audit table — unlike the delivery share audit trail — so writes
  keep today's `effect: "forge-owned-write"` + `actor` + `recorded_at`
  envelope. Nothing is invented.
- **Frontend:** the existing portfolio metadata card gains the missing
  sections in the smallest coherent layout reusing current
  styles/a11y (`.wb-card`, `.wb-plan-tools`, `.field`,
  `.wb-confirm`, `.error-summary` focus, `role="status"` result
  regions, `role="alert"` errors): a single-project `show` reader,
  tag list + remove, relation add/remove/list, review history list,
  goal add/link/list, evidence import + list. Imported evidence
  renders read-only with its `editable: false` note; no edit control
  is added for it. No new frontend dependency.

## BFS Impact Map

| Surface | Impact |
|---|---|
| `src/api/portfolio.rs` + new `src/api/portfolio_writes.rs` | `READ_KINDS` +4; `write_item` confirm gate; `remove_tag`, `remove_relation`, `import_evidence` handlers; `tags|relations|reviews|goals` list projections; contract version unchanged (`forge-web-portfolio-controls/0.1.0`, additive). Write half split into the sibling file via `#[path]` (verbatim move) so both stay under the 1000-line source-file-size cap |
| `src/api/router.rs` + `model.rs` + `admin/deploy.rs` | 3 new POST arms + variants + dispatch (`tags/remove`, `relations/remove`, `evidence/import`); OPTIONS preflight already covers 4-segment portfolio paths |
| `src/api/command_catalog/` | 3 new `web` rows referencing new route constants; implemented-route count 234 → 237; `catalog_covers_every_clap_path` pin updated |
| `frontend/index.html` + `frontend/app.js` | New card sections + `confirm` checkboxes + list renderers + `request()` calls with `confirm: true`; standalone/JSON-only/shell-free pins extended, never weakened |
| `tests/forge_web_portfolio_controls_contract.rs` | Existing write tests gain `confirm: true`; new tests: confirm refusal (409 + preview + none), remove round-trips, list reads, evidence import append-only + edit still 403, unknown kind 404, path-leak sweep over every new route |
| Canonical specs | `forge-web-portfolio-controls` +N requirements (additive); `portfolio-metadata-and-review` scenarios pinned to the browser verbs |
| CLI / registry Core / journal schema | Unchanged; `forge portfolio …` behaves byte-identically |

## Capabilities

- Operator previews any project's tags, relations, reviews, goals and
  evidence in the browser and applies removals, links, reviews and
  append-only evidence imports only after an explicit confirmation
  tick, with typed refusals (and zero state change) on every invalid,
  unauthenticated, cross-origin or stale request.
- Empty registry, unknown project, hostile-origin, shell-metacharacter
  and absolute-path-leak behavior stay exactly as pinned today.

## Non-goals

- Interest/activity analytics, activation verdicts, share lifecycle
  (all delivered by their own archived changes and canonical specs).
- Provider-backed verbs of any kind; readiness/native matrix runs;
  GitHub/adapter invocation from the browser.
- Digest hashing for portfolio writes (no manifest exists; see design
  §4); journal/audit tables for portfolio (none exist; no schema
  change in this package).
- Editing or deleting imported evidence (append-only is the invariant;
  the 403 stays).
- Any new frontend dependency, inline script, or `innerHTML` sink.
