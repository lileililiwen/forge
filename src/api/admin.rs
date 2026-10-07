//! JSON-only Forge-wide administrator endpoints used by `frontend/`.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use crate::core::manifest::Manifest;
use crate::delivery::handlers as delivery_handlers;
use crate::deploy::{
    engine, DeployAdapterConfig, DeployConfig, DeployPlan, DeployReport, DeployRequest,
};
use crate::identity::global;
use crate::policy::DriftWatchConfig;
use crate::publish::providers::{
    self as publish_providers, ProviderEntry, ProviderOperation, PublishProviderRequest,
    PUBLISH_PROVIDER_CONTRACT,
};
use crate::registry::{PublishPhaseEvidence, Registry};
use crate::release::{
    engine as release_engine, release_config_from_manifest, ReleaseAdapterConfig, ReleaseReport,
    ReleaseRequest, Semver,
};

const COOKIE: &str = "forge_admin_session";

/// The `forge feature add` authoring command exposed as a session-gated,
/// preview + confirm/digest-bound admin route. Exported so the command catalog
/// can name the exact path the router registers, keeping the two in lockstep.
pub const ROUTE_ADMIN_FEATURE: &str = "POST /v1/admin/projects/{id}/feature";

/// The `forge spec generate` authoring command exposed as a session-gated,
/// preview + confirm/digest-bound admin route.
pub const ROUTE_ADMIN_SPEC: &str = "POST /v1/admin/projects/{id}/spec";

/// The `forge feature remove` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `remove_feature` Core handler the CLI runs. Exported so the command
/// catalog names the exact path the router registers.
pub const ROUTE_ADMIN_FEATURE_REMOVE: &str = "POST /v1/admin/projects/{id}/feature/remove";

/// The `forge feature upgrade` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `upgrade_feature` Core handler the CLI runs.
pub const ROUTE_ADMIN_FEATURE_UPGRADE: &str = "POST /v1/admin/projects/{id}/feature/upgrade";

/// The `forge spec apply` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `apply_routing` Core handler the CLI runs.
pub const ROUTE_ADMIN_SPEC_APPLY: &str = "POST /v1/admin/projects/{id}/spec/apply";

/// The read-only deploy plan, exposed as a session-gated admin route that
/// renders `deploy::engine::prepare_deploy` for the server-default target. It
/// invokes no adapter, writes nothing and returns a path-free plan view.
/// Exported so the command catalog names the exact path the router registers.
pub const ROUTE_ADMIN_DEPLOY_PLAN: &str = "GET /v1/admin/projects/{id}/deploy/plan";

/// The `forge deploy` apply, exposed as a session-gated, preview +
/// confirm/digest-bound admin route delegating to the same
/// `deploy::engine::apply_deploy` the bearer `/v1` route and CLI run. The
/// descriptor binds the project id and the server-resolved target — never a
/// path, argv or shell. Exported so the command catalog names the exact path.
pub const ROUTE_ADMIN_DEPLOY: &str = "POST /v1/admin/projects/{id}/deploy";

/// The read-only release plan, exposed as a session-gated admin route that
/// renders `release::engine::prepare_release` for one typed semver version. It
/// invokes no adapter, writes nothing, mutates no git state and returns a
/// path-free plan view plus the confirm digest. Exported so the command catalog
/// names the exact path the router registers.
pub const ROUTE_ADMIN_RELEASE_PLAN: &str = "GET /v1/admin/projects/{id}/release/plan";

/// The `forge release` apply, exposed as a session-gated, preview +
/// confirm/digest-bound admin route delegating to the same
/// `release::engine::apply_release` the CLI runs, using the manifest's stages.
/// The descriptor binds the project id and the normalized semver version —
/// never a stage list, path, argv, remote or shell. Exported so the command
/// catalog names the exact path the router registers.
pub const ROUTE_ADMIN_RELEASE: &str = "POST /v1/admin/projects/{id}/release";

/// The read-only provider-publish plan, exposed as a session-gated admin route
/// that resolves the provider id, the provider configuration and the committed
/// git revision server-side. It invokes no provider, writes nothing and returns
/// a path-free plan view plus the confirm digest. Exported so the command
/// catalog names the exact path the router registers.
pub const ROUTE_ADMIN_PUBLISH_PLAN: &str = "GET /v1/admin/projects/{id}/publish/plan";

/// The `forge publish` apply, exposed as a session-gated, preview +
/// confirm/digest-bound admin route delegating to the same
/// `publish::providers::invoke_provider` the CLI and the GitHub-push handler
/// run. The descriptor binds the project id, the server-resolved provider id
/// and the committed revision — never a path, binary, argv, host, SSH target or
/// credential. Exported so the command catalog names the exact path.
pub const ROUTE_ADMIN_PUBLISH: &str = "POST /v1/admin/projects/{id}/publish";

/// The read-only project-delivery status, exposed as a session-gated admin
/// route that reuses `delivery::handlers::run_status`. It invokes no provider
/// or adapter, writes nothing and returns the path-free projection plus the
/// next eligible staged confirmation. Exported so the command catalog names
/// the exact path the router registers.
pub const ROUTE_ADMIN_DELIVERY_STATUS: &str = "GET /v1/admin/projects/{id}/delivery/status";

/// The `forge delivery preflight` operation, exposed as a session-gated,
/// preview + confirm/digest-bound admin route. The descriptor binds the
/// project id and server-resolved registered revision; the browser supplies
/// no provider, path or credential. Exported so the command catalog names
/// the exact path.
pub const ROUTE_ADMIN_DELIVERY_PREFLIGHT: &str = "POST /v1/admin/projects/{id}/delivery/preflight";

/// The `forge delivery stage` operation, exposed as a session-gated, preview
/// + confirm/digest-bound admin route. The descriptor binds the project id,
/// server-resolved revision and canonical operation id; Core independently
/// verifies that the operation is a healthy same-revision preflight.
pub const ROUTE_ADMIN_DELIVERY_STAGE: &str = "POST /v1/admin/projects/{id}/delivery/stage";

/// The `forge delivery promote` operation, exposed as a session-gated,
/// preview + confirm/digest-bound admin route. The descriptor binds the
/// project id, server-resolved revision and supplied revision; Core
/// independently requires a healthy same-revision stage row.
pub const ROUTE_ADMIN_DELIVERY_PROMOTE: &str = "POST /v1/admin/projects/{id}/delivery/promote";

/// The `forge delivery hermora-retry` operation, exposed as a session-gated,
/// preview + confirm/digest-bound admin route. The browser supplies only an
/// HTTP(S) deployment URL and an environment-variable secret reference; Core
/// independently requires a healthy production row and validates the adapter
/// envelope without republishing.
pub const ROUTE_ADMIN_DELIVERY_HERMORA_RETRY: &str =
    "POST /v1/admin/projects/{id}/delivery/hermora-retry";

/// The `forge new` creation command exposed as a session-gated, preview +
/// confirm/digest-bound admin route. The browser supplies only a validated
/// single-segment project name plus typed fields; the destination directory is
/// resolved server-side from [`ADMIN_PROJECTS_ROOT_ENV`]. Exported so the
/// command catalog can name the exact path the router registers.
pub const ROUTE_ADMIN_PROJECT_NEW: &str = "POST /v1/admin/projects/new";

/// The `forge import` adoption command exposed as a session-gated, preview +
/// confirm/digest-bound admin route. The destination is resolved server-side;
/// the browser never supplies a path.
pub const ROUTE_ADMIN_PROJECT_IMPORT: &str = "POST /v1/admin/projects/import";

/// The `forge register` command exposed as a session-gated, preview +
/// confirm/digest-bound admin route. The destination is resolved server-side;
/// the browser never supplies a path.
pub const ROUTE_ADMIN_PROJECT_REGISTER: &str = "POST /v1/admin/projects/register";

/// The server-side directory that browser-driven project creation, import and
/// registration are confined to. Left **unset by default**: an unset, blank or
/// non-directory value is a typed `409 admin-prerequisite`, mirroring the
/// delivery surface's `FORGE_SHARE_PUBLISH_TARGET`. The operator declares the
/// location; the browser never names one, so no web write can target a path the
/// operator did not explicitly opt into.
pub const ADMIN_PROJECTS_ROOT_ENV: &str = "FORGE_ADMIN_PROJECTS_ROOT";

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

