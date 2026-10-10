//! Read-only shipping/provider reads (`web-shipping-provider-reads`).
//!
//! Five session-gated GET handlers over the local-config reads the CLI
//! already serves: `publish::providers::load_config` (publish provider
//! list + inspect), `provider::matrix(false)` (evidence matrix, never
//! live) + `provider::inspect` (static descriptor, never probed), and
//! `plugins::list` over the same config plus its sibling `plugins:`
//! block. No adapter invocation, no probe, no network, no write, no
//! journal row, no browser-supplied path on any path; every response
//! carries the admin contract version and project-bound bodies are
//! scrubbed of the absolute project directory. The CLI's provider-matrix
//! journal line is deliberately not reproduced, and `enable|disable`
//! stays CLI-only.

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use super::deploy::deploy_id_gate;
use super::gateway::{cors, error, guarded, scrub_json, scrub_text};
use super::routes::PUBLISH_PROVIDER_CONFIG_ENV;

/// The `forge plugins` contract version, mirroring the CLI's
/// `PLUGINS_CONTRACT` (`src/cli/constants.rs`, binary-only so it cannot
/// be imported here). Kept as a literal beside the projection it labels
/// so the two can never silently diverge without this file changing too.
const PLUGINS_CONTRACT: &str = "forge-plugins/0.1.0";

/// Section keys served by this module on the shared `Route::AdminCreation`
/// triple. Disjoint from every creation (`profiles`, `features`,
/// `components`, `ui-patterns`, `standards`, `procedures`, `intents`,
/// `project-standard`, `project-intent`) and assurance (`contracts`,
/// `specs`, `spec`, `remediate`, `describe`, `classify`, `governance`,
/// `analytics`, `studio`) key, so the three matchers never claim the
/// same shape.
const SECTIONS: &[&str] = &[
    "shipping-providers",
    "evidence-providers",
    "shipping-plugins",
];

/// True for the three shipping sections; anything else belongs to the
/// assurance or creation catalogs (or to no beside-table route at all).
pub(in crate::api) fn is_shipping_registry(registry: &str) -> bool {
    SECTIONS.contains(&registry)
}

/// Match the five read-only shipping routes before the main table runs.
/// The literals `publish/providers`, `providers/matrix`,
/// `providers/{provider}` and `plugins` never collide with the existing
/// `feature`/`spec`/`deploy`/`release`/`publish`/`delivery`/`agents`/
/// `identity`/`releases`/`deploys`/`catalog`/`standard`/`intent`/
fn list_route(section: &str, item: &str, action: &str) -> Route {
    Route::AdminCreation {
        registry: section.to_string(),
        item: item.to_string(),
        action: action.to_string(),
    }
}

/// `creation`/`contracts` arms, so trying these shapes first shadows no
/// existing route. Kept beside the handlers (not in `router.rs`) so the
/// route table stays under the source-file-size cap. The `OPTIONS` arm
/// keeps the browser preflight beside the GETs it covers.
pub(in crate::api) fn route_shipping_reads(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "projects", id, "publish", "providers"]) => {
            Some(list_route("shipping-providers", id, "list"))
        }
        ("GET", ["v1", "admin", "projects", id, "publish", "providers", provider]) => Some(
            list_route("shipping-providers", &format!("{id}/{provider}"), "inspect"),
        ),
        ("GET", ["v1", "admin", "providers", "matrix"]) => {
            Some(list_route("evidence-providers", "", "matrix"))
        }
        ("GET", ["v1", "admin", "providers", provider]) => {
            Some(list_route("evidence-providers", provider, "inspect"))
        }
        ("GET", ["v1", "admin", "projects", id, "plugins"]) => {
            Some(list_route("shipping-plugins", id, "list"))
        }
        ("OPTIONS", ["v1", "admin", "providers", ..]) => Some(Route::AdminOptions),
        ("OPTIONS", ["v1", "admin", "projects", ..]) => None,
        _ => None,
    }
}

