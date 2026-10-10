# Design: web-portfolio-completion

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate
  (library) plus `frontend/`. No sibling touched.
- **Modules changed:**
  - `src/api/portfolio.rs` + new `src/api/portfolio_writes.rs` —
    extend `READ_KINDS` to
    `["evidence", "tags", "relations", "reviews", "goals"]`; add
    `read_tags`, `read_relations`, `read_reviews`, `read_goals`
    projections reusing `Registry::portfolio_tags_for`,
    `portfolio_relations_for`, `portfolio_reviews_for(…, 50)`,
    `portfolio_goals` (filtered to goals whose `projects` contain the
    id); confirm-gate all four existing `write_item` actions
    (`confirm: true` else `409 portfolio-confirm-required` with the
    current-state preview and `effect: "none"`); add `remove_tag`
    (`portfolio_remove_tag`), `remove_relation`
    (`portfolio_remove_relation`), `import_evidence`
    (`portfolio_import_snapshot` after `SnapshotWrite` validation,
    defaulting `observed_at` to now); add `ROUTE_ADMIN_PORTFOLIO_TAG_REMOVE`,
    `ROUTE_ADMIN_PORTFOLIO_RELATION_REMOVE`,
    `ROUTE_ADMIN_PORTFOLIO_EVIDENCE_IMPORT` constants (delivery
    `ROUTE_DELIVERY_*` pattern). The write half (`write_item`,
    `remove_tag`, `remove_relation`, `import_evidence` plus the
    confirm gate) lives in `portfolio_writes.rs`, wired as
    `#[path = "portfolio_writes.rs"] mod writes` with re-exports so
    every `portfolio::` path stays byte-identical for callers; the
    split keeps both files under the 1000-line `source-file-size`
    cap (verbatim move, no behavior change — the
    `portfolio-mod-size-split` precedent). A `portfolio/` directory
    split was rejected: the gate's source scan enumerates HEAD-tracked
    paths, so deleting the tracked `portfolio.rs` before commit reads
    as a missing file.
  - `src/api/model.rs` — new variants `AdminPortfolioTagRemove { id }`,
    `AdminPortfolioRelationRemove { id }`,
    `AdminPortfolioEvidenceImport { id }` with doc comments.
  - `src/api/router.rs` — three new arms
    (`POST ["v1","admin","portfolio",id,"tags","remove"]`,
    `…,"relations","remove"`, `…,"evidence","import"`); thread the
    variants through the `required_permission` (None, same as the
    other admin session-gated routes), the `admin::handle`
    short-circuit list, and the unreachable-but-exhaustive arm list.
    The existing 4-segment OPTIONS arm
    `("OPTIONS", ["v1","admin","portfolio",_,_])` already covers the
    5-segment preflights? No — it matches exactly 4 segments. A new
    6-segment? These are 5-segment paths
    (`v1/admin/portfolio/{id}/tags/remove` = 6 segments). Add one
    `("OPTIONS", ["v1","admin","portfolio",_,_,_])` arm mirroring the
    six-segment lifecycle-write precedent.
  - `src/api/admin/deploy.rs` — dispatch the three variants to the new
    portfolio handlers through `guarded` (+ `is_json` 415 guard, same
    as `AdminPortfolioWrite`).
  - `src/api/command_catalog/routes.rs` (+ row table) — three `web`
    rows for the new routes; `IMPLEMENTED_WEB_ROUTES` gains the three
    constants.
  - `frontend/index.html` — extend the `portfolio-metadata-title` card
    with: show reader (`portfolio-show` result `dl`), tag remove
    (`portfolio-tag-remove-*`), relation add/remove
    (`portfolio-relation-*`), review list (`portfolio-reviews`),
    goal add/link (`portfolio-goal-*`), evidence import
    (`portfolio-evidence-*`); every write form gets a `.wb-confirm`
    checkbox; every list region gets `role="status"`.
  - `frontend/app.js` — list renderers + confirm-gated `request()`
    callers + error-summary wiring, reusing `clearFieldError`,
    `setFieldError`, `renderErrorSummary`, `showPortfolioNotice/Error`
    exactly as the existing two forms do.
  - `tests/forge_web_portfolio_controls_contract.rs` — update the
    three existing write call sites with `"confirm":true`; add tests
    per design §8.