pub(super) fn handle(
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
        Route::AdminFleetStatus => {
            guarded(db_path, request, |_| super::status::fleet_status(db_path))
        }
        Route::AdminProjectDetail { id } => {
            guarded(db_path, request, |_| super::workbench::detail(db_path, id))
        }
        Route::AdminProjectStatus { id } => guarded(db_path, request, |_| {
            super::status::project_status(db_path, id)
        }),
        Route::AdminProjectPlan { id } => guarded(db_path, request, |req| {
            let feature = req.query.as_deref().and_then(parse_feature_query);
            super::workbench::plan(db_path, id, feature.as_deref())
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
                super::workbench::apply(db_path, id, &body, req)
            })
        }
        Route::AdminPortfolioList => {
            guarded(db_path, request, |req| super::portfolio::list(db_path, req))
        }
        Route::AdminPortfolioEvidence => {
            guarded(db_path, request, |_| super::portfolio::evidence(db_path))
        }
        Route::AdminPortfolioProject { id } => {
            guarded(db_path, request, |_| super::portfolio::detail(db_path, id))
        }
        Route::AdminPortfolioRead { id, kind } => guarded(db_path, request, |_| {
            super::portfolio::read_item(db_path, id, kind)
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
                super::portfolio::write_item(db_path, id, action, &body)
            })
        }
        Route::AdminDelivery => guarded(db_path, request, |_| super::delivery::overview(db_path)),
        Route::AdminDeliveryPreview => {
            guarded(db_path, request, |_| super::delivery::preview(db_path))
        }
        Route::AdminDeliveryOperation { key } => guarded(db_path, request, |_| {
            super::delivery::operation(db_path, key)
        }),
        Route::AdminDeliveryAllowlist { id } => {
            delivery_write(config, db_path, request, |req, body| {
                super::delivery::allowlist_set(db_path, id, &body, req)
            })
        }
        Route::AdminDeliveryAllowlistRemove { id } => {
            delivery_write(config, db_path, request, |req, body| {
                super::delivery::allowlist_remove(db_path, id, &body, req)
            })
        }
        Route::AdminDeliveryApprove => delivery_write(config, db_path, request, |req, body| {
            super::delivery::approve(db_path, &body, req)
        }),
        Route::AdminDeliveryPublish => delivery_write(config, db_path, request, |req, body| {
            super::delivery::publish(db_path, &body, req)
        }),
        Route::AdminDeliveryReconcile => delivery_write(config, db_path, request, |req, body| {
            super::delivery::reconcile(db_path, &body, req)
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
        Route::AdminProjectNew => {
            management_write(config, db_path, request, ProjectManagement::New)
        }
        Route::AdminProjectImport => {
            management_write(config, db_path, request, ProjectManagement::Import)
        }
        Route::AdminProjectRegister => {
            management_write(config, db_path, request, ProjectManagement::Register)
        }
        _ => error(404, "route-not-found", "no admin route matches the request"),
    };
    cors(config, request, result)
}