/// Validate one `{provider}` path segment before any store lookup. A
/// blank value, a path separator (raw or percent-encoded), a backslash,
/// a `..` traversal or any other percent-encoding is a static typed
/// `400` that never echoes the offending input. Provider ids never
/// contain those bytes.
fn shipping_id_gate(raw: &str) -> Result<String, ApiResponse> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains('%')
    {
        return Err(error(
            400,
            "admin-invalid-shipping-id",
            "that provider id is not valid; use the id shown in the provider list.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Split a `"{project}/{provider}"` item back into its validated halves.
/// Both halves pass the same gate; a malformed item is a static `404`
/// (the matcher built it, so this is unreachable in practice).
fn split_item(item: &str) -> Result<(String, String), ApiResponse> {
    match item.split_once('/') {
        Some((project, provider)) => Ok((shipping_id_gate(project)?, shipping_id_gate(provider)?)),
        None => Err(error(
            404,
            "admin-unknown-shipping",
            "no shipping route matches the request",
        )),
    }
}

/// Shipping status mapping: the shared table plus the reachable
/// shipping user errors, which the shared table would report as `500`.
/// Kept beside the handlers (not in `err_status`) so `router.rs` stays
/// under the source-file-size cap.
pub(in crate::api) fn shipping_error_status(err: &crate::core::ForgeError) -> u16 {
    match err.code() {
        "publish-invalid" | "provider-invalid" => 400,
        _ => super::super::err_status(err),
    }
}

fn typed_shipping_error(err: &crate::core::ForgeError) -> ApiResponse {
    ApiResponse::json(
        shipping_error_status(err),
        json!({
            "error": { "code": err.code(), "message": err.to_string() },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

fn typed_project_shipping_error(project_dir: &Path, err: &crate::core::ForgeError) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    ApiResponse::json(
        shipping_error_status(err),
        json!({
            "error": {
                "code": err.code(),
                "message": scrub_text(&err.to_string(), &secrets),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// Resolve the publish provider config path exactly the way the publish
/// plan handler does — the `FORGE_PUBLISH_PROVIDER_CONFIG` server
/// environment override, else the project's `.forge/providers.yaml` —
/// without the plan handler's provider-id and revision requirements.
/// The browser never names a path.
fn shipping_config_path(project_dir: &Path) -> PathBuf {
    std::env::var_os(PUBLISH_PROVIDER_CONFIG_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"))
}

/// Honest `200 unavailable-with-reason` for a missing provider config.
/// A project with no `.forge/providers.yaml` simply configures nothing;
/// that is a real answer, never a 500. The absolute path is never
/// echoed.
fn missing_config(id: &str) -> ApiResponse {
    ApiResponse::json(
        200,
        json!({
            "project": id,
            "unavailable": true,
            "reason": "this project has no publish provider configuration yet; add `.forge/providers.yaml` in a terminal via `forge publish provider`, then reload.",
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// Dispatch one shipping read. The `(registry, action)` pair is
/// re-validated here so only the matcher's five combos run; anything
/// else is a static `404`, never a guess.
pub(in crate::api) fn dispatch(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    section: &str,
    item: &str,
    action: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| match (section, action) {
            ("shipping-providers", "list") => match deploy_id_gate(db_path, item) {
                Ok(dir) => publish_providers_list(&dir, item),
                Err(response) => response,
            },
            ("shipping-providers", "inspect") => match split_item(item) {
                Ok((project, provider)) => match deploy_id_gate(db_path, &project) {
                    Ok(dir) => publish_provider_inspect(&dir, &project, &provider),
                    Err(response) => response,
                },
                Err(response) => response,
            },
            ("evidence-providers", "matrix") => evidence_matrix(),
            ("evidence-providers", "inspect") => match shipping_id_gate(item) {
                Ok(provider) => evidence_provider_inspect(&provider),
                Err(response) => response,
            },
            ("shipping-plugins", "list") => match deploy_id_gate(db_path, item) {
                Ok(dir) => shipping_plugins_list(&dir, item),
                Err(response) => response,
            },
            _ => error(
                404,
                "admin-unknown-shipping",
                "no shipping route matches the request",
            ),
        }),
    )
}

/// `GET /v1/admin/projects/{id}/publish/providers`: every configured
/// publish provider — the same entries as `forge publish provider list`.
/// A missing config answers honest `unavailable-with-reason`.
fn publish_providers_list(project_dir: &Path, id: &str) -> ApiResponse {
    let path = shipping_config_path(project_dir);
    if !path.is_file() {
        return missing_config(id);
    }
    let secrets = [project_dir.display().to_string()];
    match crate::publish::providers::load_config(&path) {
        Ok(config) => ApiResponse::json(
            200,
            scrub_json(
                json!({
                    "project": id,
                    "providers": config.providers,
                    "contract": API_CONTRACT_VERSION,
                }),
                &secrets,
            ),
        ),
        Err(err) => typed_project_shipping_error(project_dir, &err),
    }
}

/// `GET .../publish/providers/{provider}`: one configured provider —
/// the same entry as `forge publish provider inspect`. Never invokes
/// it. An unknown id is a typed `404`.
fn publish_provider_inspect(project_dir: &Path, id: &str, provider: &str) -> ApiResponse {
    let path = shipping_config_path(project_dir);
    if !path.is_file() {
        return missing_config(id);
    }
    let secrets = [project_dir.display().to_string()];
    match crate::publish::providers::load_config(&path) {
        Ok(config) => match config.providers.iter().find(|entry| entry.id == provider) {
            Some(entry) => ApiResponse::json(
                200,
                scrub_json(
                    json!({
                        "project": id,
                        "provider": entry,
                        "contract": API_CONTRACT_VERSION,
                    }),
                    &secrets,
                ),
            ),
            None => error(
                404,
                "admin-unknown-provider",
                "no publish provider with that id is configured for this project.",
            ),
        },
        Err(err) => typed_project_shipping_error(project_dir, &err),
    }
}

/// `GET /v1/admin/providers/matrix`: the evidence-provider matrix with
/// `live` hard to false — every row is `not-run`, no binary is probed,
/// no journal row is written. The CLI's journal line is deliberately
/// not reproduced.
fn evidence_matrix() -> ApiResponse {
    let report = crate::provider::matrix(false);
    ApiResponse::json(
        200,
        json!({
            "matrix": report,
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// `GET /v1/admin/providers/{provider}`: one provider's static boundary
/// descriptor — the same record as `forge provider inspect`. Performs no
/// probe and contacts no provider. An unknown id answers the Core typed
/// error without echoing the input.
fn evidence_provider_inspect(provider: &str) -> ApiResponse {
    match crate::provider::inspect(provider) {
        Ok(descriptor) => ApiResponse::json(
            200,
            json!({
                "descriptor": descriptor,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => {
            if err.code() == "provider-invalid" {
                return error(
                    404,
                    "admin-unknown-provider",
                    "no evidence provider with that id exists; use the id shown in the matrix.",
                );
            }
            typed_shipping_error(&err)
        }
    }
}

/// `GET /v1/admin/projects/{id}/plugins`: every configured plugin —
/// the same records as `forge plugins list --format json`. A missing
/// config answers an honest empty registry, mirroring the CLI.
fn shipping_plugins_list(project_dir: &Path, id: &str) -> ApiResponse {
    let path = shipping_config_path(project_dir);
    if !path.is_file() {
        let secrets = [project_dir.display().to_string()];
        return ApiResponse::json(
            200,
            scrub_json(
                json!({
                    "project": id,
                    "plugins": [],
                    "contract": PLUGINS_CONTRACT,
                }),
                &secrets,
            ),
        );
    }
    let secrets = [project_dir.display().to_string()];
    let provider_config = match crate::publish::providers::load_config(&path) {
        Ok(config) => config,
        Err(err) => return typed_project_shipping_error(project_dir, &err),
    };
    let descriptors = match crate::plugins::load_descriptors(&path) {
        Ok(descriptors) => descriptors,
        Err(err) => return typed_project_shipping_error(project_dir, &err),
    };
    let records = crate::plugins::list(&provider_config, &descriptors);
    let views: Vec<Value> = records
        .iter()
        .map(|record| {
            json!({
                "id": record.id,
                "kind": record.kind.as_str(),
                "enabled": record.enabled,
                "state": record.state.as_str(),
                "reason": record.state.reason(),
                "command": record.command.to_string_lossy(),
                "capabilities": record.capability_list(),
                "description": record.description,
            })
        })
        .collect();
    ApiResponse::json(
        200,
        scrub_json(
            json!({
                "project": id,
                "plugins": views,
                "contract": PLUGINS_CONTRACT,
            }),
            &secrets,
        ),
    )
}