- **Modules reused unchanged:** all `Registry::portfolio_*` Core
  functions, `RelationType::parse`, `Confidence::parse`,
  `Lifecycle::parse`, `validate_goal_status`, `SnapshotWrite`
  validation, `require_id`, `typed_refusal`, `refuse`, `scrub`,
  `guarded`, session/CORS layers.
- **Must NOT change:** existing route paths and shapes (additive
  fields only: responses keep every current key), contract version
  `forge-web-portfolio-controls/0.1.0`, CLI behavior, registry
  schema, `API_CONTRACT_VERSION`, the `POST …/evidence` 403 refusal.

## 2. Language and runtime

Rust 2021 (`rustc 1.87` floor per workspace) + standalone
HTML/CSS/JS frontend (no build step, no dependency). Contract tests
are Rust integration tests driving `forge::api::handle` in-process.

## 3. Contracts

### Reads

- `GET /v1/admin/portfolio/{id}/tags` →
  `{contract, project_id, tags: [{name, color?, created_at}]}`.
- `GET …/relations` →
  `{contract, project_id, relations: [{from_project, to_project,
  relation, note?, created_at}]}` (both directions, as
  `portfolio_relations_for` returns).
- `GET …/reviews` →
  `{contract, project_id, reviews: [{confidence, note?, reviewed_at}]}`
  newest-first, capped at 50 (same cap as the CLI list).
- `GET …/goals` →
  `{contract, project_id, goals: [{goal_id, title, status,
  description?, created_at, projects}]}` filtered to membership.
- `GET …/evidence` — unchanged shape (reused as the evidence list).
- `GET /v1/admin/portfolio/{id}` — unchanged shape (reused as show).

### Writes (all confirm-gated)

Missing or false `confirm` → `409 {contract, error:
{code: "portfolio-confirm-required", message}, effect: "none",
preview: <current sub-resource value>, confirmation: {requires:
["confirm"], note}}`. The preview echoes the exact state the
operator reviewed; nothing is written.

