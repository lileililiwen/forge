//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{ApiConfig, ApiRequest, ApiResponse, API_CONTRACT_VERSION};
use crate::delivery::handlers as delivery_handlers;
use crate::publish::providers::{
    self as publish_providers, ProviderOperation, PublishProviderRequest, PUBLISH_PROVIDER_CONTRACT,
};
use crate::registry::{PublishPhaseEvidence, Registry};
use chrono::Utc;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use super::deploy::{authoring_digest, deploy_id_gate};
use super::gateway::{
    cors, error, guarded, is_json, scrub_json, scrub_response, scrub_text, typed_delivery_error,
    unavailable,
};
use super::model::{DeliveryAction, DeliveryConfirmation, DeliveryResolution, PublishTarget};
use super::routes::{PUBLISH_PROVIDER_CONFIG_ENV, PUBLISH_PROVIDER_ENV};

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
pub(super) fn publish_plan(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
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
pub(super) fn publish_write(
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

/// Resolve a managed project id to its server-side directory and registered
/// revision. A missing revision is a typed prerequisite because delivery
/// idempotency and promotion are revision-bound.
pub(super) fn delivery_resolution(
    db_path: &Path,
    id: &str,
) -> Result<DeliveryResolution, ApiResponse> {
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
pub(super) fn delivery_confirmation(
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
pub(super) fn delivery_report(
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
