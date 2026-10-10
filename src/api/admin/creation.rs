//! Read-only creation-catalog browser (`web-creation-catalog-browser`).
//!
//! Twenty session-gated GET handlers over the pure Core creation
//! catalogs the CLI already reads: `profile::{list_profiles,
//! inspect_profile, resolve_profile}`, `feature::{feature_catalog,
//! inspect_feature, resolve_plan}`, `component::{component_catalog,
//! inspect_component, resolve_outcome}`,
//! `ui_pattern::{ui_pattern_catalog, inspect_ui_pattern,
//! resolve_outcome}`, `standard::{all_packs, inspect_pack}`,
//! `procedure::{procedure_catalog, inspect_procedure}`,
//! `planner::{validate_intent, intent_hash}`, plus the two
//! project-bound snapshot reads (`standard::{check_snapshot,
//! diff_snapshot}`) and the persisted plan-receipt list
//! (`.forge/planner/`). No write, no provider, no adapter, no native
//! toolchain probe, no shell, no journal row, no browser-supplied
//! path on any path; every response carries the admin contract
//! version and project-bound bodies are scrubbed of the absolute
//! project directory. The CLI's `intent validate` journal line is
//! deliberately not reproduced, and plan receipts carry plan ids
//! only (never the CLI's absolute receipt path).

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use serde_json::{json, Value};
use std::path::Path;

use super::deploy::deploy_id_gate;
use super::gateway::{cors, error, guarded, scrub_json, scrub_text, unavailable};

/// Catalog keys served by the list/inspect/resolve arms. Anything
/// else never matches (the matcher returns `None`, so the router
/// answers `route-not-found`); a key is a validated token, never a
/// path the server would open.
const LISTABLE: &[&str] = &[
    "profiles",
    "features",
    "components",
    "ui-patterns",
    "standards",
    "procedures",
];

/// Registry keys with a query-driven resolve arm.
const RESOLVABLE: &[&str] = &["profiles", "features", "components", "ui-patterns"];

fn list_route(registry: &str) -> Route {
    Route::AdminCreation {
        registry: registry.to_string(),
        item: String::new(),
        action: "list".to_string(),
    }
}

fn inspect_route(registry: &str, id: &str) -> Route {
    Route::AdminCreation {
        registry: registry.to_string(),
        item: id.to_string(),
        action: "inspect".to_string(),
    }
}

fn resolve_route(registry: &str) -> Route {
    Route::AdminCreation {
        registry: registry.to_string(),
        item: String::new(),
        action: "resolve".to_string(),
    }
}

