//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use crate::core::manifest::Manifest;
use crate::deploy::{
    engine, DeployAdapterConfig, DeployConfig, DeployPlan, DeployReport, DeployRequest,
};
use crate::registry::Registry;
use crate::release::Semver;
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use super::gateway::{
    allowed_origin, commands, cors, delivery_status, delivery_write, error, guarded, is_json,
    parse_feature_query, project_delivery_write, projects, scrub_json, scrub_response, scrub_text,
    session_state, sign_in, sign_out,
};
use super::model::{Authoring, DeliveryAction};
use super::publish::{publish_plan, publish_write};
use super::release::{release_plan, release_write};

/// Shared preview → confirm → apply dispatch for the per-field
/// classification decisions. Approve and reject differ only in the
/// direction; both require a JSON body carrying the `proposal` id
/// (`<kind>-<hash>`) and echo the digest-bound confirmation the
/// browser's action control renders.
fn classify_decide(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    approve: bool,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "classify decisions require application/json",
            ),
        );
    }
    guarded(db_path, request, |req| {
        let body = req.json_body();
        if approve {
            super::super::maintain::classify_approve(db_path, id, &body, req)
        } else {
            super::super::maintain::classify_reject(db_path, id, &body, req)
        }
    })
}

pub(in crate::api) fn handle(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    route: &Route,
) -> ApiResponse {
    if !allowed_origin(config, request.header("origin")) {
        return cors(
            config,
            request,
            error(
                403,
                "admin-origin-rejected",
                "request origin is not allowed",
            ),
        );
    }

    let result = match route {
        Route::AdminSessionGet => session_state(db_path, request),
        Route::AdminSessionPost => sign_in(config, db_path, request),
        Route::AdminSessionDelete => sign_out(config, db_path, request),
        Route::AdminProjects => projects(db_path, request),
        Route::AdminCommands => commands(db_path, request),
        Route::AdminFleetStatus => guarded(db_path, request, |_| {
            super::super::status::fleet_status(db_path)
        }),
        Route::AdminProjectDetail { id } => guarded(db_path, request, |_| {
            super::super::workbench::detail(db_path, id)
        }),
        Route::AdminProjectStatus { id } => guarded(db_path, request, |_| {
            super::super::status::project_status(db_path, id)
        }),
        Route::AdminProjectMaintain { id } => guarded(db_path, request, |_| {
            super::super::maintain::maintain(db_path, id)
        }),
        Route::AdminProjectHealthRefresh { id } => guarded(db_path, request, |_| {
            super::super::workbench::refresh_health(db_path, id)
        }),
        Route::AdminProjectClassifyApprove { id } => {
            classify_decide(config, db_path, request, id, true)
        }
        Route::AdminProjectClassifyReject { id } => {
            classify_decide(config, db_path, request, id, false)
        }
        Route::AdminProjectClassifyApply { id } => {
            if !is_json(request) {
                return cors(
                    config,
                    request,
                    error(
                        415,
                        "admin-content-type-required",
                        "classify apply requires application/json",
                    ),
                );
            }
            guarded(db_path, request, |req| {
                let body = req.json_body();
                super::super::maintain::classify_apply(db_path, id, &body, req)
            })
        }
        Route::AdminProjectPlan { id } => guarded(db_path, request, |req| {
            let feature = req.query.as_deref().and_then(parse_feature_query);
            super::super::workbench::plan(db_path, id, feature.as_deref())
        }),
        Route::AdminProjectApply { id } => {
            if !is_json(request) {
                return cors(
                    config,
                    request,
                    error(
                        415,
                        "admin-content-type-required",
                        "apply requires application/json",
                    ),
                );
            }
            guarded(db_path, request, |req| {
                let body = req.json_body();
                super::super::workbench::apply(db_path, id, &body, req)
            })
        }
        Route::AdminPortfolioList => guarded(db_path, request, |req| {
            super::super::portfolio::list(db_path, req)
        }),
        Route::AdminPortfolioEvidence => guarded(db_path, request, |_| {
            super::super::portfolio::evidence(db_path)
        }),
        Route::AdminPortfolioProject { id } => guarded(db_path, request, |_| {
            super::super::portfolio::detail(db_path, id)
        }),
        Route::AdminPortfolioRead { id, kind } => guarded(db_path, request, |_| {
            super::super::portfolio::read_item(db_path, id, kind)
        }),
        Route::AdminPortfolioWrite { id, action } => {
            if !is_json(request) {
                return cors(
                    config,
                    request,
                    error(
                        415,
                        "admin-content-type-required",
                        "portfolio writes require application/json",
                    ),
                );
            }
            guarded(db_path, request, |req| {
                let body = req.json_body();
                super::super::portfolio::write_item(db_path, id, action, &body)
            })
        }
        Route::AdminPortfolioTagRemove { id } => {
            if !is_json(request) {
                return cors(
                    config,
                    request,
                    error(
                        415,
                        "admin-content-type-required",
                        "portfolio writes require application/json",
                    ),
                );
            }
            guarded(db_path, request, |req| {
                let body = req.json_body();
                super::super::portfolio::remove_tag(db_path, id, &body)
            })
        }
        Route::AdminPortfolioRelationRemove { id } => {
            if !is_json(request) {
                return cors(
                    config,
                    request,
                    error(
                        415,
                        "admin-content-type-required",
                        "portfolio writes require application/json",
                    ),
                );
            }
            guarded(db_path, request, |req| {
                let body = req.json_body();
                super::super::portfolio::remove_relation(db_path, id, &body)
            })
        }
        Route::AdminPortfolioEvidenceImport { id } => {
            if !is_json(request) {
                return cors(
                    config,
                    request,
                    error(
                        415,
                        "admin-content-type-required",
                        "portfolio writes require application/json",
                    ),
                );
            }
            guarded(db_path, request, |req| {
                let body = req.json_body();
                super::super::portfolio::import_evidence(db_path, id, &body)
            })
        }
        Route::AdminDelivery => guarded(db_path, request, |_| {
            super::super::delivery::overview(db_path)
        }),
        Route::AdminDeliveryPreview => guarded(db_path, request, |_| {
            super::super::delivery::preview(db_path)
        }),
        Route::AdminDeliveryOperation { key } => guarded(db_path, request, |_| {
            super::super::delivery::operation(db_path, key)
        }),
        Route::AdminDeliveryAllowlist { id } => {
            delivery_write(config, db_path, request, |req, body| {
                super::super::delivery::allowlist_set(db_path, id, &body, req)
            })
        }
        Route::AdminDeliveryAllowlistRemove { id } => {
            delivery_write(config, db_path, request, |req, body| {
                super::super::delivery::allowlist_remove(db_path, id, &body, req)
            })
        }
        Route::AdminDeliveryApprove => delivery_write(config, db_path, request, |req, body| {
            super::super::delivery::approve(db_path, &body, req)
        }),
        Route::AdminDeliveryPublish => delivery_write(config, db_path, request, |req, body| {
            super::super::delivery::publish(db_path, &body, req)
        }),
        Route::AdminDeliveryReconcile => delivery_write(config, db_path, request, |req, body| {
            super::super::delivery::reconcile(db_path, &body, req)
        }),
        Route::AdminProjectFeature { id } => {
            authoring_write(config, db_path, request, id, Authoring::FeatureAdd)
        }
        Route::AdminProjectFeatureRemove { id } => {
            authoring_write(config, db_path, request, id, Authoring::FeatureRemove)
        }
        Route::AdminProjectFeatureUpgrade { id } => {
            authoring_write(config, db_path, request, id, Authoring::FeatureUpgrade)
        }
        Route::AdminProjectSpec { id } => {
            authoring_write(config, db_path, request, id, Authoring::SpecGenerate)
        }
        Route::AdminProjectSpecApply { id } => {
            authoring_write(config, db_path, request, id, Authoring::SpecApply)
        }
        Route::AdminProjectDeployPlan { id } => deploy_plan(config, db_path, request, id),
        Route::AdminProjectDeploy { id } => deploy_write(config, db_path, request, id),
        Route::AdminProjectReleasePlan { id } => release_plan(config, db_path, request, id),
        Route::AdminProjectRelease { id } => release_write(config, db_path, request, id),
        Route::AdminProjectPublishPlan { id } => publish_plan(config, db_path, request, id),
        Route::AdminProjectPublish { id } => publish_write(config, db_path, request, id),
        Route::AdminProjectDeliveryStatus { id } => delivery_status(config, db_path, request, id),
        Route::AdminProjectDeliveryPreflight { id } => {
            project_delivery_write(config, db_path, request, id, DeliveryAction::Preflight)
        }
        Route::AdminProjectDeliveryStage { id } => {
            project_delivery_write(config, db_path, request, id, DeliveryAction::Stage)
        }
        Route::AdminProjectDeliveryPromote { id } => {
            project_delivery_write(config, db_path, request, id, DeliveryAction::Promote)
        }
        Route::AdminProjectDeliveryHermoraRetry { id } => {
            project_delivery_write(config, db_path, request, id, DeliveryAction::HermoraRetry)
        }
        Route::AdminProjectNew => super::super::project_management::management_write(
            config,
            db_path,
            request,
            super::super::project_management::ProjectManagement::New,
        ),
        Route::AdminProjectImport => super::super::project_management::management_write(
            config,
            db_path,
            request,
            super::super::project_management::ProjectManagement::Import,
        ),
        Route::AdminProjectRegister => super::super::project_management::management_write(
            config,
            db_path,
            request,
            super::super::project_management::ProjectManagement::Register,
        ),
        Route::AdminWorkspaceCandidates => {
            super::super::workspace::workspace_candidates(config, db_path, request)
        }
        Route::AdminWorkspaceOnboard => {
            super::super::workspace::workspace_onboard_write(config, db_path, request)
        }
        Route::AdminGraduationPreview => {
            super::super::lifecycle_exec::graduation_preview(config, db_path, request)
        }
        Route::AdminGraduationImport => {
            super::super::lifecycle_exec::graduation_import(config, db_path, request)
        }
        Route::AdminProjectIntentResolve { id } => {
            super::super::lifecycle_exec::intent_resolve(config, db_path, request, id)
        }
        Route::AdminProjectIntentApply { id } => {
            super::super::lifecycle_exec::intent_apply(config, db_path, request, id)
        }
        Route::AdminProjectRemediatePlan { id } => {
            super::super::lifecycle_exec::remediate_plan(config, db_path, request, id)
        }
        Route::AdminProjectRemediateApply { id } => {
            super::super::lifecycle_exec::remediate_apply(config, db_path, request, id)
        }
        Route::AdminProjectDeliveryNextIdea { id } => {
            super::super::lifecycle_exec::delivery_next_idea(config, db_path, request, id)
        }
        Route::AdminProjectStudioSpecSave { id } => {
            super::super::lifecycle_exec::studio_spec_save(config, db_path, request, id)
        }
        Route::AdminProjectStudioRefine { id } => {
            super::super::lifecycle_exec::studio_refine(config, db_path, request, id)
        }
        Route::AdminCatalog => guarded(db_path, request, |req| {
            super::super::catalog_browser::list(db_path, req.query.as_deref(), chrono::Utc::now())
        }),
        Route::AdminCatalogTags => guarded(db_path, request, |req| {
            super::super::catalog_browser::tags(db_path, req.query.as_deref(), chrono::Utc::now())
        }),
        Route::AdminCatalogLanguages => guarded(db_path, request, |req| {
            super::super::catalog_browser::languages(
                db_path,
                req.query.as_deref(),
                chrono::Utc::now(),
            )
        }),
        Route::AdminCatalogGaps => guarded(db_path, request, |req| {
            super::super::catalog_browser::gaps(db_path, req.query.as_deref(), chrono::Utc::now())
        }),
        Route::AdminCatalogInspect { id } => guarded(db_path, request, |req| {
            super::super::catalog_browser::inspect(
                db_path,
                req.query.as_deref(),
                id,
                chrono::Utc::now(),
            )
        }),
        Route::AdminFleetInspect { entry } => guarded(db_path, request, |req| {
            super::super::catalog_browser::fleet_inspect(db_path, req.query.as_deref(), entry)
        }),
        _ => error(404, "route-not-found", "no admin route matches the request"),
    };
    cors(config, request, result)
}