- `POST …/tags` `{name, color?, confirm: true}` (existing + gate).
- `POST …/tags/remove` `{name, confirm: true}` →
  `{…, result: {removed: bool}}`; unknown tag removes nothing and
  reports `removed: false` with 200 (idempotent, same as
  `portfolio_remove_tag`'s CLI behavior).
- `POST …/relations` `{to, type, note?, confirm: true}` (existing +
  gate).
- `POST …/relations/remove` `{to, type, confirm: true}` →
  `{…, result: {removed: bool}}`.
- `POST …/reviews` `{confidence, lifecycle?, note?, next_action?,
  blocker?, confirm: true}` (existing + gate).
- `POST …/goals` `{title, status, description?, confirm: true}` for
  create/re-stamp and `{title, confirm: true}` for link (existing
  combined behavior + gate; status defaults to `planned` on the link
  path exactly as today).
- `POST …/evidence/import` `{source_system, source_revision, status,
  observed_at?, stale_after?, evidence?, confirm: true}` →
  `{…, result: {snapshot}}`; `status` parses via the same
  `parse_evidence_status` vocabulary the CLI uses
  (`observed|unavailable|invalid`), unknown → typed 400, nothing
  stored. The payload is redacted/bounded by Core
  `validate_snapshot` before write.

Every success keeps the envelope `{contract, project_id, action,
effect: "forge-owned-write", actor: "global-admin", recorded_at,
result}`.

## 4. Why no digest, why no journal

- **Digest:** delivery binds `confirm` to `manifest_sha256` because a
  share manifest exists to hash; workbench binds to the plan hash.
  Portfolio mutations have no multi-row manifest — each targets one
  typed row identified by `(project_id, name|to+type|title)`. Hashing
  the echoed list would bind confirmation to ordering and
  timestamps the operator never reviews, i.e. theater that fails on
  concurrent unrelated writes. The confirm binds to the reviewed
  `(project_id, key)` plus the echoed preview instead; a changed row
  surfaces on the next preview read. This is recorded here so a
  reviewer does not read the absence as an oversight.
- **Journal:** the registry has share-audit, operation and gate
  evidence tables; it has no portfolio-audit table, and this package
  performs no schema migration (out of scope for a UI-completion
  gap). Writes therefore keep the existing envelope evidence
  (`effect`, `actor`, `recorded_at`) with no new table.

## 5. Failures

| Input | Response | State change |
|---|---|---|
| No session | 401 | none |
| Hostile origin (+ preflight) | 403 | none |
| Bad id shape / traversal | 400 `portfolio-invalid`, input never echoed | none |
| Unknown / observed-only project | 404 `portfolio-unmanaged-project` | none |
| Unknown read kind | 404 `portfolio-route-not-found` | none |
| Missing/false `confirm` | 409 `portfolio-confirm-required` + preview | none |
| Invalid enum (relation type, confidence, lifecycle, goal status, evidence status) | 400 `portfolio-invalid` | none |
| Self-relation / duplicate / bad tag | typed Core refusal (400/409 as today) | none |
| Edit attempt on `…/evidence` | 403 `portfolio-source-owned` | snapshot provably unchanged |
| Registry unreadable | 503/503-style refusal | none |

## 6. Security

Id-only addressing (validated kebab-case, never a path); fixed action
vocabulary (unknown segments 404); whole-response path scrub retained
on every new projection; Core `Display` never echoed except through
`status_reason` + scrub; session gate + exact-origin CORS unchanged;
no shell, no interpreter, no subprocess, no browser-supplied path, no
provider contact on any read or write in this package.

## 7. Frontend semantics

Preview → confirm → apply per card section: the list region above
each form is the preview (fresh `GET` on section open, project
change, and after every apply); the `.wb-confirm` checkbox is the
confirm; the button POSTs `{…, confirm: ticked}`. Unticked → client
blocks with the error-summary + field error before any request
(matches delivery card behavior of refusing implicit mutation without
a round trip). Results render into `role="status"` regions;
failures into `role="alert"` + error-summary with focus (existing
`renderErrorSummary` focuses the summary). Evidence import renders
the returned snapshot read-only; no edit control exists for
`snapshots`. Zero `innerHTML`, zero inline `<script>`, zero new
dependency.

## 8. Test oracle

Extend `tests/forge_web_portfolio_controls_contract.rs` (no new test
binary — the change's oracle stays in the surface's contract file):

1. `portfolio_writes_require_confirm` — each of the 7 write entries
   without `confirm` → 409, `effect: none`, preview present,
   registry unchanged (tag count / relation count / review count /
   goal count / snapshot count identical before/after).
2. `portfolio_tag_remove_round_trip` — add → list contains → remove
   (confirm) → list empty; second remove → `removed: false`, 200.
3. `portfolio_relation_remove_round_trip` — add `depends-on` →
   list shows both directions → remove → list empty; self-relation
   still 400.
4. `portfolio_lists_read_back` — tags/relations/reviews/goals GETs
   return what the writes stored; goals list filters to membership
   (link goal to B, A's list excludes it).
5. `portfolio_evidence_import_is_append_only` — import (confirm) →
   200 + snapshot echoed; evidence list shows it with
   `editable: false`; `POST …/evidence` still 403 and count
   unchanged; second import with bad status → 400, count unchanged.
6. Update existing `forge_owned_metadata_mutations_are_recorded_and_read_back`
   call sites to include `"confirm":true` (behavior otherwise
   identical).
7. Extend `frontend_portfolio_is_standalone_json_only_and_shell_free`
   with the new endpoint/marker pins (`/tags/remove`,
   `/relations/remove`, `/evidence/import`, `/goals`, confirm
   checkbox ids) and run the forbidden-sink sweep over the enlarged
   portfolio region.
8. Path-leak sweep: every new route's success + refusal bodies
   asserted free of the fixture's absolute path.