/// Match the twenty read-only creation-catalog routes before the main
/// table runs. The literal `creation` segment never collides with the
/// `projects`/`portfolio`/`delivery`/`fleet`/`workspace`/`graduation`
/// arms; the literal `resolve` arm precedes the `{id}` arm per
/// registry so `resolve` never reads as an entry id; the
/// `projects/{id}/standard/*` and `projects/{id}/intent/plans`
/// literals never collide with the existing `feature`/`spec`/
/// `deploy`/`release`/`publish`/`delivery`/`agents`/`identity`/
/// `releases`/`deploys`/`catalog`/`status`/`plan`/`maintain` arms, so
/// trying these shapes first shadows no existing route. Kept beside
/// the handlers (not in `router.rs`) so the route table stays under
/// the source-file-size cap. The `OPTIONS` arm keeps the browser
/// preflight beside the GETs it covers.
pub(in crate::api) fn route_creation(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "creation", "profiles"]) => Some(list_route("profiles")),
        ("GET", ["v1", "admin", "creation", "profiles", "resolve"]) => {
            Some(resolve_route("profiles"))
        }
        ("GET", ["v1", "admin", "creation", "profiles", id]) => Some(inspect_route("profiles", id)),
        ("GET", ["v1", "admin", "creation", "features"]) => Some(list_route("features")),
        ("GET", ["v1", "admin", "creation", "features", "resolve"]) => {
            Some(resolve_route("features"))
        }
        ("GET", ["v1", "admin", "creation", "features", id]) => Some(inspect_route("features", id)),
        ("GET", ["v1", "admin", "creation", "components"]) => Some(list_route("components")),
        ("GET", ["v1", "admin", "creation", "components", "resolve"]) => {
            Some(resolve_route("components"))
        }
        ("GET", ["v1", "admin", "creation", "components", id]) => {
            Some(inspect_route("components", id))
        }
        ("GET", ["v1", "admin", "creation", "ui-patterns"]) => Some(list_route("ui-patterns")),
        ("GET", ["v1", "admin", "creation", "ui-patterns", "resolve"]) => {
            Some(resolve_route("ui-patterns"))
        }
        ("GET", ["v1", "admin", "creation", "ui-patterns", id]) => {
            Some(inspect_route("ui-patterns", id))
        }
        ("GET", ["v1", "admin", "creation", "standards"]) => Some(list_route("standards")),
        ("GET", ["v1", "admin", "creation", "standards", id]) => {
            Some(inspect_route("standards", id))
        }
        ("GET", ["v1", "admin", "creation", "procedures"]) => Some(list_route("procedures")),
        ("GET", ["v1", "admin", "creation", "procedures", id]) => {
            Some(inspect_route("procedures", id))
        }
        ("GET", ["v1", "admin", "creation", "intents", "validate"]) => Some(Route::AdminCreation {
            registry: "intents".to_string(),
            item: String::new(),
            action: "validate".to_string(),
        }),
        ("GET", ["v1", "admin", "projects", id, "standard", "check"]) => {
            Some(Route::AdminCreation {
                registry: "project-standard".to_string(),
                item: (*id).to_string(),
                action: "check".to_string(),
            })
        }
        ("GET", ["v1", "admin", "projects", id, "standard", "diff"]) => {
            Some(Route::AdminCreation {
                registry: "project-standard".to_string(),
                item: (*id).to_string(),
                action: "diff".to_string(),
            })
        }
        ("GET", ["v1", "admin", "projects", id, "intent", "plans"]) => Some(Route::AdminCreation {
            registry: "project-intent".to_string(),
            item: (*id).to_string(),
            action: "plans".to_string(),
        }),
        ("OPTIONS", ["v1", "admin", "creation", ..]) => Some(Route::AdminOptions),
        _ => None,
    }
}

