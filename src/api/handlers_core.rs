//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::catalog;
use crate::core::ForgeError;
use crate::publish::github::{verify_push, GitHubPushEvent};
use crate::publish::providers::{
    invoke_provider, load_config as load_publish_provider_config, select_provider,
    ProviderOperation, PublishProviderRequest, PUBLISH_PROVIDER_CONTRACT,
};
use crate::registry::{Registry, ReservationOutcome};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use super::contract::API_CONTRACT_VERSION;
use super::model::{ApiConfig, ApiRequest, ApiResponse, Route};
use super::router::{bad_request, required_permission};
use super::server::{
    build_catalog_selection, catalog_filter_pairs_from, parse_catalog_query_params,
};

/// Authorization step. Returns `Ok(actor)` when the caller
/// is permitted to issue the request, where `actor` is the
/// authenticated session subject recorded in the audit
/// trail; returns `Err(response)` with the rendered error
/// response otherwise. The session is loaded through the
/// identity surface so a token minted for project A cannot
/// authorize project B.
pub(super) fn authorize(
    config: &ApiConfig,
    db_path: &Path,
    route: &Route,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> Result<String, ApiResponse> {
    if matches!(route, Route::Healthz | Route::GitHubPush) {
        return Ok(String::new());
    }
    // The browser fleet filter row calls the read-only catalog query with
    // its administrator session cookie: the cookie is HttpOnly, so browser
    // JS cannot present it as an Authorization Bearer header. A valid
    // administrator session already reads the richer
    // `GET /v1/admin/projects` fleet, so accepting it here grants nothing
    // new; without any session the route still 401s. Bearer callers are
    // unaffected: a present Bearer token always takes the bearer path
    // below, and a hostile origin is still refused on the cookie path.
    if matches!(route, Route::CatalogQuery { .. }) && request.bearer_token.is_none() {
        if !super::admin::allowed_origin(config, request.header("origin")) {
            return Err(super::admin::error(
                403,
                "admin-origin-rejected",
                "request origin is not allowed",
            ));
        }
        let cookie = request
            .cookies
            .get("forge_admin_session")
            .map(String::as_str)
            .unwrap_or("");
        return match crate::identity::global::session_valid(db_path, cookie) {
            Ok(true) => Ok(String::new()),
            _ => Err(ApiResponse::json(
                401,
                serde_json::json!({
                    "error": {
                        "code": "api-unauthorized",
                        "message": "administrator session is required"
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            )),
        };
    }
    let token = request.bearer_token.as_deref().ok_or_else(|| {
        ApiResponse::json(
            401,
            serde_json::json!({
                "error": {
                    "code": "api-unauthorized",
                    "message": "missing Authorization: Bearer <session-id> header"
                },
                "contract": API_CONTRACT_VERSION,
            }),
        )
    })?;
    if token.is_empty() || !is_hex(token) {
        return Err(ApiResponse::json(
            401,
            serde_json::json!({
                "error": {
                    "code": "api-unauthorized",
                    "message": "bearer token must be a non-empty hex session id"
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ));
    }
    // The match is exhaustive over every route that
    // requires authorization. Healthz is already handled
    // by the early return above.
    let result: Result<String, ApiResponse> = match route {
        Route::Healthz => Ok(String::new()),
        Route::GitHubPush => Ok(String::new()),
        Route::AdminSessionGet
        | Route::AdminSessionPost
        | Route::AdminSessionDelete
        | Route::AdminProjects
        | Route::AdminCommands
        | Route::AdminFleetStatus
        | Route::AdminProjectDetail { .. }
        | Route::AdminProjectStatus { .. }
        | Route::AdminProjectMaintain { .. }
        | Route::AdminProjectClassifyApprove { .. }
        | Route::AdminProjectClassifyReject { .. }
        | Route::AdminProjectClassifyApply { .. }
        | Route::AdminProjectPlan { .. }
        | Route::AdminProjectApply { .. }
        | Route::AdminProjectFeature { .. }
        | Route::AdminProjectSpec { .. }
        | Route::AdminProjectFeatureRemove { .. }
        | Route::AdminProjectFeatureUpgrade { .. }
        | Route::AdminProjectSpecApply { .. }
        | Route::AdminProjectDeployPlan { .. }
        | Route::AdminProjectDeploy { .. }
        | Route::AdminProjectReleasePlan { .. }
        | Route::AdminProjectRelease { .. }
        | Route::AdminProjectPublishPlan { .. }
        | Route::AdminProjectPublish { .. }
        | Route::AdminProjectDeliveryStatus { .. }
        | Route::AdminProjectDeliveryPreflight { .. }
        | Route::AdminProjectDeliveryStage { .. }
        | Route::AdminProjectDeliveryPromote { .. }
        | Route::AdminProjectDeliveryHermoraRetry { .. }
        | Route::AdminProjectNew
        | Route::AdminProjectImport
        | Route::AdminProjectRegister
        | Route::AdminWorkspaceCandidates
        | Route::AdminWorkspaceOnboard
        | Route::AdminPortfolioList
        | Route::AdminPortfolioEvidence
        | Route::AdminPortfolioProject { .. }
        | Route::AdminPortfolioRead { .. }
        | Route::AdminPortfolioWrite { .. }
        | Route::AdminDelivery
        | Route::AdminDeliveryPreview
        | Route::AdminDeliveryOperation { .. }
        | Route::AdminDeliveryAllowlist { .. }
        | Route::AdminDeliveryAllowlistRemove { .. }
        | Route::AdminDeliveryApprove
        | Route::AdminDeliveryPublish
        | Route::AdminDeliveryReconcile
        | Route::AdminGraduationPreview
        | Route::AdminGraduationImport
        | Route::AdminProjectIntentResolve { .. }
        | Route::AdminProjectIntentApply { .. }
        | Route::AdminProjectRemediatePlan { .. }
        | Route::AdminProjectRemediateApply { .. }
        | Route::AdminProjectDeliveryNextIdea { .. }
        | Route::AdminProjectStudioSpecSave { .. }
        | Route::AdminProjectStudioRefine { .. }
        | Route::AdminOptions => Ok(String::new()),
        Route::GetOperation { .. } => {
            // Operation lookups are read-only; the session
            // is looked up against the registry's known
            // projects to discover the owner.
            let registry = match Registry::open(db_path) {
                Ok(reg) => reg,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            let projects: Vec<(String, PathBuf)> = registry
                .list()
                .ok()
                .map(|records| {
                    records
                        .into_iter()
                        .map(|r| (r.id, PathBuf::from(r.path)))
                        .collect()
                })
                .unwrap_or_default();
            match crate::identity::lookup_session_across_projects(token, projects) {
                Ok(Some((session, _, _))) => Ok(session.subject),
                Ok(None) => Err(ApiResponse::json(
                    401,
                    serde_json::json!({
                        "error": {
                            "code": "api-unauthorized",
                            "message": "bearer session was not found in any registered project's identity store"
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                )),
                Err(err) => Err(ApiResponse::from_error(&err)),
            }
        }
        Route::ListProjects
        | Route::CreateProject
        | Route::CatalogQuery { .. }
        | Route::SharePreview
        | Route::ShareApprove
        | Route::SharePublish
        | Route::ShareReconcile
        | Route::ShareAudit
        | Route::InterestCompare
        | Route::InterestTrend
        | Route::InterestAudit
        | Route::InterestReadiness
        | Route::DeliveryStatus { .. }
        | Route::DeliveryPreflight { .. }
        | Route::DeliveryStage { .. }
        | Route::DeliveryPromote { .. }
        | Route::DeliveryHermoraRetry { .. } => {
            // Fleet routes: walk the registry to find
            // which project minted the session, then
            // validate the permission for the action.
            let registry = match Registry::open(db_path) {
                Ok(reg) => reg,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            let projects: Vec<(String, PathBuf)> = registry
                .list()
                .ok()
                .map(|records| {
                    records
                        .into_iter()
                        .map(|r| (r.id, PathBuf::from(r.path)))
                        .collect()
                })
                .unwrap_or_default();
            let (session, owner_id, owner_dir) =
                match crate::identity::lookup_session_across_projects(token, projects) {
                    Ok(Some(value)) => value,
                    Ok(None) => {
                        return Err(ApiResponse::json(
                            401,
                            serde_json::json!({
                                "error": {
                                    "code": "api-unauthorized",
                                    "message": "bearer session was not found in any registered project's identity store"
                                },
                                "contract": API_CONTRACT_VERSION,
                            }),
                        ));
                    }
                    Err(err) => return Err(ApiResponse::from_error(&err)),
                };
            if let Err(err) = check_session_state(&session, &owner_id, now) {
                return Err(ApiResponse::from_error(&err));
            }
            if let Some(perm) = required_permission(route) {
                if let Err(err) = check_session_permission(&session, &owner_id, perm) {
                    return Err(ApiResponse::from_error(&err));
                }
                if let Err(err) = crate::identity::validate_session(&session, &owner_id, perm, now)
                {
                    return Err(ApiResponse::from_error(&err));
                }
            }
            // The session is valid for the owning project.
            // The actual call target (e.g. `POST /v1/projects`
            // for fleet-level creation) does not need the
            // per-project identity config: the registry
            // enforces id/path uniqueness at write time.
            let _ = owner_dir;
            Ok(session.subject)
        }
        Route::InspectProject { id }
        | Route::Doctor { id }
        | Route::Governance { id }
        | Route::AddFeature { id }
        | Route::UpgradeProject { id }
        | Route::GenerateSpec { id }
        | Route::AgentTransition { id }
        | Route::PortfolioProject { id }
        | Route::PortfolioTag { id }
        | Route::PortfolioRelation { id }
        | Route::PortfolioReview { id }
        | Route::PortfolioEvidence { id }
        | Route::GetShare { id }
        | Route::SetShare { id }
        | Route::RemoveShare { id }
        | Route::GetInterest { id }
        | Route::ImportInterest { id }
        | Route::ApplyDeployment { id }
        | Route::StudioSpec { id }
        | Route::StudioSpecSave { id }
        | Route::StudioPreviewGet { id }
        | Route::StudioPreviewPost { id }
        | Route::StudioRefine { id } => {
            // Project-scoped route: load the project,
            // locate the session in the project directory
            // (the common case) or in any other
            // registered project's identity store (the
            // cross-project boundary case). When the
            // session's actual owner differs from the
            // target project, refuse with
            // `api-project-mismatch` so a token minted
            // for project A cannot authorize project B.
            let registry = match Registry::open(db_path) {
                Ok(reg) => reg,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            // The project must exist in the registry
            // before any session lookup, otherwise the
            // caller could probe arbitrary project
            // identifiers.
            let record = match registry.inspect(id) {
                Ok(value) => value,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            let project_dir = PathBuf::from(record.path);
            // Direct path: the session lives in the
            // target project's identity store. Cross-
            // project fallback: the session may live
            // anywhere in the registered fleet.
            let resolved = match crate::identity::load_session(&project_dir, id, token) {
                Ok(Some(value)) => Some((value, id.to_string())),
                Ok(None) => {
                    let projects: Vec<(String, PathBuf)> = registry
                        .list()
                        .ok()
                        .map(|records| {
                            records
                                .into_iter()
                                .map(|r| (r.id, PathBuf::from(r.path)))
                                .filter(|(pid, _)| pid != id)
                                .collect()
                        })
                        .unwrap_or_default();
                    match crate::identity::lookup_session_across_projects(token, projects) {
                        Ok(Some((session, owner_id, _dir))) => Some((session, owner_id)),
                        Ok(None) => None,
                        Err(err) => return Err(ApiResponse::from_error(&err)),
                    }
                }
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            let (session, owner_id) = match resolved {
                Some(value) => value,
                None => {
                    return Err(ApiResponse::json(
                        401,
                        serde_json::json!({
                            "error": {
                                "code": "api-unauthorized",
                                "message": "bearer session was not found in this project's identity store"
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    ));
                }
            };
            if owner_id != *id {
                return Err(ApiResponse::from_error(&ForgeError::ApiProjectMismatch {
                    reason: format!(
                        "session was minted for project `{owner_id}`; presenting it to project `{id}` is refused"
                    ),
                }));
            }
            if let Some(perm) = required_permission(route) {
                if let Err(err) = crate::identity::validate_session(&session, id, perm, now) {
                    return Err(ApiResponse::from_error(&err));
                }
            } else {
                // Read-only route: still require a
                // non-revoked, non-expired session for
                // this project.
                if let Err(err) = check_session_state(&session, id, now) {
                    return Err(ApiResponse::from_error(&err));
                }
            }
            Ok(session.subject)
        }
    };
    result
}

fn check_session_state(
    session: &crate::identity::AdminSession,
    project_id: &str,
    now: DateTime<Utc>,
) -> Result<(), ForgeError> {
    use crate::identity::SessionState;
    if session.project_id != project_id {
        return Err(ForgeError::ApiProjectMismatch {
            reason: format!(
                "session was minted for project `{}`; presenting it to project `{project_id}` is refused",
                session.project_id
            ),
        });
    }
    if session.state == SessionState::Revoked {
        return Err(ForgeError::ApiUnauthorized {
            reason: format!(
                "session `{}` is revoked; a new challenge must be built",
                session.session_id
            ),
        });
    }
    if session.is_expired(now) {
        return Err(ForgeError::ApiUnauthorized {
            reason: format!(
                "session `{}` expired at {}",
                session.session_id,
                session.expires_at.to_rfc3339()
            ),
        });
    }
    Ok(())
}

fn check_session_permission(
    session: &crate::identity::AdminSession,
    project_id: &str,
    perm: &str,
) -> Result<(), ForgeError> {
    if !session.permissions.iter().any(|p| p == perm) {
        return Err(ForgeError::ApiUnauthorized {
            reason: format!(
                "session `{}` for project `{project_id}` does not carry the `{perm}` permission; minted permissions: {:?}",
                session.session_id, session.permissions
            ),
        });
    }
    Ok(())
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}

pub(super) fn handle_github_push(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let signature = match request.header("x-hub-signature-256") {
        Some(value) => value.to_string(),
        None => return bad_request("GitHub push requires X-Hub-Signature-256"),
    };
    let delivery_id = match request.header("x-github-delivery") {
        Some(value) if !value.trim().is_empty() => value.to_string(),
        _ => return bad_request("GitHub push requires X-GitHub-Delivery"),
    };
    let body: Value = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(_) => return bad_request("GitHub push body must be valid JSON"),
    };
    let repository = body
        .get("repository")
        .and_then(|value| value.get("full_name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let git_ref = body.get("ref").and_then(Value::as_str).unwrap_or_default();
    let after = body
        .get("after")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let secret = match std::env::var("FORGE_GITHUB_WEBHOOK_SECRET") {
        Ok(value) if !value.is_empty() => value,
        _ => return bad_request("FORGE_GITHUB_WEBHOOK_SECRET is not configured"),
    };
    let allowed_repository = std::env::var("FORGE_GITHUB_REPOSITORY").unwrap_or_default();
    let allowed_ref =
        std::env::var("FORGE_GITHUB_REF").unwrap_or_else(|_| "refs/heads/main".to_string());
    let event = GitHubPushEvent {
        delivery_id: delivery_id.clone(),
        repository: repository.to_string(),
        git_ref: git_ref.to_string(),
        after: after.to_string(),
        signature,
        body: request.body.clone(),
    };
    if let Err(error) = verify_push(&event, secret.as_bytes(), &allowed_repository, &allowed_ref) {
        return bad_request(&error.to_string());
    }
    let project_id = match std::env::var("FORGE_GITHUB_PROJECT_ID") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return bad_request("FORGE_GITHUB_PROJECT_ID is not configured"),
    };
    let provider_id = match std::env::var("FORGE_PUBLISH_PROVIDER") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return bad_request("FORGE_PUBLISH_PROVIDER is not configured"),
    };
    let registry = match Registry::open(db_path) {
        Ok(value) => value,
        Err(error) => return ApiResponse::from_error(&error),
    };
    let mut hash = Sha256::new();
    hash.update(&request.body);
    let request_hash = format!("{:x}", hash.finalize());
    let reservation = match registry.reserve_idempotent_operation(
        "publish.github",
        &project_id,
        &delivery_id,
        &request_hash,
    ) {
        Ok(value) => value,
        Err(error) => return ApiResponse::from_error(&error),
    };
    let op_id = match reservation {
        ReservationOutcome::Reused { op_id } => {
            return ApiResponse::json(
                200,
                serde_json::json!({
                    "contract": API_CONTRACT_VERSION,
                    "delivery_id": delivery_id,
                    "operation_id": op_id,
                    "status": "duplicate",
                }),
            )
        }
        ReservationOutcome::Reserved { op_id } => op_id,
    };
    let record = match registry.inspect(&project_id) {
        Ok(value) => value,
        Err(error) => {
            let _ = registry.finalize_operation(op_id, "failed", &error.to_string());
            return ApiResponse::from_error(&error);
        }
    };
    let project_dir = PathBuf::from(record.path);
    let config_path = std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"));
    let config = match load_publish_provider_config(&config_path) {
        Ok(value) => value,
        Err(error) => {
            let _ = registry.finalize_operation(op_id, "failed", &error.to_string());
            return ApiResponse::from_error(&error);
        }
    };
    let provider = match select_provider(&config, &provider_id) {
        Ok(value) => value,
        Err(error) => {
            let _ = registry.finalize_operation(op_id, "failed", &error.to_string());
            return ApiResponse::from_error(&error);
        }
    };
    let provider_request = PublishProviderRequest {
        contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: ProviderOperation::Publish,
        provider: provider_id.clone(),
        project_id: project_id.clone(),
        revision: after.to_string(),
        operation_id: format!("github-{delivery_id}"),
        folder: Some(project_dir.display().to_string()),
        dry_run: false,
        queue_id: None,
    };
    let response = match invoke_provider(&provider, &provider_request, &project_dir) {
        Ok(value) => value,
        Err(error) => {
            let _ = registry.finalize_operation(op_id, "failed", &error.to_string());
            return ApiResponse::from_error(&error);
        }
    };
    let phase_revision = response
        .revision
        .clone()
        .unwrap_or_else(|| provider_request.revision.clone());
    let container_identity = response.container_identity.clone().unwrap_or_else(|| {
        crate::publish::providers::compose_project_name(&project_id, &phase_revision)
    });
    let _ = registry.update_operation_phase(
        op_id,
        Some(&phase_revision),
        response.build_status.as_deref(),
        response.run_status.as_deref(),
        Some(&container_identity),
    );
    let detail = format!(
        "provider={} revision={} health={} build={:?} run={:?}",
        response.provider,
        phase_revision,
        response.health,
        response.build_status,
        response.run_status
    );
    let _ = registry.finalize_operation(op_id, &response.status, &detail);
    ApiResponse::json(
        202,
        serde_json::json!({
            "contract": API_CONTRACT_VERSION,
            "delivery_id": delivery_id,
            "operation_id": op_id,
            "provider": response.provider,
            "revision": phase_revision,
            "build_status": response.build_status,
            "run_status": response.run_status,
            "container_identity": container_identity,
            "status": response.status,
            "health": response.health,
            "evidence": response.evidence,
            "recovery": response.recovery,
        }),
    )
}

pub(super) fn handle_list_projects(db_path: &Path) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.list() {
        Ok(projects) => ApiResponse::json(
            200,
            serde_json::json!({
                "projects": projects,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_inspect_project(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.inspect(id) {
        Ok(record) => ApiResponse::json(
            200,
            serde_json::json!({
                "project": record,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/projects/catalog` and `GET /v1/projects/{id}/catalog`.
///
/// Read-only projection over the shared Core catalog service
/// (`src/catalog/`). The handler parses query-string parameters,
/// builds a `CatalogQuery`, and delegates to `catalog::collect` +
/// `catalog::apply` (list) or `catalog::inspect_records` (inspect).
/// No filtering, ordering or pagination rule lives in the
/// transport: the same inputs produce the same `CatalogPage` bytes
/// the CLI and MCP surfaces serialize.
pub(super) fn handle_catalog_query(
    db_path: &Path,
    request: &ApiRequest,
    id: Option<&str>,
    now: DateTime<Utc>,
) -> ApiResponse {
    let params = match parse_catalog_query_params(request.query.as_deref()) {
        Ok(params) => params,
        Err(response) => return response,
    };
    let selection = match build_catalog_selection(&params) {
        Ok(selection) => selection,
        Err(response) => return response,
    };
    if let Err(err) = catalog::validate_max_age(params.max_age) {
        return ApiResponse::from_error(&err);
    }
    let pairs = catalog_filter_pairs_from(&params);
    let query = match catalog::CatalogQuery::from_pairs(&pairs, params.limit, params.cursor.clone())
    {
        Ok(query) => query.normalize(),
        Err(err) => return ApiResponse::from_error(&err),
    };
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: params.max_age,
        now,
    });
    match id {
        Some(project_id) => match catalog::inspect_records(&bundle, project_id) {
            Ok(records) => ApiResponse::json(
                200,
                serde_json::json!({
                    "catalog": {
                        "contract": catalog::CATALOG_CONTRACT_VERSION,
                        "project_id": project_id,
                        "records": records,
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            ),
            Err(err) => ApiResponse::from_error(&err),
        },
        None => {
            let mut page = match catalog::apply(&bundle.records, &query, &bundle.observed_at) {
                Ok(page) => page,
                Err(err) => return ApiResponse::from_error(&err),
            };
            page.sources = bundle.statuses.clone();
            ApiResponse::json(
                200,
                serde_json::json!({
                    "catalog": page,
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
    }
}

pub(super) fn handle_delivery_status(db_path: &Path, id: &str, now: DateTime<Utc>) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_status(&registry, id, now) {
        Ok(report) => ApiResponse::json(200, serde_json::to_value(&report).unwrap_or(Value::Null)),
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_delivery_preflight(
    db_path: &Path,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_preflight(&registry, id, now) {
        Ok(outcome) => ApiResponse::json(
            200,
            serde_json::to_value(&outcome.report).unwrap_or(Value::Null),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_delivery_stage(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let confirm_operation_id = match body.get("confirm_operation_id").and_then(|v| v.as_i64()) {
        Some(value) => value,
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "delivery-invalid",
                        "message": "delivery stage requires a `confirm_operation_id` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_stage(&registry, id, confirm_operation_id, now) {
        Ok(outcome) => ApiResponse::json(
            200,
            serde_json::to_value(&outcome.report).unwrap_or(Value::Null),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_delivery_promote(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let confirm_revision = match body.get("confirm_revision").and_then(|v| v.as_str()) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "delivery-invalid",
                        "message": "delivery promote requires a `confirm_revision` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_promote(&registry, id, &confirm_revision, now) {
        Ok(outcome) => ApiResponse::json(
            200,
            serde_json::to_value(&outcome.report).unwrap_or(Value::Null),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_delivery_hermora_retry(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let deployment_url = match body.get("deployment_url").and_then(|v| v.as_str()) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "delivery-invalid",
                        "message": "delivery hermora-retry requires a `deployment_url` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let secret_ref = match body.get("secret_ref").and_then(|v| v.as_str()) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "delivery-invalid",
                        "message": "delivery hermora-retry requires a `secret_ref` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_hermora_retry(
        &registry,
        id,
        &deployment_url,
        &secret_ref,
        now,
    ) {
        Ok(outcome) => ApiResponse::json(
            200,
            serde_json::to_value(&outcome.report).unwrap_or(Value::Null),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_studio_spec_get(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match crate::studio::load_session(&project_root) {
        Ok(Some(session)) => {
            let envelope = crate::studio::state::session_envelope(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "studio": envelope,
                    "contract": crate::studio::STUDIO_SESSION_CONTRACT,
                }),
            )
        }
        Ok(None) => ApiResponse::json(
            404,
            serde_json::json!({
                "error": {
                    "code": "studio-invalid-spec",
                    "message": format!("no Studio session for project '{id}'; save a spec first"),
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}
