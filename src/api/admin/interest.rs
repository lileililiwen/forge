//! Read-only portfolio interest reads for the portal panel
//! (`forge-web-portfolio-interest`).
//!
//! Three session-gated GET handlers over the existing Core interest store
//! the CLI already reads via `forge::portfolio::interest_report`:
//! `project_interest` (per-project snapshot list + whole projection with
//! refusals) and `interest_trend` (one metric's windowed history). No
//! write, no provider, no adapter, no shell, no browser-supplied path on
//! any path; every response carries the interest contract version and is
//! free of absolute filesystem paths.
//!
//! The recorded-never-synthesized invariant is preserved literally:
//! freshness is derived via `interest::freshness_label` at read time
//! (identical to the CLI), a stale window keeps its value and gains a
//! `stale` label, and `findings` (refusals) are rendered verbatim.
//! Nothing here invents a `current` label or hides a refusal.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route};
use crate::portfolio::interest::{
    freshness_label, InterestMetric, DEFAULT_STALE_AFTER_DAYS, INTEREST_CONTRACT_VERSION,
    MAX_TREND_POINTS,
};
use crate::portfolio::interest_report::{interest_trend, project_interest, ProjectInterest};
use crate::registry::Registry;

use super::gateway::{cors, error, guarded, unavailable};

/// Match the three read-only portfolio interest routes before the main
/// table runs. The literals `interest/list`, `interest/show` and
/// `interest/trend` never collide with the `{kind}` arm in `router.rs`
/// (`GET /v1/admin/portfolio/{id}/{kind}` is a three-segment path;
/// these are four segments), so trying them first shadows nothing. Kept
/// beside the handlers (not in `router.rs`) so the route table stays
/// under the source-file-size cap.
pub(in crate::api) fn route_interest(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "portfolio", id, "interest", "list"]) => {
            Some(Route::AdminPortfolioInterestList {
                id: (*id).to_string(),
            })
        }
        ("GET", ["v1", "admin", "portfolio", id, "interest", "show"]) => {
            Some(Route::AdminPortfolioInterestShow {
                id: (*id).to_string(),
            })
        }
        ("GET", ["v1", "admin", "portfolio", id, "interest", "trend"]) => {
            Some(Route::AdminPortfolioInterestTrend {
                id: (*id).to_string(),
            })
        }
        _ => None,
    }
}

/// Project-level stale summary: `no-evidence` (no snapshots stored),
/// `stale` (at least one current window is older than the bound), else
/// `current`. Mirrors how `project_interest` derives `stale_snapshots`.
fn stale_label(measured: bool, stale_snapshots: usize) -> &'static str {
    if !measured {
        "no-evidence"
    } else if stale_snapshots > 0 {
        "stale"
    } else {
        "current"
    }
}

/// Project a `ProjectInterest` to JSON, injecting a per-snapshot
/// `freshness` label computed at read time so the browser never
/// re-derives staleness with divergent logic.
fn project_to_value(
    projection: &ProjectInterest,
    now: DateTime<Utc>,
    stale_after_days: i64,
) -> Value {
    let snapshots: Vec<Value> = projection
        .snapshots
        .iter()
        .map(|s| {
            let mut value = serde_json::to_value(s).unwrap_or(Value::Null);
            let fresh = freshness_label(&s.window_end, now, stale_after_days);
            value["freshness"] = json!(fresh.label());
            value
        })
        .collect();
    let findings: Vec<Value> = projection
        .findings
        .iter()
        .map(|f| {
            json!({
                "index": f.index,
                "project_id": f.project_id,
                "field": f.field,
                "code": f.code,
                "detail": f.detail,
            })
        })
        .collect();
    json!({
        "project_id": projection.project_id,
        "snapshots": snapshots,
        "stale_after_days": projection.stale_after_days,
        "stale_snapshots": projection.stale_snapshots,
        "superseded_snapshots": projection.superseded_snapshots,
        "findings": findings,
    })
}

/// Parse `stale_after_days` via the existing shared parser; a missing
/// value falls back to the default, an out-of-range value is the same
/// typed `400` the CLI path returns (wrapped in the CORS envelope).
fn parse_stale_days(config: &ApiConfig, request: &ApiRequest) -> Result<i64, ApiResponse> {
    match super::super::server::parse_interest_stale_after_days(
        request.query.as_deref().unwrap_or_default(),
    ) {
        Ok(value) => Ok(value),
        Err(response) => Err(cors(config, request, response)),
    }
}