/// Validate one `{id}` path segment before any catalog lookup. A
/// blank value, a path separator (raw or percent-encoded), a
/// backslash, a `..` traversal or any other percent-encoding is a
/// static typed `400` that never echoes the offending input.
/// Catalog ids (including `<pack>@<version>` selectors) never
/// contain those bytes.
fn creation_id_gate(raw: &str) -> Result<String, ApiResponse> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains('%')
    {
        return Err(error(
            400,
            "admin-invalid-creation-id",
            "that catalog id is not valid; use the id shown in the catalog list.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Collect the repeatable `?key=` query values in order,
/// percent-decoded like every other browser query value. Blank
/// values are dropped so `?feature=&feature=auth` means `auth`.
fn query_values(query: Option<&str>, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for pair in query
        .unwrap_or_default()
        .split('&')
        .filter(|p| !p.is_empty())
    {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k.trim() == key {
            let decoded = super::super::percent_decode(v);
            if !decoded.trim().is_empty() {
                out.push(decoded);
            }
        }
    }
    out
}

/// The single `?key=` query value, if any.
fn query_single(query: Option<&str>, key: &str) -> Option<String> {
    query_values(query, key).into_iter().next()
}

/// Require one `?key=` query value. Absent or blank is a static
/// typed `400` that never echoes the query string.
fn require_query(query: Option<&str>, key: &str, what: &str) -> Result<String, ApiResponse> {
    match query_single(query, key) {
        Some(value) => Ok(value),
        None => Err(error(400, "admin-invalid-creation-query", what)),
    }
}

/// Creation-catalog status mapping: the shared table plus the six
/// reachable creation user errors, which the shared table would
/// report as `500`. Unknown entries, planned-profile resolves and
/// malformed snapshot selectors are operator input errors, so they
/// answer `400` with the typed Core code. Kept beside the handlers
/// (not in `err_status`) so `router.rs` stays under the
/// source-file-size cap.
fn creation_error_status(err: &crate::core::ForgeError) -> u16 {
    match err.code() {
        "unsupported-profile"
        | "component-invalid"
        | "ui-pattern-invalid"
        | "ui-pattern-unsupported-platform"
        | "standard-invalid"
        | "procedure-invalid" => 400,
        _ => super::super::err_status(err),
    }
}

/// Render a Core failure with the creation status mapping. The typed
/// code is preserved so a boundary scenario is reported honestly;
/// pure-catalog errors name only catalog keys, never paths.
fn typed_creation_error(err: &crate::core::ForgeError) -> ApiResponse {
    ApiResponse::json(
        creation_error_status(err),
        json!({
            "error": {
                "code": err.code(),
                "message": err.to_string(),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// Render a project-bound Core failure, scrubbing the
/// server-resolved project directory from the message.
fn typed_project_creation_error(project_dir: &Path, err: &crate::core::ForgeError) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    let status = creation_error_status(err);
    ApiResponse::json(
        status,
        json!({
            "error": {
                "code": err.code(),
                "message": scrub_text(&err.to_string(), &secrets),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// Dispatch one creation-catalog read. The `(registry, action)` pair
/// is re-validated here so only the matcher's twenty combos run;
/// anything else is a static `404`, never a guess.
pub(in crate::api) fn dispatch(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    registry: &str,
    item: &str,
    action: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let query = req.query.as_deref();
            match (registry, action) {
                (r, "list") if LISTABLE.contains(&r) => creation_list(r),
                (r, "inspect") if LISTABLE.contains(&r) => match creation_id_gate(item) {
                    Ok(id) => creation_inspect(r, &id),
                    Err(response) => response,
                },
                (r, "resolve") if RESOLVABLE.contains(&r) => creation_resolve(r, query),
                ("intents", "validate") => intent_validate(query),
                ("project-standard", "check") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => standard_check(&dir, item),
                    Err(response) => response,
                },
                ("project-standard", "diff") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => match require_query(
                        query,
                        "against",
                        "the standard `against` pack selector is required, e.g. ?against=baseline-service@1.1.0.",
                    ) {
                        Ok(against) => standard_diff(&dir, item, &against),
                        Err(response) => response,
                    },
                    Err(response) => response,
                },
                ("project-intent", "plans") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => intent_plans(&dir, item),
                    Err(response) => response,
                },
                _ => error(
                    404,
                    "admin-unknown-creation",
                    "no creation-catalog route matches the request",
                ),
            }
        }),
    )
}

/// `GET /v1/admin/creation/{registry}`: the full catalog in stable
/// order — the same entries the CLI `list` renders.
fn creation_list(registry: &str) -> ApiResponse {
    let (entries, contract): (Value, Option<&str>) = match registry {
        "profiles" => (json!(crate::profile::list_profiles()), None),
        "features" => (
            json!(crate::feature::feature_catalog()),
            Some(crate::feature::FEATURE_CATALOG_VERSION),
        ),
        "components" => (
            json!(crate::component::component_catalog()),
            Some(crate::component::COMPONENT_CATALOG_VERSION),
        ),
        "ui-patterns" => (
            json!(crate::ui_pattern::ui_pattern_catalog()),
            Some(crate::ui_pattern::UI_PATTERN_CATALOG_VERSION),
        ),
        "standards" => (json!(crate::standard::all_packs()), None),
        "procedures" => (
            json!(crate::procedure::procedure_catalog()),
            Some(crate::procedure::PROCEDURE_CONTRACT_VERSION),
        ),
        _ => {
            return error(
                404,
                "admin-unknown-creation",
                "no creation-catalog route matches the request",
            )
        }
    };
    let mut creation = json!({
        "registry": registry,
        "entries": entries,
    });
    if let Some(contract) = contract {
        creation["contract"] = json!(contract);
    }
    ApiResponse::json(
        200,
        json!({
            "creation": creation,
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// `GET /v1/admin/creation/{registry}/{id}`: one catalog entry —
/// the same descriptor the CLI `inspect` renders.
fn creation_inspect(registry: &str, id: &str) -> ApiResponse {
    let (entry, contract): (Result<Value, crate::core::ForgeError>, Option<&str>) = match registry {
        "profiles" => (crate::profile::inspect_profile(id).map(|d| json!(d)), None),
        "features" => (
            crate::feature::inspect_feature(id).map(|d| json!(d)),
            Some(crate::feature::FEATURE_CATALOG_VERSION),
        ),
        "components" => (
            crate::component::inspect_component(id).map(|d| json!(d)),
            Some(crate::component::COMPONENT_CATALOG_VERSION),
        ),
        "ui-patterns" => (
            crate::ui_pattern::inspect_ui_pattern(id).map(|d| json!(d)),
            Some(crate::ui_pattern::UI_PATTERN_CATALOG_VERSION),
        ),
        "standards" => (crate::standard::inspect_pack(id).map(|d| json!(d)), None),
        "procedures" => (
            crate::procedure::inspect_procedure(id).map(|d| json!(d)),
            Some(crate::procedure::PROCEDURE_CONTRACT_VERSION),
        ),
        _ => {
            return error(
                404,
                "admin-unknown-creation",
                "no creation-catalog route matches the request",
            )
        }
    };
    match entry {
        Ok(entry) => {
            let mut creation = json!({
                "registry": registry,
                "id": id,
                "entry": entry,
            });
            if let Some(contract) = contract {
                creation["contract"] = json!(contract);
            }
            ApiResponse::json(
                200,
                json!({
                    "creation": creation,
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
        Err(err) => typed_creation_error(&err),
    }
}

/// `GET /v1/admin/creation/{registry}/resolve`: the deterministic
/// resolve outcome for the query-supplied subject — the same payload
/// the CLI `resolve` renders, with rejections kept reviewable.
/// Query keys mirror the CLI flags exactly: profile resolve takes
/// `?id=` + repeatable `?feature=`; feature resolve takes
/// `?profile=` + repeatable `?feature=`; component resolve takes
/// `?profile=` + repeatable `?component=`; ui-pattern resolve takes
/// `?profile=` + repeatable `?pattern=`.
fn creation_resolve(registry: &str, query: Option<&str>) -> ApiResponse {
    match registry {
        "profiles" => {
            let id = match require_query(
                query,
                "id",
                "the profile `id` is required, e.g. ?id=rust-web&feature=auth.",
            ) {
                Ok(id) => id,
                Err(response) => return response,
            };
            if let Err(response) = creation_id_gate(&id) {
                return response;
            }
            let features = query_values(query, "feature");
            match crate::profile::resolve_profile(&id, &features) {
                Ok(resolution) => ApiResponse::json(
                    200,
                    json!({
                        "creation": {
                            "registry": registry,
                            "id": id,
                            "requested": features,
                            "resolution": resolution,
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => typed_creation_error(&err),
            }
        }
        "features" => {
            let profile = match require_query(
                query,
                "profile",
                "the resolve `profile` is required, e.g. ?profile=rust-web&feature=auth.",
            ) {
                Ok(profile) => profile,
                Err(response) => return response,
            };
            let features = query_values(query, "feature");
            if features.is_empty() {
                return error(
                    400,
                    "admin-invalid-creation-query",
                    "at least one `feature` is required, e.g. ?profile=rust-web&feature=auth.",
                );
            }
            match crate::feature::resolve_plan(&profile, &features) {
                Ok(plan) => ApiResponse::json(
                    200,
                    json!({
                        "creation": {
                            "registry": registry,
                            "profile": profile,
                            "requested": features,
                            "contract": crate::feature::FEATURE_CATALOG_VERSION,
                            "resolution": plan,
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => typed_creation_error(&err),
            }
        }
        "components" => {
            let profile = match require_query(
                query,
                "profile",
                "the resolve `profile` is required, e.g. ?profile=rust-web&component=audit-action.",
            ) {
                Ok(profile) => profile,
                Err(response) => return response,
            };
            let components = query_values(query, "component");
            if components.is_empty() {
                return error(
                    400,
                    "admin-invalid-creation-query",
                    "at least one `component` is required, e.g. ?profile=rust-web&component=audit-action.",
                );
            }
            let request = crate::component::ComponentRequest {
                profile: profile.clone(),
                component_ids: components.clone(),
            };
            match crate::component::resolve_outcome(&request) {
                Ok(outcome) => ApiResponse::json(
                    200,
                    json!({
                        "creation": {
                            "registry": registry,
                            "profile": profile,
                            "requested": components,
                            "contract": crate::component::COMPONENT_CATALOG_VERSION,
                            "resolution": outcome,
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => typed_creation_error(&err),
            }
        }
        "ui-patterns" => {
            let profile = match require_query(
                query,
                "profile",
                "the resolve `profile` is required, e.g. ?profile=react-web&pattern=login.",
            ) {
                Ok(profile) => profile,
                Err(response) => return response,
            };
            let patterns = query_values(query, "pattern");
            if patterns.is_empty() {
                return error(
                    400,
                    "admin-invalid-creation-query",
                    "at least one `pattern` is required, e.g. ?profile=react-web&pattern=login.",
                );
            }
            let request = crate::ui_pattern::UiPatternRequest {
                profile: profile.clone(),
                pattern_ids: patterns.clone(),
            };
            match crate::ui_pattern::resolve_outcome(&request) {
                Ok(outcome) => ApiResponse::json(
                    200,
                    json!({
                        "creation": {
                            "registry": registry,
                            "profile": profile,
                            "requested": patterns,
                            "contract": crate::ui_pattern::UI_PATTERN_CATALOG_VERSION,
                            "resolution": outcome,
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => typed_creation_error(&err),
            }
        }
        _ => error(
            404,
            "admin-unknown-creation",
            "no creation-catalog route matches the request",
        ),
    }
}

/// Parse one `create_project` / `extend_project` action value with
/// the CLI's rule and message shape.
fn parse_action(value: &str) -> Result<crate::planner::IntentAction, crate::core::ForgeError> {
    match value {
        "create_project" => Ok(crate::planner::IntentAction::CreateProject),
        "extend_project" => Ok(crate::planner::IntentAction::ExtendProject),
        _ => Err(crate::core::ForgeError::IntentInvalid {
            reason: format!(
                "unknown intent action '{value}'; accepted actions: create_project, extend_project"
            ),
        }),
    }
}

/// Parse repeatable `key=value` constraints with the CLI's rule and
/// message shape.
fn parse_constraints(
    raw: &[String],
) -> Result<Vec<crate::planner::IntentConstraint>, crate::core::ForgeError> {
    let mut out = Vec::new();
    for entry in raw {
        let (key, value) =
            entry
                .split_once('=')
                .ok_or_else(|| crate::core::ForgeError::IntentInvalid {
                    reason: format!(
                        "constraint '{entry}' is not in 'key=value' form; the planner refuses a \
                         malformed constraint"
                    ),
                })?;
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            return Err(crate::core::ForgeError::IntentInvalid {
                reason: format!(
                    "constraint '{entry}' has an empty key or value; the planner refuses a \
                     malformed constraint"
                ),
            });
        }
        out.push(crate::planner::IntentConstraint {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(out)
}

/// `GET /v1/admin/creation/intents/validate`: validate a structured
/// intent without resolving or applying it — the same verdict the
/// CLI `intent validate` renders, minus its journal line (reads
/// never call `record_operation`). Query keys mirror the CLI flags:
/// `?action=` + `?profile=` + repeatable `require`/`forbid`/
/// `constraint`.
fn intent_validate(query: Option<&str>) -> ApiResponse {
    let action_raw = match require_query(
        query,
        "action",
        "the intent `action` is required: create_project or extend_project.",
    ) {
        Ok(action) => action,
        Err(response) => return response,
    };
    let profile = match require_query(query, "profile", "the intent `profile` is required.") {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let action = match parse_action(action_raw.trim()) {
        Ok(action) => action,
        Err(err) => return typed_creation_error(&err),
    };
    let constraints = match parse_constraints(&query_values(query, "constraint")) {
        Ok(constraints) => constraints,
        Err(err) => return typed_creation_error(&err),
    };
    let intent = crate::planner::Intent {
        action,
        profile: profile.clone(),
        required_capabilities: query_values(query, "require"),
        forbidden_capabilities: query_values(query, "forbid"),
        constraints,
    };
    match crate::planner::validate_intent(&intent) {
        Ok(validated) => {
            let hash = crate::planner::intent_hash(&validated.intent);
            ApiResponse::json(
                200,
                json!({
                    "creation": {
                        "registry": "intents",
                        "contract": crate::planner::PLANNER_CONTRACT_VERSION,
                        "validated": validated,
                        "intent_hash": hash,
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
        Err(err) => typed_creation_error(&err),
    }
}

/// `GET /v1/admin/projects/{id}/standard/check`: verify the
/// project's `.standard/` snapshot against its ownership receipt.
/// The directory is server-resolved; the report body is scrubbed so
/// no absolute path survives.
fn standard_check(project_dir: &Path, id: &str) -> ApiResponse {
    match crate::standard::check_snapshot(project_dir) {
        Ok(report) => {
            let secrets = [project_dir.display().to_string()];
            let report = scrub_json(json!(report), &secrets);
            ApiResponse::json(
                200,
                json!({
                    "creation": {
                        "registry": "standards",
                        "project_id": id,
                        "report": report,
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
        Err(err) => typed_project_creation_error(project_dir, &err),
    }
}

/// `GET /v1/admin/projects/{id}/standard/diff?against=`: show what
/// an upgrade to a pack version would change (read-only). Same
/// server-side resolution and scrubbing as `check`.
fn standard_diff(project_dir: &Path, id: &str, against: &str) -> ApiResponse {
    if let Err(response) = creation_id_gate(against) {
        return response;
    }
    match crate::standard::diff_snapshot(project_dir, against) {
        Ok(report) => {
            let secrets = [project_dir.display().to_string()];
            let report = scrub_json(json!(report), &secrets);
            ApiResponse::json(
                200,
                json!({
                    "creation": {
                        "registry": "standards",
                        "project_id": id,
                        "against": against,
                        "report": report,
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
        Err(err) => typed_project_creation_error(project_dir, &err),
    }
}

/// `GET /v1/admin/projects/{id}/intent/plans`: the persisted plan
/// receipts under the project's `.forge/planner/` — plan ids only,
/// never a filesystem path. An absent directory is an empty catalog,
/// not an error; an unreadable one is an honest `503`.
fn intent_plans(project_dir: &Path, id: &str) -> ApiResponse {
    let dir = crate::planner::plans_dir(project_dir);
    if !dir.exists() {
        return ApiResponse::json(
            200,
            json!({
                "creation": {
                    "registry": "intents",
                    "contract": crate::planner::PLANNER_CONTRACT_VERSION,
                    "project_id": id,
                    "plans": [],
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let read = match std::fs::read_dir(&dir) {
        Ok(read) => read,
        Err(_) => return unavailable(),
    };
    let mut plans: Vec<Value> = Vec::new();
    for entry in read.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            plans.push(json!({
                "plan_id": entry.file_name().to_string_lossy(),
            }));
        }
    }
    plans.sort_by(|a, b| a["plan_id"].as_str().cmp(&b["plan_id"].as_str()));
    ApiResponse::json(
        200,
        json!({
            "creation": {
                "registry": "intents",
                "contract": crate::planner::PLANNER_CONTRACT_VERSION,
                "project_id": id,
                "plans": plans,
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}
