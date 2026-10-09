//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{ApiConfig, ApiRequest, ApiResponse, API_CONTRACT_VERSION};
use crate::core::manifest::Manifest;
use crate::policy::DriftWatchConfig;
use crate::release::{
    engine as release_engine, release_config_from_manifest, ReleaseAdapterConfig, ReleaseReport,
    ReleaseRequest, Semver,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use super::deploy::{authoring_digest, deploy_id_gate, parse_release_version};
use super::gateway::{cors, error, guarded, is_json, scrub_json, scrub_response, scrub_text};

/// Extract the `version` query parameter for the read-only plan route, decoded
/// like every other browser query value. Used only as a semver string by Core,
/// never as a path, stage list or argument.
fn release_version_from_query(query: Option<&str>) -> Option<String> {
    for pair in query?.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == "version" {
            let decoded = super::super::percent_decode(value);
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
pub(super) fn release_plan(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
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
pub(super) fn release_write(
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
            match super::super::run_with_operation(
                db_path,
                "release",
                id,
                req,
                |op_id, _registry| {
                    let report = release_engine::apply_release(
                        &project_dir,
                        &manifest,
                        &config_value,
                        &request_value,
                        &adapters,
                    )?;
                    let view = scrub_json(release_report_view(&report), &secrets);
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