/// Build the canonical, path-free descriptor of an authoring action from its
/// structured fields, refusing (via `Err`) when a required field is missing or
/// malformed so a preview never reports a digest for an action that cannot run.
fn authoring_descriptor(
    kind: Authoring,
    id: &str,
    body: &Value,
) -> Result<Value, (&'static str, &'static str)> {
    match kind {
        Authoring::FeatureAdd => {
            let feature = body
                .get("feature")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or((
                    "admin-feature-required",
                    "feature add requires a `feature` field",
                ))?;
            let mut descriptor = json!({
                "action": "feature-add",
                "project_id": id,
                "feature": feature,
            });
            if let Some(version) = body.get("version").and_then(Value::as_str) {
                descriptor["version"] = json!(version);
            }
            Ok(descriptor)
        }
        Authoring::SpecGenerate => {
            let findings: Vec<&str> = body
                .get("findings")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default();
            if findings.is_empty() {
                return Err((
                    "admin-findings-required",
                    "spec generate requires at least one finding id",
                ));
            }
            let mut descriptor = json!({
                "action": "spec-generate",
                "project_id": id,
                "findings": findings,
            });
            if let Some(reason) = body.get("reason").and_then(Value::as_str) {
                descriptor["reason"] = json!(reason);
            }
            Ok(descriptor)
        }
        Authoring::FeatureRemove => {
            let feature = body
                .get("feature")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or((
                    "admin-feature-required",
                    "feature remove requires a `feature` field",
                ))?;
            Ok(json!({
                "action": "feature-remove",
                "project_id": id,
                "feature": feature,
            }))
        }
        Authoring::FeatureUpgrade => {
            let feature = body
                .get("feature")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or((
                    "admin-feature-required",
                    "feature upgrade requires a `feature` field",
                ))?;
            let mut descriptor = json!({
                "action": "feature-upgrade",
                "project_id": id,
                "feature": feature,
            });
            if let Some(version) = body.get("version").and_then(Value::as_str) {
                descriptor["version"] = json!(version);
            }
            Ok(descriptor)
        }
        Authoring::SpecApply => {
            let findings: Vec<&str> = body
                .get("findings")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default();
            if findings.is_empty() {
                return Err((
                    "admin-findings-required",
                    "spec apply requires at least one finding id",
                ));
            }
            let mut descriptor = json!({
                "action": "spec-apply",
                "project_id": id,
                "findings": findings,
            });
            if let Some(reason) = body.get("reason").and_then(Value::as_str) {
                descriptor["reason"] = json!(reason);
            }
            Ok(descriptor)
        }
    }
}