/// Which handler-backed authoring command a `/v1/admin/projects/{id}/…` write
/// runs. A closed enum — the browser picks one of these by the URL segment, and
/// no free-form command, path or argv ever reaches the handler.
#[derive(Clone, Copy)]
enum Authoring {
    FeatureAdd,
    FeatureRemove,
    FeatureUpgrade,
    SpecGenerate,
    SpecApply,
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
fn authoring_digest(descriptor: &Value) -> String {
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
                    "the project id is not a valid identifier; it may not contain a path.",
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
                    "this project is not managed by this Forge registry; register it with `forge register <path>` in a terminal first.",
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
                Authoring::FeatureAdd => super::handle_add_feature(db_path, id, req, now),
                Authoring::FeatureRemove => super::handle_remove_feature(db_path, id, req, now),
                Authoring::FeatureUpgrade => super::handle_upgrade_feature(db_path, id, req, now),
                Authoring::SpecGenerate => super::handle_generate_spec(db_path, id, req, now),
                Authoring::SpecApply => super::handle_apply_spec(db_path, id, req, now),
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
fn deploy_id_gate(db_path: &Path, id: &str) -> Result<PathBuf, ApiResponse> {
    if crate::core::validate_project_id(id).is_err() {
        return Err(error(
            400,
            "admin-invalid-project-id",
            "the project id is not a valid identifier; it may not contain a path.",
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
            "this project is not managed by this Forge registry; register it with `forge register <path>` in a terminal first.",
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
            match super::run_with_operation(db_path, "admin.deploy", id, req, |op_id, _registry| {
                let report = engine::apply_deploy(
                    &project_dir,
                    &manifest,
                    &deploy_config,
                    &deploy_request,
                    &adapters,
                )?;
                let view = scrub_json(report_view(&report), &secrets);
                Ok((view, op_id, id.to_string()))
            }) {
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
    let status = super::err_status(err);
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
fn parse_release_version(raw: Option<&str>) -> Result<Semver, ApiResponse> {
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

/// Extract the `version` query parameter for the read-only plan route, decoded
/// like every other browser query value. Used only as a semver string by Core,
/// never as a path, stage list or argument.
fn release_version_from_query(query: Option<&str>) -> Option<String> {
    for pair in query?.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == "version" {
            let decoded = super::percent_decode(value);
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }
    None
}

/// The id + version gate shared by both release routes. The `{id}` segment must
/// be a valid kebab-case identifier that resolves to a project managed by this
/// registry, and `version` must parse as semver; both are checked before any
/// descriptor, digest, config load or Core call. A hostile, path-bearing id is
/// a `400`, an unknown or observed-only id a `404`, a bad version a `400`;
/// none echoes the offending input. On success the server-side project
/// directory and the normalized semver are returned — the directory is used
/// only to locate the manifest and is never serialized.
fn release_gate(
    db_path: &Path,
    id: &str,
    raw_version: Option<&str>,
) -> Result<(PathBuf, Semver), ApiResponse> {
    let project_dir = match deploy_id_gate(db_path, id) {
        Ok(dir) => dir,
        Err(response) => return Err(response),
    };
    match parse_release_version(raw_version) {
        Ok(version) => Ok((project_dir, version)),
        Err(response) => Err(response),
    }
}

/// Build the [`ReleaseRequest`] both routes share. The stages always come from
/// the manifest-derived configuration; the browser can never supply a stage
/// list, path, remote or argv.
fn release_request(id: &str, version: &Semver, confirm: bool, dry_run: bool) -> ReleaseRequest {
    ReleaseRequest {
        project_id: id.to_string(),
        version: version.clone(),
        confirm,
        dry_run,
        retry: false,
        stages: Vec::new(),
    }
}

/// `GET /v1/admin/projects/{id}/release/plan`. Runs the read-only
/// `release::engine::prepare_release` for the typed version and returns a
/// path-free plan view plus the confirm digest. Invokes no adapter, mutates no
/// git state and writes nothing; every Core error is returned already scrubbed
/// of the project directory path.
fn release_plan(config: &ApiConfig, db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let raw = release_version_from_query(req.query.as_deref());
            let (project_dir, version) = match release_gate(db_path, id, raw.as_deref()) {
                Ok(value) => value,
                Err(response) => return response,
            };
            match run_release_plan_view(&project_dir, id, &version) {
                Ok((view, digest)) => ApiResponse::json(
                    200,
                    json!({
                        "release_plan": view,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(response) => response,
            }
        }),
    )
}

/// One JSON-only, session-gated release apply. Mirrors the deploy gate: a
/// non-JSON request is refused before the session gate; the id + version gate
/// precedes any descriptor or digest; the canonical descriptor is
/// `{ project_id, version }` with the normalized semver label, and
/// `plan_digest` is its SHA-256 hex (the same primitive `authoring_digest`
/// uses). Without `confirm: true` it runs the read-only plan and returns a
/// preview plus the digest — no write, no git stage. A confirmed request with a
/// mismatched digest is refused with `409` and a fresh preview — no write. Only
/// a confirmed request whose digest matches delegates to `apply_release` under
/// the `release` operation journal, exactly as the CLI does.
fn release_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "release mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let body = req.json_body();
            let requested = body.get("version").and_then(Value::as_str);
            let (project_dir, version) = match release_gate(db_path, id, requested) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let descriptor = json!({ "project_id": id, "version": version.label() });
            let digest = authoring_digest(&descriptor);
            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                let view = match run_release_plan_view(&project_dir, id, &version) {
                    Ok((view, _)) => view,
                    Err(response) => return response,
                };
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": view,
                        "plan_digest": digest,
                        "confirmation": {
                            "requires": ["confirm", "plan_digest"],
                            "note": "This preview releases nothing. To run the release, send `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused.",
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
                let view = match run_release_plan_view(&project_dir, id, &version) {
                    Ok((view, _)) => view,
                    Err(response) => return response,
                };
                return ApiResponse::json(
                    409,
                    json!({
                        "error": {
                            "code": "admin-digest-mismatch",
                            "message": "the confirmed digest does not match this release's current preview; nothing was released. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": view,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            // Confirmed and matching: delegate to the same `prepare_release` /
            // `apply_release` the CLI runs, journaled as `release`. The working
            // tree, git remote and adapter binaries are only ever produced by
            // `release::engine` with its own fixed argv — this route passes
            // typed fields, never a shell, stage list, argv or browser path.
            let manifest = match Manifest::load_from_dir(&project_dir, None) {
                Ok((manifest, _)) => manifest,
                Err(err) => return typed_release_error(&project_dir, &[], &err),
            };
            let config_value = match release_config_from_manifest(&manifest) {
                Ok(config) => config,
                Err(err) => return typed_release_error(&project_dir, &[], &err),
            };
            let adapters = ReleaseAdapterConfig::from_env();
            let secrets = [
                project_dir.display().to_string(),
                adapters.package_bin.clone(),
                adapters.container_bin.clone(),
                adapters.notes_bin.clone(),
            ];
            let request_value = release_request(id, &version, true, false);
            match super::run_with_operation(db_path, "release", id, req, |op_id, _registry| {
                let report = release_engine::apply_release(
                    &project_dir,
                    &manifest,
                    &config_value,
                    &request_value,
                    &adapters,
                )?;
                let view = scrub_json(release_report_view(&report), &secrets);
                Ok((view, op_id, id.to_string()))
            }) {
                Ok((view, op_id, _pid)) => {
                    let mut response = view;
                    response["operation_id"] = json!(op_id);
                    response["project_id"] = json!(id);
                    response["contract"] = json!(API_CONTRACT_VERSION);
                    ApiResponse::json(202, response)
                }
                // `run_with_operation` renders a Core error with `from_error`,
                // which can name the project directory, the adapter binaries or
                // a git remote; scrub the known secrets before returning it so
                // no absolute path ever leaves this route.
                Err(response) => scrub_response(response, &secrets),
            }
        }),
    )
}

/// Run the read-only `prepare_release` for a typed version and render its
/// path-free view plus digest. Errors are returned already scrubbed of the
/// project directory (plan/preview never reaches an adapter, so only that path
/// can appear).
fn run_release_plan_view(
    project_dir: &Path,
    id: &str,
    version: &Semver,
) -> Result<(Value, String), ApiResponse> {
    let manifest = match Manifest::load_from_dir(project_dir, None) {
        Ok((manifest, _)) => manifest,
        Err(err) => return Err(typed_release_error(project_dir, &[], &err)),
    };
    let config = match release_config_from_manifest(&manifest) {
        Ok(config) => config,
        Err(err) => return Err(typed_release_error(project_dir, &[], &err)),
    };
    let request = release_request(id, version, false, true);
    let policy = DriftWatchConfig::from_env();
    let plan =
        match release_engine::prepare_release(project_dir, &manifest, &config, &request, &policy) {
            Ok(plan) => plan,
            Err(err) => return Err(typed_release_error(project_dir, &[], &err)),
        };
    let digest = authoring_digest(&json!({ "project_id": id, "version": version.label() }));
    let secrets = [project_dir.display().to_string()];
    Ok((scrub_json(release_plan_view(&plan), &secrets), digest))
}

/// Flat, path-free release plan view built from typed fields — never a
/// serialized `PlanReport`. The changelog is reported as its project-relative
/// path and content hash; the state path, git remote and adapter binaries are
/// omitted; check details are the engine's own text and the whole value is
/// scrubbed by the caller as a final guard.
fn release_plan_view(plan: &release_engine::PlanReport) -> Value {
    let checks: Vec<Value> = plan
        .checks
        .iter()
        .map(|check| {
            json!({
                "kind": &check.kind,
                "status": &check.status,
                "applicable": check.applicable,
                "source_revision": &check.source_revision,
                "detail": &check.detail,
            })
        })
        .collect();
    let mut view = json!({
        "action": "release",
        "project_id": &plan.project_id,
        "release_id": &plan.identity.id,
        "version": &plan.identity.version,
        "source_revision": &plan.identity.source_revision,
        "docs_locales": &plan.docs_locales,
        "stages": &plan.stages,
        "checks": checks,
        "ready": plan.ready,
        "note": &plan.note,
    });
    if let Some(changelog) = &plan.changelog {
        view["changelog_path"] = json!(&changelog.path);
        view["changelog_hash"] = json!(&changelog.content_hash);
    }
    view
}

/// Flat, path-free apply report view built from typed fields — never a
/// serialized `ReleaseReport`. `state_path` (an absolute path) is omitted; the
/// per-stage and per-check notes are the engine's already-redacted text. The
/// caller scrubs the whole value as a final guard.
fn release_report_view(report: &ReleaseReport) -> Value {
    let stage_outcomes: Vec<Value> = report
        .stage_outcomes
        .iter()
        .map(|outcome| {
            json!({
                "stage": &outcome.stage,
                "target": &outcome.target,
                "status": &outcome.status,
                "identity": &outcome.identity,
                "note": &outcome.note,
                "evidence": &outcome.evidence,
                "recovery": &outcome.recovery,
            })
        })
        .collect();
    let checks: Vec<Value> = report
        .checks
        .iter()
        .map(|check| {
            json!({
                "kind": &check.kind,
                "status": &check.status,
                "applicable": check.applicable,
                "source_revision": &check.source_revision,
                "detail": &check.detail,
            })
        })
        .collect();
    let mut view = json!({
        "contract": &report.contract,
        "project_id": &report.project_id,
        "release_id": &report.identity.id,
        "version": &report.identity.version,
        "source_revision": &report.identity.source_revision,
        "docs_locales": &report.docs_locales,
        "stages": &report.stages,
        "stage_outcomes": stage_outcomes,
        "checks": checks,
        "dry_run": report.dry_run,
        "retry": report.retry,
        "healthy": report.healthy,
        "note": &report.note,
    });
    if let Some(changelog) = &report.changelog {
        view["changelog_path"] = json!(&changelog.path);
        view["changelog_hash"] = json!(&changelog.content_hash);
    }
    view
}

/// Render a release-domain Core failure with the shared status mapping,
/// scrubbing the project directory and any adapter binary names from the
/// message. The typed code and status are preserved so a boundary scenario is
/// reported honestly, never as a fake success.
fn typed_release_error(
    project_dir: &Path,
    adapter_bins: &[String],
    err: &crate::core::ForgeError,
) -> ApiResponse {
    let mut secrets = vec![project_dir.display().to_string()];
    secrets.extend(adapter_bins.iter().cloned());
    let status = super::err_status(err);
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

/// The fully server-resolved publish target for one managed project. The
/// project directory is used only to locate the provider configuration and to
/// run the provider; neither it nor the provider executable path is ever
/// serialized. The revision is the committed `HEAD` the publish binds to.
struct PublishTarget {
    project_dir: PathBuf,
    provider_id: String,
    entry: ProviderEntry,
    revision: String,
}

/// The server environment variable that selects the publish provider, matching
/// the CLI (`FORGE_PUBLISH_PROVIDER`).
const PUBLISH_PROVIDER_ENV: &str = "FORGE_PUBLISH_PROVIDER";
/// The server environment variable that overrides the provider configuration
/// path. When unset the project's `.forge/providers.yaml` is used.
const PUBLISH_PROVIDER_CONFIG_ENV: &str = "FORGE_PUBLISH_PROVIDER_CONFIG";

/// A typed `409 admin-prerequisite` for a publish that cannot run because the
/// server is not fully configured. The message is static: it names the variable
/// or configuration but never a value, a provider id or an absolute path.
fn publish_prerequisite(message: &str) -> ApiResponse {
    error(409, "admin-prerequisite", message)
}

/// Resolve the project directory, provider id, provider configuration and
/// committed revision **only** from server-side state, in the exact order the
/// CLI uses. Any failure is a typed `409 admin-prerequisite`; the provider id,
/// the configuration path and the revision are never echoed.
fn publish_target(db_path: &Path, id: &str) -> Result<PublishTarget, ApiResponse> {
    let project_dir = deploy_id_gate(db_path, id)?;
    let provider_id = std::env::var(PUBLISH_PROVIDER_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            publish_prerequisite(
                "publishing requires FORGE_PUBLISH_PROVIDER to name a configured provider; set it in the server environment.",
            )
        })?;
    let config_path = std::env::var_os(PUBLISH_PROVIDER_CONFIG_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"));
    let config = publish_providers::load_config(&config_path).map_err(|_| {
        publish_prerequisite(
            "the publish provider configuration could not be read; set FORGE_PUBLISH_PROVIDER_CONFIG to a readable file or add .forge/providers.yaml to the project.",
        )
    })?;
    let entry = publish_providers::select_provider(&config, &provider_id).map_err(|_| {
        publish_prerequisite(
            "FORGE_PUBLISH_PROVIDER does not name an enabled provider in the publish provider configuration.",
        )
    })?;
    let revision = publish_revision(&project_dir).ok_or_else(|| {
        publish_prerequisite(
            "the project has no committed git revision; commit the project before publishing from the browser.",
        )
    })?;
    publish_providers::validate_revision(&revision).map_err(|_| {
        publish_prerequisite(
            "the project's committed revision is not a 40-character hex SHA; a publish needs a committed revision.",
        )
    })?;
    Ok(PublishTarget {
        project_dir,
        provider_id,
        entry,
        revision,
    })
}

/// The committed `HEAD` of the project working tree, resolved with a fixed
/// `git rev-parse HEAD` argv. A missing repo, a git failure or an empty result
/// is `None`, which the caller reports as a prerequisite.
fn publish_revision(project_dir: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|revision| !revision.is_empty())
}

/// The path-free canonical descriptor the plan and apply digest bind to. It
/// carries only the project id, the server-resolved provider id and the
/// committed revision — never the project directory or the provider binary.
fn publish_descriptor(id: &str, provider_id: &str, revision: &str) -> Value {
    json!({ "project_id": id, "provider": provider_id, "revision": revision })
}

/// Flat, path-free publish plan view built from typed server-resolved fields —
/// never a serialized request. The provider, operation and revision reach the
/// browser; the project directory and the provider executable stay server-side.
fn publish_plan_view(id: &str, target: &PublishTarget) -> Value {
    json!({
        "action": "publish",
        "project_id": id,
        "provider": target.provider_id,
        "operation": "publish",
        "revision": target.revision,
        "note": "This preview runs no provider and writes nothing. Confirming with this exact `plan_digest` invokes the server-configured provider once.",
    })
}

/// Append one `publish` journal row for the project using the same
/// [`Registry::record_publish_phase`] the CLI writes, so the fleet `published`
/// projection reads the provider-reported state and phase columns.
fn journal_publish_phase(
    db_path: &Path,
    id: &str,
    state: &str,
    phase: PublishPhaseEvidence<'_>,
    detail: Option<&str>,
) -> Result<i64, crate::core::ForgeError> {
    let registry = Registry::open(db_path)?;
    registry.record_publish_phase(id, state, phase, detail)
}

/// `GET /v1/admin/projects/{id}/publish/plan`. Resolves the provider id, the
/// provider configuration and the committed revision server-side and returns a
/// path-free plan view plus the confirm digest. Invokes no provider and writes
/// nothing.
fn publish_plan(config: &ApiConfig, db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let target = match publish_target(db_path, id) {
                Ok(target) => target,
                Err(response) => return response,
            };
            let digest = authoring_digest(&publish_descriptor(
                id,
                &target.provider_id,
                &target.revision,
            ));
            ApiResponse::json(
                200,
                json!({
                    "publish_plan": publish_plan_view(id, &target),
                    "plan_digest": digest,
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }),
    )
}

/// One JSON-only, session-gated provider publish. Mirrors the release gate: a
/// non-JSON request is refused before the session gate; the project/provider/
/// revision resolution precedes any digest; the canonical descriptor is
/// `{ project_id, provider, revision }` and `plan_digest` is its SHA-256 hex.
/// Without `confirm: true` it returns a preview plus the digest — no provider
/// call, no journal. A confirmed request with a mismatched digest is refused
/// with `409` and a fresh preview — no provider call. Only a confirmed request
/// whose digest matches invokes the server-configured provider exactly as the
/// CLI and the GitHub-push handler do, journals one `publish` row carrying the
/// provider-reported state and phase evidence, and returns its typed result.
/// Any provider failure is journaled `failed` and returned as a typed
/// `503 publish-provider-unavailable` — never a fake success.
fn publish_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "publish mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let target = match publish_target(db_path, id) {
                Ok(target) => target,
                Err(response) => return response,
            };
            let digest = authoring_digest(&publish_descriptor(
                id,
                &target.provider_id,
                &target.revision,
            ));
            let body = req.json_body();
            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": publish_plan_view(id, &target),
                        "plan_digest": digest,
                        "confirmation": {
                            "requires": ["confirm", "plan_digest"],
                            "note": "This preview publishes nothing. To run the publish, send `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused.",
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
                            "message": "the confirmed digest does not match this publish's current preview; nothing was published. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": publish_plan_view(id, &target),
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            // Confirmed and matching: delegate to the same `invoke_provider`
            // the CLI runs. The provider executable, the provider id, the
            // project directory and the revision are all server-resolved; this
            // route passes no browser-supplied path, argv, host or credential.
            let operation_id = format!(
                "publish-{id}-{}",
                &target.revision[..target.revision.len().min(12)]
            );
            let provider_request = PublishProviderRequest {
                contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
                operation: ProviderOperation::Publish,
                provider: target.provider_id.clone(),
                project_id: id.to_string(),
                revision: target.revision.clone(),
                operation_id: operation_id.clone(),
                folder: Some(target.project_dir.display().to_string()),
                dry_run: false,
                queue_id: None,
            };
            let secrets = [
                target.project_dir.display().to_string(),
                target.entry.command.display().to_string(),
            ];
            match publish_providers::invoke_provider(
                &target.entry,
                &provider_request,
                &target.project_dir,
            ) {
                Ok(response) => {
                    let phase_revision = response
                        .revision
                        .clone()
                        .unwrap_or_else(|| target.revision.clone());
                    let container_identity =
                        response.container_identity.clone().unwrap_or_else(|| {
                            publish_providers::compose_project_name(id, &phase_revision)
                        });
                    let healthy = response.status == "done" && response.health == "healthy";
                    let journal_detail = format!(
                        "provider={} revision={} health={} build={} run={}",
                        response.provider,
                        phase_revision,
                        response.health,
                        response.build_status.as_deref().unwrap_or("unknown"),
                        response.run_status.as_deref().unwrap_or("unknown"),
                    );
                    if let Err(err) = journal_publish_phase(
                        db_path,
                        id,
                        &response.status,
                        PublishPhaseEvidence::new()
                            .revision(&phase_revision)
                            .container_identity(&container_identity)
                            .build_status_opt(response.build_status.as_deref())
                            .run_status_opt(response.run_status.as_deref()),
                        Some(&journal_detail),
                    ) {
                        return scrub_response(ApiResponse::from_error(&err), &secrets);
                    }
                    let mut view = scrub_json(
                        json!({
                            "project_id": id,
                            "operation_id": operation_id,
                            "provider": response.provider,
                            "operation": "publish",
                            "revision": phase_revision,
                            "status": response.status,
                            "health": response.health,
                            "healthy": healthy,
                            "build_status": response.build_status,
                            "run_status": response.run_status,
                            "container_identity": container_identity,
                            "evidence": response.evidence,
                            "recovery": response.recovery,
                        }),
                        &secrets,
                    );
                    view["contract"] = json!(API_CONTRACT_VERSION);
                    ApiResponse::json(202, view)
                }
                Err(err) => {
                    let reason = scrub_text(&err.to_string(), &secrets);
                    let _ = journal_publish_phase(
                        db_path,
                        id,
                        "failed",
                        PublishPhaseEvidence::new().revision(&target.revision),
                        Some(&reason),
                    );
                    ApiResponse::json(
                        503,
                        json!({
                            "error": {
                                "code": "publish-provider-unavailable",
                                "message": reason,
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
            }
        }),
    )
}

/// One staged browser delivery operation. The route segment selects the verb;
/// the request body carries only that verb's documented confirmation fields.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeliveryAction {
    Preflight,
    Stage,
    Promote,
    HermoraRetry,
}

impl DeliveryAction {
    fn name(self) -> &'static str {
        match self {
            DeliveryAction::Preflight => "delivery-preflight",
            DeliveryAction::Stage => "delivery-stage",
            DeliveryAction::Promote => "delivery-promote",
            DeliveryAction::HermoraRetry => "delivery-hermora-retry",
        }
    }
}

/// Server-resolved delivery state shared by the status read and every staged
/// mutation: the project directory plus the registered 40-hex revision Core
/// binds all delivery idempotency keys to. Neither value is serialized.
struct DeliveryResolution {
    project_dir: PathBuf,
    revision: String,
}

/// Typed staged confirmation parsed from the browser body. Only the verb's
/// documented fields are read; every other key is ignored and can never
/// become a provider, path, argv, host or credential.
enum DeliveryConfirmation {
    None,
    OperationId(i64),
    Revision(String),
    Hermora {
        deployment_url: String,
        secret_ref: String,
    },
}

/// Resolve a managed project id to its server-side directory and registered
/// revision. A missing revision is a typed prerequisite because delivery
/// idempotency and promotion are revision-bound.
fn delivery_resolution(db_path: &Path, id: &str) -> Result<DeliveryResolution, ApiResponse> {
    let project_dir = deploy_id_gate(db_path, id)?;
    let registry = Registry::open_read_only(db_path).map_err(|_| unavailable())?;
    let record = registry
        .inspect(id)
        .map_err(|err| typed_delivery_error(&project_dir, &[], &err))?;
    let revision = record
        .last_commit
        .filter(|revision| {
            revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .ok_or_else(|| {
            error(
                409,
                "admin-prerequisite",
                "delivery needs the project's registered 40-character source revision; re-run `forge register` in a terminal after committing.",
            )
        })?;
    Ok(DeliveryResolution {
        project_dir,
        revision,
    })
}

/// Parse the staged confirmation for one delivery verb. Malformed values are
/// static typed refusals that never echo the offending input.
fn delivery_confirmation(
    action: DeliveryAction,
    body: &Value,
) -> Result<DeliveryConfirmation, ApiResponse> {
    match action {
        DeliveryAction::Preflight => Ok(DeliveryConfirmation::None),
        DeliveryAction::Stage => {
            let raw = body.get("confirm_operation_id");
            let operation_id = match raw.and_then(Value::as_i64) {
                Some(value) if value >= 0 => value,
                Some(_) => {
                    return Err(error(
                        400,
                        "delivery-invalid",
                        "delivery stage requires `confirm_operation_id` as a non-negative operation id.",
                    ))
                }
                None => match raw.and_then(Value::as_str) {
                    Some(text) => {
                        let text = text.trim();
                        if text.is_empty()
                            || !text.bytes().all(|byte| byte.is_ascii_digit())
                        {
                            return Err(error(
                                400,
                                "delivery-invalid",
                                "delivery stage requires `confirm_operation_id` as a non-negative operation id.",
                            ));
                        }
                        text.parse::<i64>().map_err(|_| {
                            error(
                                400,
                                "delivery-invalid",
                                "delivery stage requires `confirm_operation_id` as a non-negative operation id.",
                            )
                        })?
                    }
                    None => {
                        return Err(error(
                            400,
                            "delivery-invalid",
                            "delivery stage requires `confirm_operation_id` from a healthy preflight.",
                        ))
                    }
                },
            };
            Ok(DeliveryConfirmation::OperationId(operation_id))
        }
        DeliveryAction::Promote => {
            let revision = body
                .get("confirm_revision")
                .and_then(Value::as_str)
                .unwrap_or("");
            if publish_providers::validate_revision(revision).is_err() {
                return Err(error(
                    400,
                    "delivery-invalid",
                    "delivery promote requires `confirm_revision` as the project's 40-character source revision.",
                ));
            }
            Ok(DeliveryConfirmation::Revision(revision.to_string()))
        }
        DeliveryAction::HermoraRetry => {
            let deployment_url = parse_delivery_url(body)?;
            let secret_ref = parse_delivery_secret_ref(body)?;
            Ok(DeliveryConfirmation::Hermora {
                deployment_url,
                secret_ref,
            })
        }
    }
}

/// Validate an HTTP(S) deployment URL without accepting embedded credentials,
/// control characters or credential-shaped values. The adapter receives the
/// canonical string only after Core independently validates it.
fn parse_delivery_url(body: &Value) -> Result<String, ApiResponse> {
    let url = body
        .get("deployment_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if url.is_empty() || url.len() > 2048 {
        return Err(error(
            400,
            "delivery-invalid",
            "delivery hermora-retry requires an HTTP(S) `deployment_url` of 1..=2048 characters.",
        ));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(error(
            400,
            "delivery-invalid",
            "delivery hermora-retry requires an HTTP(S) `deployment_url`.",
        ));
    }
    if url
        .bytes()
        .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        return Err(error(
            400,
            "delivery-invalid",
            "delivery hermora-retry requires an HTTP(S) `deployment_url` without whitespace or control characters.",
        ));
    }
    let authority = url
        .split("://")
        .nth(1)
        .unwrap_or("")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return Err(error(
            400,
            "delivery-invalid",
            "delivery hermora-retry requires an HTTP(S) `deployment_url` without embedded credentials.",
        ));
    }
    if crate::portfolio::share::validation::looks_like_secret(url) {
        return Err(error(
            400,
            "delivery-invalid",
            "delivery hermora-retry refuses a credential-shaped `deployment_url`; supply the deployment address, never a secret.",
        ));
    }
    Ok(url.to_string())
}

/// Validate an environment-variable secret reference. Only the variable name
/// travels to the adapter; the secret value stays in server-side environment.
/// Credential-shaped references are refused before any adapter call.
fn parse_delivery_secret_ref(body: &Value) -> Result<String, ApiResponse> {
    let secret_ref = body
        .get("secret_ref")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if secret_ref.len() > 128 {
        return Err(error(
            400,
            "delivery-invalid",
            "delivery hermora-retry requires `secret_ref` of 1..=128 characters.",
        ));
    }
    let name = secret_ref.strip_prefix("env:").unwrap_or("");
    let mut chars = name.chars();
    let valid = !name.is_empty()
        && matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|next| next.is_ascii_alphanumeric() || next == '_');
    if !valid {
        return Err(error(
            400,
            "delivery-invalid",
            "delivery hermora-retry requires `secret_ref` as an environment-variable reference such as `env:HERMORA_TOKEN`.",
        ));
    }
    if crate::portfolio::share::validation::looks_like_secret(secret_ref) {
        return Err(error(
            400,
            "delivery-invalid",
            "delivery hermora-retry refuses a credential-shaped `secret_ref`; supply the variable name, never the secret.",
        ));
    }
    Ok(secret_ref.to_string())
}

/// Read the delivery projection for a resolved project and scrub every
/// absolute project path from it. Core already scrubs credential-shaped
/// evidence before it reaches the journal.
fn delivery_report(
    db_path: &Path,
    id: &str,
    resolution: &DeliveryResolution,
) -> Result<Value, ApiResponse> {
    let registry = Registry::open_read_only(db_path).map_err(|_| unavailable())?;
    let now = Utc::now();
    let report = delivery_handlers::run_status(&registry, id, now)
        .map_err(|err| typed_delivery_error(&resolution.project_dir, &[], &err))?;
    let value = serde_json::to_value(&report).map_err(|_| {
        error(
            500,
            "admin-delivery-unavailable",
            "delivery status could not be encoded; nothing was changed.",
        )
    })?;
    Ok(scrub_json(
        value,
        &[resolution.project_dir.display().to_string()],
    ))
}

/// The path-free canonical descriptor a preview digest binds. Hermora URL and
/// secret reference participate in the digest but are intentionally omitted
/// from the visible preview; the digest is opaque and bound to them.
fn delivery_descriptor(
    action: DeliveryAction,
    id: &str,
    revision: &str,
    confirmation: &DeliveryConfirmation,
) -> Value {
    let mut descriptor = json!({
        "action": action.name(),
        "project_id": id,
        "revision": revision,
    });
    match confirmation {
        DeliveryConfirmation::None => {}
        DeliveryConfirmation::OperationId(operation_id) => {
            descriptor["confirm_operation_id"] = json!(operation_id);
        }
        DeliveryConfirmation::Revision(confirm_revision) => {
            descriptor["confirm_revision"] = json!(confirm_revision);
        }
        DeliveryConfirmation::Hermora {
            deployment_url,
            secret_ref,
        } => {
            descriptor["deployment_url"] = json!(deployment_url);
            descriptor["secret_ref"] = json!(secret_ref);
        }
    }
    descriptor
}

/// Flat preview fields the generic workbench renderer can print without
/// bespoke templates. Sensitive Hermora inputs are bound into the digest but
/// never echoed here.
fn delivery_preview(
    action: DeliveryAction,
    id: &str,
    resolution: &DeliveryResolution,
    confirmation: &DeliveryConfirmation,
    report: &Value,
) -> Value {
    let mut preview = json!({
        "action": action.name(),
        "project_id": id,
        "revision": resolution.revision,
        "phase": report.pointer("/phase").cloned().unwrap_or(Value::Null),
        "next": delivery_next(report, &resolution.revision),
    });
    match confirmation {
        DeliveryConfirmation::None => {}
        DeliveryConfirmation::OperationId(operation_id) => {
            preview["confirm_operation_id"] = json!(operation_id);
        }
        DeliveryConfirmation::Revision(confirm_revision) => {
            preview["confirm_revision"] = json!(confirm_revision);
        }
        DeliveryConfirmation::Hermora { .. } => {
            preview["deployment_url"] = json!("bound into the digest, not displayed");
            preview["secret_ref"] = json!("bound into the digest, not displayed");
        }
    }
    preview
}

/// Plain-language next staged confirmation derived from the scrubbed report.
/// Failed or stale prerequisites report `blocked` with recovery guidance
/// rather than suggesting the same mutation blindly.
fn delivery_next(report: &Value, revision: &str) -> Value {
    let verb = |name: &str| {
        report
            .pointer(&format!("/{name}"))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let state = |value: &Value| {
        value
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let report_revision = |value: &Value| {
        value
            .get("revision")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let operation_id = |value: &Value| value.get("op_id").cloned().unwrap_or(Value::Null);

    let preflight = verb("preflight");
    let stage = verb("stage");
    let promote = verb("promote");
    let hermora = verb("hermora");
    let healthy = |value: &Value| state(value) == "done" && report_revision(value) == revision;

    if !healthy(&preflight) {
        return json!({
            "action": DeliveryAction::Preflight.name(),
            "requires": [],
            "blocked": state(&preflight) == "failed",
            "guidance": "Run delivery preflight for the registered revision. A failed preflight must be resolved before stage.",
        });
    }
    if !healthy(&stage) {
        return json!({
            "action": DeliveryAction::Stage.name(),
            "requires": ["confirm_operation_id"],
            "confirm_operation_id": operation_id(&preflight),
            "blocked": state(&stage) == "failed",
            "guidance": "Stage with the healthy preflight operation id. A failed stage blocks promotion.",
        });
    }
    if !healthy(&promote) {
        return json!({
            "action": DeliveryAction::Promote.name(),
            "requires": ["confirm_revision"],
            "confirm_revision": revision,
            "blocked": state(&promote) == "failed",
            "guidance": "Promote with the registered revision after a healthy stage. A failed promotion blocks Hermora enrollment.",
        });
    }
    json!({
        "action": DeliveryAction::HermoraRetry.name(),
        "requires": ["deployment_url", "secret_ref"],
        "blocked": state(&hermora) == "failed",
        "guidance": "Enroll the healthy deployment with Hermora using an HTTP(S) URL and environment-variable secret reference. This never republishes.",
    })
}

/// `GET /v1/admin/projects/{id}/delivery/status`. Session-gated read-only
/// delivery status; invokes no provider or adapter and writes nothing.
fn delivery_status(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let resolution = match delivery_resolution(db_path, id) {
                Ok(resolution) => resolution,
                Err(response) => return response,
            };
            match delivery_report(db_path, id, &resolution) {
                Ok(report) => ApiResponse::json(
                    200,
                    json!({
                        "delivery_status": report,
                        "next": delivery_next(&report, &resolution.revision),
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(response) => response,
            }
        }),
    )
}

/// One JSON-only, session-gated staged delivery mutation. The request is
/// refused as non-JSON before the session gate; the managed-project,
/// typed-confirmation and digest checks precede any Core call. Confirmed
/// matching requests delegate to the unchanged delivery handler for the
/// verb. Core's typed errors preserve status and code; responses are
/// scrubbed of the project path and exact Hermora inputs.
fn project_delivery_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    action: DeliveryAction,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "delivery mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let resolution = match delivery_resolution(db_path, id) {
                Ok(resolution) => resolution,
                Err(response) => return response,
            };
            let body = req.json_body();
            let confirmation = match delivery_confirmation(action, &body) {
                Ok(confirmation) => confirmation,
                Err(response) => return response,
            };
            let digest = authoring_digest(&delivery_descriptor(
                action,
                id,
                &resolution.revision,
                &confirmation,
            ));
            let report = match delivery_report(db_path, id, &resolution) {
                Ok(report) => report,
                Err(response) => return response,
            };
            let preview = delivery_preview(action, id, &resolution, &confirmation, &report);
            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": preview,
                        "plan_digest": digest,
                        "confirmation": {
                            "requires": ["confirm", "plan_digest"],
                            "note": "This preview dispatches nothing. To run this staged delivery verb, send `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused.",
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
                            "message": "the confirmed digest does not match this delivery preview; nothing was dispatched. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": preview,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }

            let mut secrets = vec![resolution.project_dir.display().to_string()];
            if let DeliveryConfirmation::Hermora {
                deployment_url,
                secret_ref,
            } = &confirmation
            {
                secrets.push(deployment_url.clone());
                secrets.push(secret_ref.clone());
            }
            let registry = match Registry::open(db_path) {
                Ok(registry) => registry,
                Err(_) => return unavailable(),
            };
            let now = Utc::now();
            let outcome = match (action, &confirmation) {
                (DeliveryAction::Preflight, DeliveryConfirmation::None) => {
                    delivery_handlers::run_preflight(&registry, id, now)
                }
                (DeliveryAction::Stage, DeliveryConfirmation::OperationId(operation_id)) => {
                    delivery_handlers::run_stage(&registry, id, *operation_id, now)
                }
                (DeliveryAction::Promote, DeliveryConfirmation::Revision(confirm_revision)) => {
                    delivery_handlers::run_promote(&registry, id, confirm_revision, now)
                }
                (
                    DeliveryAction::HermoraRetry,
                    DeliveryConfirmation::Hermora {
                        deployment_url,
                        secret_ref,
                    },
                ) => delivery_handlers::run_hermora_retry(
                    &registry,
                    id,
                    deployment_url,
                    secret_ref,
                    now,
                ),
                _ => {
                    return error(
                        400,
                        "delivery-invalid",
                        "the staged delivery confirmation does not match this route.",
                    )
                }
            };
            match outcome {
                Ok(outcome) => {
                    let report = match serde_json::to_value(&outcome.report) {
                        Ok(report) => scrub_json(report, &secrets),
                        Err(_) => {
                            return error(
                                500,
                                "admin-delivery-unavailable",
                                "delivery completed but its report could not be encoded; check the journal directly.",
                            )
                        }
                    };
                    ApiResponse::json(
                        202,
                        json!({
                            "project_id": id,
                            "operation_id": outcome.op_id,
                            "action": action.name(),
                            "delivery": report,
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
                Err(err) => typed_delivery_error(&resolution.project_dir, &secrets, &err),
            }
        }),
    )
}

/// Map a delivery Core error to its typed API status/code while scrubbing
/// the project path and any exact caller-supplied Hermora inputs. The typed
/// code and status are preserved so boundary scenarios stay honest.
fn typed_delivery_error(
    project_dir: &Path,
    secrets: &[String],
    err: &crate::core::ForgeError,
) -> ApiResponse {
    let mut all = vec![project_dir.display().to_string()];
    all.extend(secrets.iter().cloned());
    scrub_response(ApiResponse::from_error(err), &all)
}

/// Replace the raw body of an already-built error response with a scrubbed copy,
/// preserving status and shape. Used on the confirmed-apply failure path where
/// the engine's message can name the adapter binary.
fn scrub_response(response: ApiResponse, secrets: &[String]) -> ApiResponse {
    let status = response.status;
    let parsed: Value = serde_json::from_slice(&response.body).unwrap_or(Value::Null);
    ApiResponse::json(status, scrub_json(parsed, secrets))
}

/// Recursively scrub every string in a JSON value: replace each known absolute
/// secret verbatim, then redact any remaining absolute-path token.
fn scrub_json(value: Value, secrets: &[String]) -> Value {
    match value {
        Value::String(text) => Value::String(scrub_text(&text, secrets)),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(|v| scrub_json(v, secrets)).collect())
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, item) in map {
                out.insert(key, scrub_json(item, secrets));
            }
            Value::Object(out)
        }
        other => other,
    }
}

/// Replace each secret substring (the project directory, the adapter binary)
/// with a fixed marker, then apply the whitespace-token path redactor so no bare
/// absolute path survives either.
fn scrub_text(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            out = out.replace(secret.as_str(), "[local path]");
        }
    }
    redact_local_paths(&out)
}

/// Replace whitespace-separated tokens that look like absolute local paths
/// (`/home/…`, `C:\…`, `key=/value`) with a fixed marker. API route strings
/// (always under `/v1/…`) are left intact: they are self-authored endpoints,
/// never filesystem locations. Mirrors the delivery projection's discipline so a
/// deploy response can never carry a real filesystem path.
fn redact_local_paths(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            let is_route = token == "/v1" || token.starts_with("/v1/");
            let is_abs = !is_route
                && (token.starts_with('/')
                    || (token.len() > 2
                        && token.as_bytes()[1] == b':'
                        && (token.as_bytes()[2] == b'\\' || token.as_bytes()[2] == b'/'))
                    || token.contains("=/")
                    || token.contains(":\\"));
            if is_abs {
                "[local path]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Which handler-backed project-management command a
/// `/v1/admin/projects/{new,import,register}` write runs. A closed enum — the
/// browser picks one by the URL segment, and no free-form command, path or argv
/// ever reaches the handler.
#[derive(Clone, Copy)]
enum ProjectManagement {
    New,
    Import,
    Register,
}

impl ProjectManagement {
    fn action(self) -> &'static str {
        match self {
            ProjectManagement::New => "project-new",
            ProjectManagement::Import => "project-import",
            ProjectManagement::Register => "project-register",
        }
    }

    fn operation_kind(self) -> &'static str {
        match self {
            ProjectManagement::New => "admin.project.new",
            ProjectManagement::Import => "admin.project.import",
            ProjectManagement::Register => "admin.project.register",
        }
    }
}

/// Build the canonical, **path-free** descriptor of a project-management action
/// from its structured fields. It refuses a missing field, a non-kebab
/// `project` name (so no path separator, traversal segment, percent-encoding or
/// uppercase reaches the filesystem) and an invalid `id` override, so a preview
/// never reports a digest for an action that cannot run. The absolute
/// destination is deliberately absent: the digest binds the reviewed fields, not
/// a machine location.
fn management_descriptor(
    kind: ProjectManagement,
    body: &Value,
) -> Result<Value, (&'static str, &'static str)> {
    let project = body
        .get("project")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or(("admin-field-required", "a `project` name is required"))?;
    if crate::core::validate_project_id(project).is_err() {
        return Err((
            "admin-invalid-project-name",
            "the `project` name must be a lowercase kebab-case identifier with no path separators.",
        ));
    }
    match kind {
        ProjectManagement::New => {
            let profile = body
                .get("profile")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or((
                    "admin-field-required",
                    "creating a project requires a `profile` field",
                ))?;
            let mut descriptor = json!({
                "action": kind.action(),
                "project": project,
                "profile": profile,
            });
            if let Some(name) = body
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                descriptor["name"] = json!(name);
            }
            let mut features: Vec<&str> = body
                .get("features")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default();
            features.sort_unstable();
            features.dedup();
            if !features.is_empty() {
                descriptor["features"] = json!(features);
            }
            Ok(descriptor)
        }
        ProjectManagement::Import => {
            let mut descriptor = json!({ "action": kind.action(), "project": project });
            if let Some(profile) = body
                .get("profile")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                descriptor["profile"] = json!(profile);
            }
            if let Some(id) = body
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                if crate::core::validate_project_id(id).is_err() {
                    return Err((
                        "admin-invalid-project-name",
                        "the `id` override must be a lowercase kebab-case identifier with no path separators.",
                    ));
                }
                descriptor["id"] = json!(id);
            }
            Ok(descriptor)
        }
        ProjectManagement::Register => Ok(json!({ "action": kind.action(), "project": project })),
    }
}

/// Resolve the server-side project root. Unset, blank or non-directory is a
/// typed `409 admin-prerequisite`; the configured value is never echoed. The
/// root is canonicalized once so every destination is compared against the same
/// resolved prefix.
fn projects_root() -> Result<PathBuf, ApiResponse> {
    let value = std::env::var(ADMIN_PROJECTS_ROOT_ENV).unwrap_or_default();
    if value.trim().is_empty() {
        return Err(error(
            409,
            "admin-prerequisite",
            "the server has no project root configured; set the server-side project root environment variable before managing projects from the browser.",
        ));
    }
    match PathBuf::from(value.trim()).canonicalize() {
        Ok(canonical) if canonical.is_dir() => Ok(canonical),
        _ => Err(error(
            409,
            "admin-prerequisite",
            "the configured server-side project root does not resolve to an existing directory; fix the server configuration and retry.",
        )),
    }
}

/// Join a validated single-segment project name to the resolved root. For
/// `new` the destination may not exist yet; for `register`/`import` the
/// directory must exist, canonicalize (resolving symlinks) and stay a descendant
/// of the root, so a symlink out of the root is refused before any read.
fn management_destination(
    kind: ProjectManagement,
    root: &Path,
    name: &str,
) -> Result<PathBuf, ApiResponse> {
    let candidate = root.join(name);
    if matches!(kind, ProjectManagement::New) {
        return Ok(candidate);
    }
    let canonical = candidate.canonicalize().map_err(|_| {
        error(
            409,
            "admin-prerequisite",
            "the named project directory does not exist under the configured root; create or place it there first.",
        )
    })?;
    if !canonical.starts_with(root) {
        return Err(error(
            409,
            "admin-prerequisite",
            "the named project directory resolves outside the configured root; refusing to touch it.",
        ));
    }
    if !canonical.is_dir() {
        return Err(error(
            409,
            "admin-prerequisite",
            "the named project path is not a directory under the configured root.",
        ));
    }
    Ok(canonical)
}

/// Build the read-only, path-free preview for one management action. It runs
/// only the Core function's read side (`normalize_explicit` validation,
/// `inspect_import` proposal, manifest load) and writes nothing. A Core failure
/// is returned as an already-typed response (scrubbed by the caller).
fn management_preview(
    kind: ProjectManagement,
    descriptor: &Value,
    dest: &Path,
) -> Result<Value, ApiResponse> {
    match kind {
        ProjectManagement::New => {
            let profile = descriptor["profile"].as_str().unwrap_or_default();
            let project = descriptor["project"].as_str().unwrap_or_default();
            let name = descriptor.get("name").and_then(Value::as_str);
            let features: Vec<String> = descriptor
                .get("features")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let request = crate::generate::normalize_explicit(
                Some(profile),
                Some(project),
                name,
                &features,
                dest,
                None,
            )
            .map_err(|err| management_error(&err, &[dest]))?;
            Ok(json!({
                "action": "project-new",
                "project": request.id,
                "profile": request.profile,
                "name": request.name,
                "features": request.features,
            }))
        }
        ProjectManagement::Import => {
            let profile = descriptor.get("profile").and_then(Value::as_str);
            let proposal = crate::import::inspect_import(dest, profile)
                .map_err(|err| management_error(&err, &[dest]))?;
            Ok(json!({
                "action": "project-import",
                "suggested_profile": proposal.suggested_profile,
                "suggested_maturity": proposal.suggested_maturity,
                "confidence": proposal.confidence,
                "language": proposal.language.value,
                "manifest_exists": proposal.manifest_exists,
                "explicit_profile": proposal.explicit_profile,
                "alternatives": proposal.alternatives,
            }))
        }
        ProjectManagement::Register => {
            let (manifest, _) = Manifest::load_from_dir(dest, None)
                .map_err(|err| management_error(&err, &[dest]))?;
            Ok(json!({
                "action": "project-register",
                "manifest": {
                    "id": manifest.project.id,
                    "name": manifest.project.name,
                    "profile": manifest.project.profile,
                },
                "ready": true,
            }))
        }
    }
}

/// One JSON-only, session-gated, preview-then-confirm project-management
/// mutation (`forge new` / `forge import` / `forge register`). The browser
/// supplies only structured typed fields; the destination is resolved from the
/// server-side root. Without `confirm: true` it returns a path-free preview and
/// the digest and writes nothing; a mismatched digest is a `409` with a fresh
/// preview and no write; only a confirmed matching digest delegates to the same
/// in-process Core function the CLI runs, journaled through the shared operation
/// boundary. Every response is scrubbed of the root and destination.
fn management_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    kind: ProjectManagement,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "project-management mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let body = req.json_body();
            let descriptor = match management_descriptor(kind, &body) {
                Ok(value) => value,
                Err((code, message)) => return error(400, code, message),
            };
            let digest = authoring_digest(&descriptor);
            let root = match projects_root() {
                Ok(root) => root,
                Err(response) => return response,
            };
            let project = descriptor["project"].as_str().unwrap_or_default();
            let dest = match management_destination(kind, &root, project) {
                Ok(dest) => dest,
                Err(response) => return response,
            };
            let secrets = [root.display().to_string(), dest.display().to_string()];

            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                let preview = match management_preview(kind, &descriptor, &dest) {
                    Ok(view) => scrub_json(view, &secrets),
                    Err(response) => return scrub_response(response, &secrets),
                };
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": preview,
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
                let preview = match management_preview(kind, &descriptor, &dest) {
                    Ok(view) => scrub_json(view, &secrets),
                    Err(response) => return scrub_response(response, &secrets),
                };
                return ApiResponse::json(
                    409,
                    json!({
                        "error": {
                            "code": "admin-digest-mismatch",
                            "message": "the confirmed digest does not match this action's current preview; nothing was written. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": preview,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            // Confirmed and matching: delegate to the same in-process Core
            // function the CLI runs, journaled as `admin.project.*`. No shell,
            // argv or browser path ever reaches the handler.
            let kind_name = kind.operation_kind();
            let result = match kind {
                ProjectManagement::New => {
                    run_management_new(db_path, req, kind_name, &descriptor, &dest)
                }
                ProjectManagement::Import => {
                    run_management_import(db_path, req, kind_name, &descriptor, &dest)
                }
                ProjectManagement::Register => {
                    run_management_register(db_path, req, kind_name, &dest)
                }
            };
            match result {
                Ok(response) => response,
                Err(response) => scrub_response(response, &secrets),
            }
        }),
    )
}

/// Confirmed `forge new`: normalize the typed fields against the server-resolved
/// destination and run the deterministic `generate` Core function under the
/// shared operation journal. Returns a path-free `created` view.
fn run_management_new(
    db_path: &Path,
    req: &ApiRequest,
    kind: &str,
    descriptor: &Value,
    dest: &Path,
) -> Result<ApiResponse, ApiResponse> {
    let profile = descriptor["profile"].as_str().unwrap_or_default();
    let project = descriptor["project"].as_str().unwrap_or_default();
    let name = descriptor.get("name").and_then(Value::as_str);
    let features: Vec<String> = descriptor
        .get("features")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let normalized = crate::generate::normalize_explicit(
        Some(profile),
        Some(project),
        name,
        &features,
        dest,
        None,
    )
    .map_err(|err| ApiResponse::from_error(&err))?;
    let (view, op_id, project_id) = super::run_with_operation(
        db_path,
        kind,
        super::API_SYNTHETIC_PROJECT,
        req,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let generated = crate::generate::generate(&mut registry, &normalized)?;
            Ok((
                json!({
                    "created": {
                        "id": generated.record.id,
                        "profile": generated.record.profile,
                        "name": generated.record.name,
                        "files": generated.files,
                    }
                }),
                op_id,
                generated.record.id,
            ))
        },
    )?;
    Ok(managed_response(view, op_id, project_id))
}

