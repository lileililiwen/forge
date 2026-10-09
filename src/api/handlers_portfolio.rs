//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::deploy::DeployRequest;
use crate::registry::Registry;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::contract::API_CONTRACT_VERSION;
use super::model::{ApiRequest, ApiResponse};
use super::router::bad_request;
use super::server::{parse_interest_stale_after_days, percent_decode, run_with_operation};

pub(super) fn handle_apply_deployment(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let confirm = body
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !confirm {
        return ApiResponse::json(
            409,
            serde_json::json!({
                "error": {
                    "code": "deploy-confirm-required",
                    "message": "deploy requires `confirm: true`; refusing implicit remote write"
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let dry_run = body
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_dir = match registry.inspect(id).map(|r| PathBuf::from(r.path)) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let (manifest, config) = match crate::deploy::engine::load_config(&project_dir) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let target = body
        .get("target_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| config.default_target.clone());
    let deploy_request = DeployRequest {
        project_id: id.to_string(),
        target,
        confirm: true,
        dry_run,
    };
    let (value, _op_id, _pid) =
        match run_with_operation(db_path, "api.deploy", id, request, |op_id, _registry| {
            let adapter = crate::deploy::DeployAdapterConfig::from_env();
            let report = if dry_run {
                let plan = crate::deploy::engine::prepare_deploy(
                    &project_dir,
                    &manifest,
                    &config,
                    &deploy_request,
                )?;
                serde_json::to_value(&plan).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?
            } else {
                let report = crate::deploy::engine::apply_deploy(
                    &project_dir,
                    &manifest,
                    &config,
                    &deploy_request,
                    &adapter,
                )?;
                serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?
            };
            Ok((report, op_id, id.to_string()))
        }) {
            Ok(value) => value,
            Err(response) => return response,
        };
    ApiResponse::json(
        202,
        serde_json::json!({
            "deploy": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

pub(super) fn handle_get_operation(db_path: &Path, op_id: i64) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.operation(op_id) {
        Ok(Some(entry)) => {
            ApiResponse::json(200, serde_json::to_value(&entry).unwrap_or(Value::Null))
        }
        Ok(None) => ApiResponse::json(
            404,
            serde_json::json!({
                "error": {
                    "code": "operation-not-found",
                    "message": format!("operation `{op_id}` is not recorded in the registry's journal")
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/projects/{id}/portfolio`
pub(super) fn handle_portfolio_project(
    db_path: &Path,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.portfolio_project_view(id, now) {
        Ok(view) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": view,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn required_field<'a>(body: &'a Value, field: &str) -> Result<&'a str, ApiResponse> {
    body.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| bad_request(&format!("portfolio request requires a `{field}` field")))
}

fn optional_field<'a>(body: &'a Value, field: &str) -> Option<&'a str> {
    body.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
}

/// `POST /v1/projects/{id}/portfolio/tags`
pub(super) fn handle_portfolio_tag(db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    let body = request.json_body();
    let name = match required_field(&body, "name") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let color = optional_field(&body, "color").map(|value| value.to_string());
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.portfolio_add_tag(id, &name, color.as_deref()) {
        Ok(tag) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": { "project_id": id, "tag": tag },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/portfolio/relations`
pub(super) fn handle_portfolio_relation(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    let body = request.json_body();
    let to = match required_field(&body, "to") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let raw_type = match required_field(&body, "type") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let relation_type = match crate::portfolio::RelationType::parse(&raw_type) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let note = optional_field(&body, "note").map(|value| value.to_string());
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.portfolio_add_relation(id, &to, relation_type, note.as_deref()) {
        Ok(relation) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": { "project_id": id, "relation": relation },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/portfolio/reviews`
pub(super) fn handle_portfolio_review(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    let body = request.json_body();
    let raw_confidence = match required_field(&body, "confidence") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let confidence = match crate::portfolio::Confidence::parse(&raw_confidence) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let lifecycle = match optional_field(&body, "lifecycle") {
        Some(raw) => match crate::portfolio::Lifecycle::parse(raw) {
            Ok(value) => Some(value),
            Err(reason) => return bad_request(&reason),
        },
        None => None,
    };
    let note = optional_field(&body, "note").map(|value| value.to_string());
    let next_action = optional_field(&body, "next_action").map(|value| value.to_string());
    let blocker = optional_field(&body, "blocker").map(|value| value.to_string());
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let write = crate::registry::PortfolioWrite {
        lifecycle,
        confidence: Some(confidence),
        next_action,
        blocker,
    };
    let outcome = registry.portfolio_write(id, &write).and_then(|profile| {
        registry
            .portfolio_record_review(id, confidence, note.as_deref())
            .map(|review| (profile, review))
    });
    match outcome {
        Ok((profile, review)) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": { "project_id": id, "profile": profile, "review": review },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/portfolio/evidence`
pub(super) fn handle_portfolio_evidence(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    let body = request.json_body();
    let source = match required_field(&body, "source") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let revision = match required_field(&body, "revision") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let raw_status = match required_field(&body, "status") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let status = match crate::portfolio::EvidenceStatus::parse(&raw_status) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let write = crate::registry::SnapshotWrite {
        source_system: source,
        source_revision: revision,
        observed_at: optional_field(&body, "observed_at")
            .map(|value| value.to_string())
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
        status,
        stale_after: optional_field(&body, "stale_after").map(|value| value.to_string()),
        evidence_json: body
            .get("evidence")
            .cloned()
            .filter(|value| !value.is_null())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "{}".to_string()),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.portfolio_import_snapshot(id, &write) {
        Ok(snapshot) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": { "project_id": id, "snapshot": snapshot },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/projects/{id}/share`
pub(super) fn handle_get_share(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    if let Err(err) = registry.inspect(id) {
        return ApiResponse::from_error(&err);
    }
    match registry.share_record(id) {
        // A project without a share record is private, not empty: the
        // projection says so rather than inventing a blank entry.
        Ok(None) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "project_id": id, "shared": false },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Ok(Some(record)) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "project_id": id, "shared": true, "record": record },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/share`
pub(super) fn handle_set_share(db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    use crate::portfolio::share::{ShareSurface, ShareWrite, ShowcaseStatus, Visibility};
    let body = request.json_body();
    let title = match required_field(&body, "title") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let summary = match required_field(&body, "summary") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let category = match required_field(&body, "category") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let source_url = match required_field(&body, "source_url") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let visibility = match optional_field(&body, "visibility") {
        Some(raw) => match Visibility::parse(raw) {
            Ok(value) => value,
            Err(reason) => return bad_request(&reason),
        },
        None => Visibility::Public,
    };
    let showcase_status = match optional_field(&body, "showcase_status") {
        Some(raw) => match ShowcaseStatus::parse(raw) {
            Ok(value) => value,
            Err(reason) => return bad_request(&reason),
        },
        None => ShowcaseStatus::Unknown,
    };
    let mut surfaces = Vec::new();
    if let Some(entries) = body.get("surfaces").and_then(|v| v.as_array()) {
        for entry in entries {
            let label = entry
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let url = entry
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            // Unvalidated on purpose: the registry's `validate_share`
            // is the single gate, so the refusal and its persisted
            // finding look the same whether they came from the CLI or
            // from this route.
            surfaces.push(ShareSurface::new(label, url));
        }
    }
    let write = ShareWrite {
        title,
        summary,
        category,
        source_url,
        demo_url: optional_field(&body, "demo_url").map(|value| value.to_string()),
        visibility,
        featured: body
            .get("featured")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        showcase_status,
        status_evidence: optional_field(&body, "status_evidence").map(|value| value.to_string()),
        surfaces,
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.share_upsert_record(id, &write) {
        Ok(record) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "project_id": id, "record": record },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/share/remove`
pub(super) fn handle_remove_share(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.share_remove_record(id) {
        Ok(removed) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": {
                    "project_id": id,
                    "shared": false,
                    "removed": removed,
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/share/manifest`
pub(super) fn handle_share_preview(db_path: &Path, now: DateTime<Utc>) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::portfolio::publication::preview_manifest(&registry) {
        Ok(draft) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": {
                    "manifest_revision": draft.body.manifest_revision,
                    "manifest_sha256": draft.manifest_sha256(),
                    "project_count": draft.project_count(),
                    "approvable": draft.approvable(),
                    "canonical_body": draft.body.canonical_json(),
                    "findings": draft.findings,
                    "generated_at": now.to_rfc3339(),
                },
                "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/share/approve`
pub(super) fn handle_share_approve(
    db_path: &Path,
    request: &ApiRequest,
    actor: &str,
) -> ApiResponse {
    let body = request.json_body();
    let manifest_sha256 = match required_field(&body, "manifest_sha256") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.share_approve(&manifest_sha256, actor) {
        Ok(approval) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "approval": approval },
                "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/share/publish`
pub(super) fn handle_share_publish(
    db_path: &Path,
    request: &ApiRequest,
    actor: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let target = match required_field(&body, "target") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let operation_key = match required_field(&body, "operation_key") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let adapter = optional_field(&body, "adapter").map(PathBuf::from);
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let plan = crate::portfolio::publication::PublishPlan {
        operation_key,
        target,
        actor: actor.to_string(),
        adapter,
    };
    match crate::portfolio::publication::publish_approved_manifest(&registry, &plan, now) {
        Ok(report) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "publication": report },
                "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/share/reconcile`
pub(super) fn handle_share_reconcile(
    db_path: &Path,
    request: &ApiRequest,
    actor: &str,
) -> ApiResponse {
    use crate::portfolio::share::PublicationStatus;
    let body = request.json_body();
    let raw_id = match required_field(&body, "publication_id") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let publication_id = match raw_id.parse::<i64>() {
        Ok(value) => value,
        Err(_) => return bad_request("publication_id must be an integer"),
    };
    let status = match required_field(&body, "status") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let status = match PublicationStatus::parse(&status) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.share_reconcile_publication(publication_id, status, actor) {
        Ok(attempt) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "publication": attempt },
                "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/share/audit`
pub(super) fn handle_share_audit(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let limit = match parse_share_limit(request.query.as_deref().unwrap_or_default()) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let approvals = match registry.share_approvals(limit) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let publications = match registry.share_publications(limit) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let unreconciled = match registry.share_unreconciled_publication() {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    ApiResponse::json(
        200,
        serde_json::json!({
            "share": {
                "approvals": approvals,
                "publications": publications,
                "unreconciled": unreconciled,
            },
            "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
        }),
    )
}

/// Parse the `limit` query parameter of the share audit route. An
/// absent parameter is the house default; a present but unusable one
/// is a typed refusal rather than a silently widened list.
fn parse_share_limit(query: &str) -> Result<usize, String> {
    let Some(pair) = query.split('&').find(|part| !part.is_empty()) else {
        return Ok(50);
    };
    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
    if key.trim() != "limit" {
        return Err(format!("unknown share audit query parameter `{key}`"));
    }
    let parsed = value
        .trim()
        .parse::<usize>()
        .map_err(|_| "limit must be an integer".to_string())?;
    if !(1..=500).contains(&parsed) {
        return Err("limit must be between 1 and 500".to_string());
    }
    Ok(parsed)
}

/// `GET /v1/projects/{id}/interest`
pub(super) fn handle_get_interest(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let query = request.query.as_deref().unwrap_or_default();
    let stale_after_days = match parse_interest_stale_after_days(query) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::portfolio::interest_report::project_interest(&registry, id, stale_after_days, now)
    {
        Ok(projection) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": {
                    "project_id": id,
                    "measured": !projection.snapshots.is_empty(),
                    "projection": projection,
                },
                "contract": crate::portfolio::interest::INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/interest`
pub(super) fn handle_import_interest(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    actor: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    use crate::portfolio::interest::{RawSnapshot, INTEREST_CONTRACT_VERSION};
    let body = request.json_body();
    // The body carries the same array an importer hands the CLI, with
    // the path supplying the project: a body that names a different
    // project is refused rather than silently re-scoped.
    let Some(entries) = body.get("snapshots").and_then(|v| v.as_array()) else {
        return bad_request("interest import requires a `snapshots` array");
    };
    if request.body.len() > crate::portfolio::interest::MAX_IMPORT_BYTES {
        return bad_request(&format!(
            "import document is larger than {} bytes",
            crate::portfolio::interest::MAX_IMPORT_BYTES
        ));
    }
    let mut records: Vec<RawSnapshot> = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let Value::Object(record) = entry else {
            return bad_request(&format!(
                "snapshot {index} must be a JSON object; an aggregate record carries no free-form value"
            ));
        };
        match record.get("project_id").and_then(|v| v.as_str()) {
            Some(declared) if declared.trim() != id => {
                return bad_request(&format!(
                    "snapshot {index} declares project_id `{declared}` but the route targets `{id}`"
                ))
            }
            _ => {}
        }
        // Unvalidated on purpose: the registry's `validate_snapshot`
        // is the single gate, so a refusal looks the same whether the
        // record arrived over HTTP or from a file. The route supplies
        // the project so a caller cannot import one project's evidence
        // under another's name by accident.
        let mut record = record.clone();
        record.insert("project_id".to_string(), Value::String(id.to_string()));
        records.push(RawSnapshot { index, record });
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    // The importer is the authenticated session subject, never a
    // client-claimed string: provenance is the point of the store.
    match crate::portfolio::interest_report::import_snapshots(
        &registry,
        &crate::portfolio::interest::InterestImport { records },
        actor,
        now,
    ) {
        Ok(imported) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": {
                    "project_id": id,
                    "import": imported,
                },
                "contract": INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/interest/compare`
pub(super) fn handle_interest_compare(
    db_path: &Path,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    use crate::portfolio::interest::{InterestMetric, INTEREST_CONTRACT_VERSION};
    let query = request.query.as_deref().unwrap_or_default().to_string();
    let mut projects: Vec<String> = Vec::new();
    let mut metrics: Vec<String> = Vec::new();
    let mut source: Option<String> = None;
    let mut stale_after_days = crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS;
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key.trim() {
            "projects" => projects.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string),
            ),
            "metric" => metrics.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string),
            ),
            "source" => source = Some(value),
            "stale_after_days" => match value.trim().parse::<i64>() {
                Ok(parsed) => stale_after_days = parsed,
                Err(_) => return bad_request("stale_after_days must be an integer"),
            },
            other => {
                return bad_request(&format!(
                    "unknown interest compare query parameter `{other}`"
                ))
            }
        }
    }
    if projects.is_empty() {
        return bad_request("interest compare requires a `projects` parameter");
    }
    let selected: Vec<InterestMetric> = if metrics.is_empty() {
        InterestMetric::ALL.to_vec()
    } else {
        let mut chosen = Vec::with_capacity(metrics.len());
        for raw in &metrics {
            match InterestMetric::parse(raw) {
                Ok(metric) if !chosen.contains(&metric) => chosen.push(metric),
                Ok(_) => {}
                Err(reason) => return bad_request(&reason),
            }
        }
        chosen
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::portfolio::interest_report::compare_projects(
        &registry,
        &projects,
        &selected,
        source.as_deref(),
        stale_after_days,
        now,
    ) {
        Ok(comparison) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": { "comparison": comparison },
                "contract": INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/interest/trend`
pub(super) fn handle_interest_trend(
    db_path: &Path,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    use crate::portfolio::interest::{InterestMetric, INTEREST_CONTRACT_VERSION};
    let query = request.query.as_deref().unwrap_or_default().to_string();
    let mut project: Option<String> = None;
    let mut metric: Option<String> = None;
    let mut limit = 12usize;
    let mut stale_after_days = crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS;
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key.trim() {
            "project" => project = Some(value),
            "metric" => metric = Some(value),
            "limit" => match value.trim().parse::<usize>() {
                Ok(parsed) => limit = parsed,
                Err(_) => return bad_request("limit must be an integer"),
            },
            "stale_after_days" => match value.trim().parse::<i64>() {
                Ok(parsed) => stale_after_days = parsed,
                Err(_) => return bad_request("stale_after_days must be an integer"),
            },
            other => {
                return bad_request(&format!("unknown interest trend query parameter `{other}`"))
            }
        }
    }
    let (Some(project), Some(raw_metric)) = (project, metric) else {
        return bad_request("interest trend requires `project` and `metric` parameters");
    };
    let metric = match InterestMetric::parse(raw_metric.trim()) {
        Ok(metric) => metric,
        Err(reason) => return bad_request(&reason),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::portfolio::interest_report::interest_trend(
        &registry,
        project.trim(),
        metric,
        limit,
        stale_after_days,
        now,
    ) {
        Ok(trend) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": { "trend": trend },
                "contract": INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}