/// Hex SHA-256 of the canonical descriptor bytes, matching the workbench plan
/// digest. The descriptor carries no absolute path, so the digest binds
/// confirmation to the exact project, action and structured field set.
pub(in crate::api) fn authoring_digest(descriptor: &Value) -> String {
    let bytes = serde_json::to_vec(descriptor).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let out = hasher.finalize();
    let mut hex = String::with_capacity(out.len() * 2);
    for byte in out {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// One JSON-only, session-gated, preview-then-confirm authoring mutation. It
/// never trusts a browser to run the action implicitly: without `confirm: true`
/// it returns only the canonical descriptor plus its `plan_digest` and runs no
/// Core write; with `confirm: true` it recomputes the digest and, only if the
/// supplied digest still matches, delegates to the same in-process Core handler
/// the CLI and `/v1` bearer route use. A stale or forged digest is refused with
/// a fresh digest and no write. Structured fields only — never a shell, argv or
/// browser-supplied path.
fn authoring_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    kind: Authoring,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "authoring mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            // Id gate (design §4): the route `{id}` must be a valid kebab-case
            // identifier that resolves to a project managed by this registry,
            // checked before any descriptor, digest or Core call. A hostile,
            // path-bearing, unknown or observed-only id is refused with a
            // static typed error — the offending input is never echoed and no
            // absolute path is ever serialized.
            if crate::core::validate_project_id(id).is_err() {
                return error(
                    400,
                    "admin-invalid-project-id",
                    "that project name is not valid; use lowercase letters, numbers and dashes.",
                );
            }
            let managed = Registry::open(db_path)
                .ok()
                .and_then(|registry| registry.inspect(id).ok())
                .is_some();
            if !managed {
                return error(
                    404,
                    "admin-project-unmanaged",
                    "this project is not managed by this Forge registry; register it from a terminal first, then try again.",
                );
            }
            let body = req.json_body();
            let descriptor = match authoring_descriptor(kind, id, &body) {
                Ok(value) => value,
                Err((code, message)) => return error(400, code, message),
            };
            let digest = authoring_digest(&descriptor);
            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": descriptor,
                        "plan_digest": digest,
                        "confirmation": {
                            "requires": ["confirm", "plan_digest"],
                            "note": "This preview writes nothing. To run the action, send `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused.",
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            let supplied = body
                .get("plan_digest")
                .and_then(Value::as_str)
                .unwrap_or("");
            if supplied != digest {
                return ApiResponse::json(
                    409,
                    json!({
                        "error": {
                            "code": "admin-digest-mismatch",
                            "message": "the confirmed digest does not match this action's current preview; nothing was written. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": descriptor,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            let now = Utc::now();
            match kind {
                Authoring::FeatureAdd => super::super::handle_add_feature(db_path, id, req, now),
                Authoring::FeatureRemove => {
                    super::super::handle_remove_feature(db_path, id, req, now)
                }
                Authoring::FeatureUpgrade => {
                    super::super::handle_upgrade_feature(db_path, id, req, now)
                }
                Authoring::SpecGenerate => {
                    super::super::handle_generate_spec(db_path, id, req, now)
                }
                Authoring::SpecApply => super::super::handle_apply_spec(db_path, id, req, now),
            }
        }),
    )
}

/// Id gate shared by both deploy routes. The `{id}` segment must be a valid
/// kebab-case identifier that resolves to a project managed by this registry,
/// checked before any descriptor, digest, config load or Core call. A hostile,
/// path-bearing id is a `400`, an unknown or observed-only id a `404`; neither
/// echoes the offending input. On success the server-side project directory is
/// returned — it is used only to locate the manifest and never serialized.
pub(super) fn deploy_id_gate(db_path: &Path, id: &str) -> Result<PathBuf, ApiResponse> {
    if crate::core::validate_project_id(id).is_err() {
        return Err(error(
            400,
            "admin-invalid-project-id",
            "that project name is not valid; use lowercase letters, numbers and dashes.",
        ));
    }
    let dir = Registry::open(db_path)
        .ok()
        .and_then(|registry| registry.inspect(id).ok())
        .map(|record| PathBuf::from(record.path));
    match dir {
        Some(dir) => Ok(dir),
        None => Err(error(
            404,
            "admin-project-unmanaged",
            "this project is not managed by this Forge registry; register it from a terminal first, then try again.",
        )),
    }
}

/// Resolve the deploy target the same way the CLI does: an empty/absent request
/// target falls back to the manifest's `deployment.default`. Normalizing before
/// the digest is what makes the preview and the confirmed run bind the exact
/// same target.
fn normalize_deploy_target(requested: Option<&str>, default: &str) -> String {
    requested
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| default.to_string())
}

/// `GET /v1/admin/projects/{id}/deploy/plan`. Runs the read-only
/// `deploy::engine::prepare_deploy` against the server-default target and
/// returns a path-free plan view. Invokes no adapter and writes nothing; every
/// Core error is returned already scrubbed of the project directory path.
fn deploy_plan(config: &ApiConfig, db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            let (manifest, deploy_config) = match engine::load_config(&project_dir) {
                Ok(value) => value,
                Err(err) => return typed_deploy_error(&project_dir, &err),
            };
            match run_plan_view(
                &project_dir,
                &manifest,
                &deploy_config,
                id,
                &deploy_config.default_target,
            ) {
                Ok(view) => ApiResponse::json(
                    200,
                    json!({
                        "deploy_plan": view,
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(response) => response,
            }
        }),
    )
}

/// One JSON-only, session-gated deploy apply. Mirrors the authoring gate: a
/// non-JSON request is refused before the session gate; the id gate precedes any
/// descriptor or digest; the canonical descriptor is `{ project_id, target }`
/// with the server-resolved target, and `plan_digest` is its SHA-256 hex (the
/// same primitive `authoring_digest` uses). Without `confirm: true` it runs the
/// read-only plan and returns a preview plus the digest — no write, no adapter.
/// A confirmed request with a mismatched digest is refused with `409` and a
/// fresh preview — no write. Only a confirmed request whose digest matches
/// delegates to `apply_deploy` under the `admin.deploy` operation journal,
/// exactly as the CLI and the bearer `/v1` route do.
fn deploy_write(config: &ApiConfig, db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "deploy mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            // Load the manifest/config first so the empty target resolves to the
            // server default BEFORE the digest is bound — preview and confirm
            // then agree on the exact normalized target.
            let (manifest, deploy_config) = match engine::load_config(&project_dir) {
                Ok(value) => value,
                Err(err) => return typed_deploy_error(&project_dir, &err),
            };
            let body = req.json_body();
            let requested = body.get("target").and_then(Value::as_str);
            let target = normalize_deploy_target(requested, &deploy_config.default_target);
            let descriptor = json!({ "project_id": id, "target": target });
            let digest = authoring_digest(&descriptor);
            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                let view = match run_plan_view(&project_dir, &manifest, &deploy_config, id, &target)
                {
                    Ok(view) => view,
                    Err(response) => return response,
                };
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": view,
                        "plan_digest": digest,
                        "confirmation": {
                            "requires": ["confirm", "plan_digest"],
                            "note": "This preview deploys nothing. To run the deploy, send `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused.",
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            let supplied = body
                .get("plan_digest")
                .and_then(Value::as_str)
                .unwrap_or("");
            if supplied != digest {
                let view = match run_plan_view(&project_dir, &manifest, &deploy_config, id, &target)
                {
                    Ok(view) => view,
                    Err(response) => return response,
                };
                return ApiResponse::json(
                    409,
                    json!({
                        "error": {
                            "code": "admin-digest-mismatch",
                            "message": "the confirmed digest does not match this deploy's current preview; nothing was deployed. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": view,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            // Confirmed and matching: delegate to the same `apply_deploy` the CLI
            // runs, journaled as `admin.deploy`. The adapter binary path is only
            // ever produced by `deploy::engine` with its own fixed argv — this
            // route passes typed fields, never a shell, argv or browser path.
            let deploy_request = DeployRequest {
                project_id: id.to_string(),
                target,
                confirm: true,
                dry_run: false,
            };
            let adapters = DeployAdapterConfig::from_env();
            let secrets = [
                project_dir.display().to_string(),
                adapters.deployer_bin.clone(),
            ];
            match super::super::run_with_operation(
                db_path,
                "admin.deploy",
                id,
                req,
                |op_id, _registry| {
                    let report = engine::apply_deploy(
                        &project_dir,
                        &manifest,
                        &deploy_config,
                        &deploy_request,
                        &adapters,
                    )?;
                    let view = scrub_json(report_view(&report), &secrets);
                    Ok((view, op_id, id.to_string()))
                },
            ) {
                Ok((view, op_id, _pid)) => {
                    let mut response = view;
                    response["operation_id"] = json!(op_id);
                    response["project_id"] = json!(id);
                    response["contract"] = json!(API_CONTRACT_VERSION);
                    ApiResponse::json(202, response)
                }
                // `run_with_operation` renders a Core error with `from_error`,
                // which can name the adapter binary; scrub both known secrets
                // before returning it so no absolute path ever leaves this route.
                Err(response) => scrub_response(response, &secrets),
            }
        }),
    )
}

/// Run the read-only `prepare_deploy` for a resolved target and render its
/// path-free view. Errors are returned already scrubbed of the project
/// directory (plan/preview never reaches the adapter, so only that path can
/// appear).
fn run_plan_view(
    project_dir: &Path,
    manifest: &Manifest,
    config: &DeployConfig,
    id: &str,
    target: &str,
) -> Result<Value, ApiResponse> {
    let deploy_request = DeployRequest {
        project_id: id.to_string(),
        target: target.to_string(),
        confirm: false,
        dry_run: true,
    };
    let plan = match engine::prepare_deploy(project_dir, manifest, config, &deploy_request) {
        Ok(plan) => plan,
        Err(err) => return Err(typed_deploy_error(project_dir, &err)),
    };
    let secrets = [project_dir.display().to_string()];
    Ok(scrub_json(plan_view(&plan), &secrets))
}

/// Flat, path-free plan view built from typed fields — never a serialized
/// `DeployPlan`. Host, user, target path and the adapter binary are omitted;
/// only the target name/kind, source revision, artifact hash/size, health kind
/// and readiness reach the browser, so the generic frontend renderer prints
/// scalar lines with no `[object Object]` and no filesystem location.
fn plan_view(plan: &DeployPlan) -> Value {
    let mut view = json!({
        "action": "deploy",
        "project_id": plan.project_id,
        "target": plan.target.name,
        "target_kind": plan.target.kind,
        "source_revision": plan.identity.source_revision,
        "ready": plan.ready,
    });
    if let Some(artifact) = &plan.artifact {
        view["artifact_hash"] = json!(artifact.content_hash);
        view["artifact_bytes"] = json!(artifact.byte_size);
    }
    if let Some(health) = &plan.health {
        view["health_kind"] = json!(health.kind);
    }
    view
}

/// Flat, path-free apply report view built from typed fields — never a
/// serialized `DeployReport`. `state_path`, the target host/user/path and the
/// adapter binary are all omitted; stage/observation `note`s are the engine's
/// already-redacted text. The caller scrubs the whole value as a final guard.
fn report_view(report: &DeployReport) -> Value {
    let stages: Vec<Value> = report
        .stages
        .iter()
        .map(|stage| {
            json!({
                "stage": stage.stage,
                "target": stage.target,
                "status": stage.status,
                "note": stage.note,
            })
        })
        .collect();
    let mut view = json!({
        "contract": report.contract,
        "project_id": report.project_id,
        "target": report.target.name,
        "target_kind": report.target.kind,
        "adapter": report.adapter,
        "deploy_id": report.identity.id,
        "source_revision": report.identity.source_revision,
        "dry_run": report.dry_run,
        "healthy": report.healthy,
        "note": report.note,
        "stages": stages,
    });
    if let Some(observation) = &report.observation {
        view["observation"] = json!({
            "status": observation.status,
            "detail": observation.detail,
        });
    }
    if let Some(artifact) = &report.artifact {
        view["artifact_hash"] = json!(artifact.content_hash);
        view["artifact_bytes"] = json!(artifact.byte_size);
    }
    view
}

/// Render a deploy-domain Core failure with the shared status mapping, scrubbing
/// the project directory (the only absolute path plan/preview can leak) from the
/// message. The typed code and status are preserved so a boundary scenario is
/// reported honestly, never as a fake success.
fn typed_deploy_error(project_dir: &Path, err: &crate::core::ForgeError) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    let status = super::super::err_status(err);
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

/// Parse and normalize a requested release version. A missing, blank or
/// non-semver value is a static typed `400` that never echoes the offending
/// input; a valid value is normalized to its canonical semver label so the
/// preview and the confirmed run bind the exact same version.
pub(super) fn parse_release_version(raw: Option<&str>) -> Result<Semver, ApiResponse> {
    let raw = match raw {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            return Err(error(
                400,
                "admin-invalid-version",
                "a semver `version` is required (for example 1.2.3).",
            ))
        }
    };
    Semver::parse(raw).map_err(|_| {
        error(
            400,
            "admin-invalid-version",
            "the requested version is not a semver triple (expected major.minor.patch).",
        )
    })
}