/// Confirmed `forge import`: adopt an existing directory through the same
/// `adopt_import` Core function the CLI runs, under the shared operation
/// journal. Returns a path-free `imported` view.
fn run_management_import(
    db_path: &Path,
    req: &ApiRequest,
    kind: &str,
    descriptor: &Value,
    dest: &Path,
) -> Result<ApiResponse, ApiResponse> {
    let profile = descriptor.get("profile").and_then(Value::as_str);
    let id = descriptor.get("id").and_then(Value::as_str);
    let (view, op_id, project_id) = super::run_with_operation(
        db_path,
        kind,
        super::API_SYNTHETIC_PROJECT,
        req,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let record = crate::import::adopt_import(&mut registry, dest, profile, id)?;
            Ok((
                json!({
                    "imported": {
                        "id": record.id,
                        "name": record.name,
                        "profile": record.profile,
                    }
                }),
                op_id,
                record.id,
            ))
        },
    )?;
    Ok(managed_response(view, op_id, project_id))
}

/// Confirmed `forge register`: persist an existing manifest directory through
/// the same `Registry::register` Core function the CLI runs, under the shared
/// operation journal. Returns a path-free `registered` view.
fn run_management_register(
    db_path: &Path,
    req: &ApiRequest,
    kind: &str,
    dest: &Path,
) -> Result<ApiResponse, ApiResponse> {
    let (view, op_id, project_id) = super::run_with_operation(
        db_path,
        kind,
        super::API_SYNTHETIC_PROJECT,
        req,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let record = registry.register(dest, None)?;
            Ok((
                json!({
                    "registered": {
                        "id": record.id,
                        "name": record.name,
                        "profile": record.profile,
                    }
                }),
                op_id,
                record.id,
            ))
        },
    )?;
    Ok(managed_response(view, op_id, project_id))
}

