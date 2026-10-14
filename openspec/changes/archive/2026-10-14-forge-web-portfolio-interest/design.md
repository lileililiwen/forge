# Design: forge-web-portfolio-interest

## Ownership

The browser is a thin, read-only caller of the existing `forge::portfolio::interest_report` Core path
the CLI already uses. The three portal reads map directly onto that Core:

- `GET /v1/admin/portfolio/{id}/interest/list` and `.../show` both call
  `interest_report::project_interest(&registry, id, stale_after_days, now)` — the same function
  `handle_get_interest` calls (`src/api/handlers_portfolio.rs:670`). `list` renders the snapshot list
  (with per-window `freshness`); `show` renders the same projection including `findings` (refusals).
- `GET /v1/admin/portfolio/{id}/interest/trend` calls
  `interest_report::interest_trend(&registry, id, metric, limit, stale_after_days, now)` — the same
  function `handle_interest_trend` calls (`src/api/handlers_portfolio.rs:882`). The returned `Trend`
  already carries per-point `freshness`.

Nothing new touches `src/portfolio/**`. The handlers live in a new `src/api/admin/interest.rs`,
registered in `src/api/admin/mod.rs::route_beside` so `src/api/router.rs` adds **zero** lines — the
precedent is `agent_identity::route_agent_identity` / `history::route_history`, consumed via a single
`.or_else` call. The three new `Route` variants live in `src/api/model.rs` beside the other
`AdminPortfolio*` variants, and the three classification tables in `src/api/router_classify.rs` gain
three arms each (one per match). Dispatch lands in `src/api/admin/deploy.rs::handle` via three new
arms calling `super::interest::*`.

## Contracts

### Wire shape

```
GET /v1/admin/portfolio/{id}/interest/list   -> 200 { project_id, measured, stale_label,
                                                  stale_after_days, stale_snapshots, snapshots[] }
GET /v1/admin/portfolio/{id}/interest/show   -> 200 { project_id, measured, stale_label,
                                                  stale_after_days, projection{ snapshots[],
                                                  stale_snapshots, superseded_snapshots, findings[] } }
GET /v1/admin/portfolio/{id}/interest/trend  -> 200 { project_id, metric, stale_after_days,
                                                  interest{ trend{ points[], unreported_windows, note } } }
```

`{id}` is a validated project id passed through `interest_report`, which calls
`registry.require_project` and returns an honest refusal for an unmanaged/observed-only id (no id is
echoed). `stale_after_days` is parsed by the existing `parse_interest_stale_after_days`
(`src/api/server.rs:184`); an out-of-range value is the same typed `400` the CLI path returns. `trend`
parses `metric` (via `InterestMetric::parse`), `limit` (default 12, clamped to 1..=`MAX_TREND_POINTS`)
and `stale_after_days` from the query string.

### Stale labeling (the never-synthesize invariant)

`project_interest` derives `stale_snapshots` from `freshness_label(window_end, now, stale_after_days)`
at read time. The handlers re-use that exact function to attach a per-snapshot `freshness`
(`"current"` / `"stale"`) and a project-level `stale_label`:

- `measured == false` → `no-evidence` (no snapshots stored)
- `stale_snapshots > 0` → `stale` (at least one current window is older than the bound)
- otherwise → `current`

The UI renders a `stale` chip whenever `freshness == "stale"` (or `stale_label == "stale"`), using the
existing `badge-stale` / `badge-fresh` / `badge-unavailable` classes — never a synthesized
`current`. `show`'s `findings` array is rendered verbatim (field + code + detail), never summarized
into a clean projection. The `trend` points already carry `freshness`, and the UI reuses the same
chip logic.

### Success envelopes (200)

```json
// list
{
  "project_id": "<id>",
  "measured": true,
  "stale_label": "stale",
  "stale_after_days": 14,
  "stale_snapshots": 1,
  "snapshots": [ { ...serialized InterestSnapshot..., "freshness": "stale" } ],
  "contract": "forge-portfolio-interest/0.1.0"
}
// show
{
  "project_id": "<id>",
  "measured": true,
  "stale_label": "current",
  "stale_after_days": 14,
  "projection": {
    "project_id": "<id>",
    "snapshots": [ { ...with "freshness" } ],
    "stale_after_days": 14,
    "stale_snapshots": 0,
    "superseded_snapshots": 0,
    "findings": [ { "index": 0, "project_id": "...", "field": "...", "code": "...", "detail": "..." } ]
  },
  "contract": "forge-portfolio-interest/0.1.0"
}
// trend
{
  "project_id": "<id>",
  "metric": "unique_visitors",
  "stale_after_days": 14,
  "interest": {
    "trend": { "project_id": "<id>", "metric": "unique_visitors",
               "points": [ { "window_start": "...", "window_end": "...", "value": 12,
                            "privacy_mode": "...", "coverage": "...", "freshness": "current",
                            "source": "..." } ],
               "unreported_windows": 0, "note": "..." }
  },
  "contract": "forge-portfolio-interest/0.1.0"
}
```

## Failures

- **Unmanaged / observed-only id:** `interest_report::require_project` returns a typed
  `portfolio-unmanaged-project` (or equivalent) refusal; the handler returns it unchanged via
  `ApiResponse::from_error`. No project id is echoed.
- **Out-of-range `stale_after_days`:** the existing `parse_interest_stale_after_days` returns a typed
  `400`; the handler wraps it in the `cors` envelope (same posture as every other admin handler) and
  returns it.
- **Missing `metric` on `trend`:** typed `400 admin-invalid-interest-metric`.
- **Stale / no evidence:** honest empty/`stale` state, never a fabricated `current` or `ready`. The
  `measured:false` + `stale_label:"no-evidence"` pair is exactly the CLI's "no snapshots" reading.
- **No snapshots yet:** `list`/`show` return `measured:false`, `stale_label:"no-evidence"`,
  empty `snapshots`/`findings` — the portal renders an honest empty state, never a healthy one.

## Migrations

None. No schema, no registry migration, no new contract version beyond reusing
`INTEREST_CONTRACT_VERSION`. The change is purely additive read surface over an existing Core store.