/// `GET /v1/admin/portfolio/{id}/interest/list` — the per-project
/// snapshot list with per-window `stale` labels.
pub(in crate::api) fn interest_list(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    let stale_after_days = match parse_stale_days(config, request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let now = Utc::now();
            let registry = match Registry::open(db_path) {
                Ok(registry) => registry,
                Err(_) => return unavailable(),
            };
            match project_interest(&registry, id, stale_after_days, now) {
                Ok(projection) => {
                    let measured = !projection.snapshots.is_empty();
                    ApiResponse::json(
                        200,
                        json!({
                            "project_id": id,
                            "measured": measured,
                            "stale_label": stale_label(measured, projection.stale_snapshots),
                            "stale_after_days": projection.stale_after_days,
                            "stale_snapshots": projection.stale_snapshots,
                            "snapshots": project_to_value(&projection, now, stale_after_days)["snapshots"],
                            "contract": INTEREST_CONTRACT_VERSION,
                        }),
                    )
                }
                Err(err) => ApiResponse::from_error(&err),
            }
        }),
    )
}

/// `GET /v1/admin/portfolio/{id}/interest/show` — the whole projection
/// for one project, including its recorded refusals (`findings`).
pub(in crate::api) fn interest_show(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    let stale_after_days = match parse_stale_days(config, request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let now = Utc::now();
            let registry = match Registry::open(db_path) {
                Ok(registry) => registry,
                Err(_) => return unavailable(),
            };
            match project_interest(&registry, id, stale_after_days, now) {
                Ok(projection) => {
                    let measured = !projection.snapshots.is_empty();
                    ApiResponse::json(
                        200,
                        json!({
                            "project_id": id,
                            "measured": measured,
                            "stale_label": stale_label(measured, projection.stale_snapshots),
                            "stale_after_days": projection.stale_after_days,
                            "projection": project_to_value(&projection, now, stale_after_days),
                            "contract": INTEREST_CONTRACT_VERSION,
                        }),
                    )
                }
                Err(err) => ApiResponse::from_error(&err),
            }
        }),
    )
}

/// `GET /v1/admin/portfolio/{id}/interest/trend?metric=&limit=&stale_after_days=`
/// — one metric's windowed history; each point already carries its
/// `freshness`.
pub(in crate::api) fn handle_interest_trend(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let query = request.query.as_deref().unwrap_or_default();
            let mut metric_raw: Option<String> = None;
            let mut limit = 12usize;
            let mut stale_after_days = DEFAULT_STALE_AFTER_DAYS;
            for pair in query.split('&').filter(|part| !part.is_empty()) {
                let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                let value = super::super::percent_decode(value);
                match key.trim() {
                    "metric" => metric_raw = Some(value),
                    "limit" => match value.trim().parse::<usize>() {
                        Ok(parsed) => limit = parsed,
                        Err(_) => {
                            return error(
                                400,
                                "admin-invalid-interest-limit",
                                "the interest trend `limit` must be an integer",
                            )
                        }
                    },
                    "stale_after_days" => match value.trim().parse::<i64>() {
                        Ok(parsed) => stale_after_days = parsed,
                        Err(_) => {
                            return error(
                                400,
                                "admin-invalid-interest-stale",
                                "the interest trend `stale_after_days` must be an integer",
                            )
                        }
                    },
                    other => {
                        return error(
                            400,
                            "admin-invalid-interest-query",
                            &format!("unknown interest trend query parameter `{other}`"),
                        )
                    }
                }
            }
            let raw = match metric_raw {
                Some(metric) => metric,
                None => {
                    return error(
                        400,
                        "admin-invalid-interest-metric",
                        "interest trend requires a `metric` parameter",
                    )
                }
            };
            let metric = match InterestMetric::parse(raw.trim()) {
                Ok(metric) => metric,
                Err(reason) => return error(400, "admin-invalid-interest-metric", &reason),
            };
            if limit == 0 || limit > MAX_TREND_POINTS {
                return error(
                    400,
                    "admin-invalid-interest-limit",
                    &format!("the interest trend `limit` must be between 1 and {MAX_TREND_POINTS}"),
                );
            }
            let now = Utc::now();
            let registry = match Registry::open(db_path) {
                Ok(registry) => registry,
                Err(_) => return unavailable(),
            };
            match interest_trend(&registry, id, metric, limit, stale_after_days, now) {
                Ok(trend) => ApiResponse::json(
                    200,
                    json!({
                        "project_id": id,
                        "metric": trend.metric,
                        "stale_after_days": stale_after_days,
                        "interest": { "trend": trend },
                        "contract": INTEREST_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => ApiResponse::from_error(&err),
            }
        }),
    )
}