/// Attach the operation id, resolved project id and contract to a
/// project-management success view.
fn managed_response(mut view: Value, op_id: i64, project_id: String) -> ApiResponse {
    view["operation_id"] = json!(op_id);
    view["project_id"] = json!(project_id);
    view["contract"] = json!(API_CONTRACT_VERSION);
    ApiResponse::json(202, view)
}

/// Render a project-management Core failure with the shared status mapping,
/// scrubbing the resolved destination (the only absolute path the preview can
/// leak) from the message. The typed code and status are preserved.
fn management_error(err: &crate::core::ForgeError, secrets: &[&Path]) -> ApiResponse {
    let owned: Vec<String> = secrets.iter().map(|p| p.display().to_string()).collect();
    let status = super::err_status(err);
    ApiResponse::json(
        status,
        json!({
            "error": {
                "code": err.code(),
                "message": scrub_text(&err.to_string(), &owned),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// One JSON-only, session-gated delivery mutation: a non-JSON content type
/// is refused before the session gate ever runs the handler, and the
/// handler itself enforces the confirm- and digest-binding. Delivery
/// actions are typed in-process Core calls — never a shell, never a
/// browser-supplied path.
fn delivery_write<F>(config: &ApiConfig, db_path: &Path, request: &ApiRequest, f: F) -> ApiResponse
where
    F: FnOnce(&ApiRequest, serde_json::Value) -> ApiResponse,
{
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "delivery mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| f(req, req.json_body())),
    )
}

/// Run `f` only after the same global-admin session gate every other admin
/// read/write uses. A missing or expired session is a `401`, never a partial
/// workbench projection; a registry failure is an honest `503`. The workbench
/// `f` closures perform only typed in-process Core calls — no shell.
fn guarded<F>(db_path: &Path, request: &ApiRequest, f: F) -> ApiResponse
where
    F: FnOnce(&ApiRequest) -> ApiResponse,
{
    let token = request
        .cookies
        .get(COOKIE)
        .map(String::as_str)
        .unwrap_or("");
    match global::session_valid(db_path, token) {
        Ok(true) => f(request),
        Ok(false) => error(
            401,
            "api-unauthorized",
            "Forge administrator session is required",
        ),
        Err(_) => unavailable(),
    }
}

/// True when the request declares a JSON content type, so a mutating apply
/// never parses a body of an unexpected media type.
fn is_json(request: &ApiRequest) -> bool {
    request
        .header("content-type")
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
}

/// Extract the optional `feature` query parameter (the one catalog feature id
/// to upgrade). It is percent-decoded like every other browser query value and
/// used only as a catalog feature key by Core — never as a path or argument.
fn parse_feature_query(query: &str) -> Option<String> {
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == "feature" && !value.is_empty() {
            return Some(super::percent_decode(value));
        }
    }
    None
}

pub(super) fn handle_preflight(config: &ApiConfig, request: &ApiRequest) -> ApiResponse {
    if !allowed_origin(config, request.header("origin")) {
        return error(
            403,
            "admin-origin-rejected",
            "request origin is not allowed",
        );
    }
    ApiResponse {
        status: 204,
        headers: cors_headers(config, request),
        body: Vec::new(),
    }
}

fn session_state(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    match global::is_configured(db_path) {
        Ok(configured) => {
            let authenticated = request
                .cookies
                .get(COOKIE)
                .filter(|token| !token.is_empty())
                .and_then(|token| global::session_valid(db_path, token).ok())
                .unwrap_or(false);
            let mut response = ApiResponse::json(
                200,
                json!({
                    "configured": configured,
                    "authenticated": authenticated,
                    "setup_command": if configured { serde_json::Value::Null } else { json!("forge identity setup --email you@example.com") },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
            if request.cookies.contains_key(COOKIE) && !authenticated {
                response.headers.insert(
                    "set-cookie".to_string(),
                    format!(
                        "{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
                        if request
                            .header("origin")
                            .unwrap_or("")
                            .starts_with("https://")
                        {
                            "; Secure"
                        } else {
                            ""
                        }
                    ),
                );
            }
            response
        }
        Err(_) => unavailable(),
    }
}

fn sign_in(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    if request.body.len() > 8192 {
        return error(413, "admin-request-too-large", "login request is too large");
    }
    if !request
        .header("content-type")
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
    {
        return error(
            415,
            "admin-content-type-required",
            "login requires application/json",
        );
    }
    let input: LoginBody = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(_) => {
            return error(
                400,
                "admin-invalid-request",
                "email and password are required",
            )
        }
    };
    let token = match global::authenticate(db_path, &input.email, &input.password) {
        Ok(Some(token)) => token,
        Ok(None) => return error(401, "api-unauthorized", "email or password is incorrect"),
        Err(_) => return unavailable(),
    };
    let mut response = ApiResponse::json(
        200,
        json!({ "authenticated": true, "contract": API_CONTRACT_VERSION }),
    );
    response.headers.insert(
        "set-cookie".to_string(),
        format!(
            "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=43200{}",
            if config.frontend_origin.starts_with("https://") {
                "; Secure"
            } else {
                ""
            }
        ),
    );
    response
}

fn sign_out(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let Some(token) = request
        .cookies
        .get(COOKIE)
        .filter(|value| !value.is_empty())
    else {
        return error(
            401,
            "api-unauthorized",
            "Forge administrator session is required",
        );
    };
    match global::session_valid(db_path, token) {
        Ok(true) => {}
        Ok(false) => {
            return error(
                401,
                "api-unauthorized",
                "Forge administrator session is required",
            )
        }
        Err(_) => return unavailable(),
    }
    match global::revoke(db_path, token) {
        Ok(()) => {
            let mut response = ApiResponse::json(
                200,
                json!({ "authenticated": false, "contract": API_CONTRACT_VERSION }),
            );
            response.headers.insert(
                "set-cookie".to_string(),
                format!(
                    "{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
                    if config.frontend_origin.starts_with("https://") {
                        "; Secure"
                    } else {
                        ""
                    }
                ),
            );
            response
        }
        Err(_) => unavailable(),
    }
}

fn projects(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let token = request
        .cookies
        .get(COOKIE)
        .map(String::as_str)
        .unwrap_or("");
    match global::session_valid(db_path, token) {
        Ok(true) => {}
        Ok(false) => {
            return error(
                401,
                "api-unauthorized",
                "Forge administrator session is required",
            )
        }
        Err(_) => return unavailable(),
    }
    match super::fleet::load(db_path) {
        Ok(envelope) => ApiResponse::json(200, envelope),
        Err(_) => unavailable(),
    }
}

/// `GET /v1/admin/commands`: the typed CLI command catalog. Same global
/// session gate as [`projects`]; the body is static metadata only — the
/// catalog describes commands and never executes anything, and the API
/// exposes no shell/eval route anywhere.
fn commands(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let token = request
        .cookies
        .get(COOKIE)
        .map(String::as_str)
        .unwrap_or("");
    match global::session_valid(db_path, token) {
        Ok(true) => {}
        Ok(false) => {
            return error(
                401,
                "api-unauthorized",
                "Forge administrator session is required",
            )
        }
        Err(_) => return unavailable(),
    }
    ApiResponse::json(200, super::command_catalog::envelope())
}

fn allowed_origin(config: &ApiConfig, origin: Option<&str>) -> bool {
    origin == Some(config.frontend_origin.as_str())
}

fn cors(config: &ApiConfig, request: &ApiRequest, mut response: ApiResponse) -> ApiResponse {
    if allowed_origin(config, request.header("origin")) {
        response.headers.extend(cors_headers(config, request));
    }
    response
}

fn cors_headers(
    config: &ApiConfig,
    _request: &ApiRequest,
) -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::from([
        (
            "access-control-allow-origin".to_string(),
            config.frontend_origin.clone(),
        ),
        (
            "access-control-allow-credentials".to_string(),
            "true".to_string(),
        ),
        (
            "access-control-allow-methods".to_string(),
            "GET, POST, DELETE, OPTIONS".to_string(),
        ),
        (
            "access-control-allow-headers".to_string(),
            "Content-Type, Idempotency-Key".to_string(),
        ),
        ("vary".to_string(), "Origin".to_string()),
    ])
}

fn error(status: u16, code: &str, message: &str) -> ApiResponse {
    ApiResponse::json(
        status,
        json!({ "error": { "code": code, "message": message }, "contract": API_CONTRACT_VERSION }),
    )
}

fn unavailable() -> ApiResponse {
    error(
        503,
        "admin-api-unavailable",
        "Forge administrator service is unavailable",
    )
}
