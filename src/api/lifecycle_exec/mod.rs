//! Web lifecycle execution (`web-lifecycle-execution/0.1.0`).
//!
//! Session-gated, preview + confirm/digest-bound typed routes that let the
//! workbench walk idea → maintain in the browser through the same
//! in-process Core functions the CLI runs. No shell, no argv, no
//! browser-supplied path: every destination/target is resolved
//! server-side from a validated id under `FORGE_ADMIN_PROJECTS_ROOT` or
//! from the registry-resolved project directory.
//!
//! Routes (all `application/json`, all session-gated via `guarded`):
//! - `POST /v1/admin/graduation/preview` — validate artifact text, no write.
//! - `POST /v1/admin/graduation/import` — digest-bound adopt, journaled.
//! - `POST /v1/admin/projects/{id}/intent/resolve` — plan preview, no receipt.
//! - `POST /v1/admin/projects/{id}/intent/apply` — digest-bound apply, journaled.
//! - `POST /v1/admin/projects/{id}/remediate/plan` — plan preview, no write.
//! - `POST /v1/admin/projects/{id}/remediate/apply` — digest-bound apply, journaled.
//! - `POST /v1/admin/projects/{id}/delivery/next-idea` — journal the loop transition.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::ApiResponse;
use crate::registry::Registry;

pub const ROUTE_ADMIN_GRADUATION_PREVIEW: &str = "POST /v1/admin/graduation/preview";
pub const ROUTE_ADMIN_GRADUATION_IMPORT: &str = "POST /v1/admin/graduation/import";
pub const ROUTE_ADMIN_INTENT_RESOLVE: &str = "POST /v1/admin/projects/{id}/intent/resolve";
pub const ROUTE_ADMIN_INTENT_APPLY: &str = "POST /v1/admin/projects/{id}/intent/apply";
pub const ROUTE_ADMIN_REMEDIATE_PLAN: &str = "POST /v1/admin/projects/{id}/remediate/plan";
pub const ROUTE_ADMIN_REMEDIATE_APPLY: &str = "POST /v1/admin/projects/{id}/remediate/apply";
pub const ROUTE_ADMIN_DELIVERY_NEXT_IDEA: &str = "POST /v1/admin/projects/{id}/delivery/next-idea";
pub const ROUTE_ADMIN_STUDIO_SPEC_SAVE: &str = "POST /v1/admin/projects/{id}/studio/spec-save";
pub const ROUTE_ADMIN_STUDIO_REFINE: &str = "POST /v1/admin/projects/{id}/studio/refine-admin";

pub const CONTRACT: &str = "web-lifecycle-execution/0.1.0";

fn confirm_note() -> Value {
    json!({
        "requires": ["confirm", "plan_digest"],
        "note": "This preview writes nothing. To run the action, send `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused and nothing is written.",
    })
}

fn digest_mismatch() -> Value {
    json!({
        "code": "lifecycle-digest-mismatch",
        "message": "the confirmed digest does not match the current preview; nothing was written. Review the refreshed preview and confirm its new digest.",
    })
}

fn confirm_required(action: &str) -> ApiResponse {
    ApiResponse::json(
        409,
        json!({
            "error": { "code": "lifecycle-confirm-required", "message": format!("{action} requires `confirm: true`; refusing implicit project mutation.") },
            "effect": "none",
            "contract": CONTRACT,
        }),
    )
}

fn missing_digest() -> ApiResponse {
    ApiResponse::json(
        400,
        json!({
            "error": { "code": "lifecycle-digest-required", "message": "this action requires the `plan_digest` of the exact preview you reviewed." },
            "effect": "none",
            "contract": CONTRACT,
        }),
    )
}

fn registry_unavailable() -> ApiResponse {
    ApiResponse::json(
        503,
        json!({
            "error": { "code": "lifecycle-unavailable", "message": "the registry could not be opened; nothing was changed." },
            "effect": "none",
            "contract": CONTRACT,
        }),
    )
}

fn record_journal(db_path: &Path, kind: &str, project_id: &str, detail: &str) {
    if let Ok(registry) = Registry::open(db_path) {
        let safe: String = detail
            .split_whitespace()
            .map(|t| {
                if t.starts_with('/') || t.contains(":\\") || t.contains("=/") {
                    "[local path]"
                } else {
                    t
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let _ = registry.record_operation(kind, project_id, "done", &safe);
    }
}

fn latest_op_id(db_path: &Path, kind: &str, project_id: &str) -> Option<i64> {
    Registry::open(db_path)
        .ok()?
        .journal_entries()
        .ok()?
        .iter()
        .rev()
        .find(|e| e.kind == kind && e.project_id == project_id)
        .map(|e| e.op_id)
}

fn projects_root_response() -> Result<PathBuf, ApiResponse> {
    crate::api::project_management::projects_root_public()
}

fn sha_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(out.len() * 2);
    for b in out {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn resolve_project_dir(db_path: &Path, id: &str) -> Result<(String, PathBuf), ApiResponse> {
    if crate::core::validate_project_id(id).is_err() {
        return Err(ApiResponse::json(
            400,
            json!({
                "error": { "code": "lifecycle-invalid", "message": "the project id is not a valid identifier; it may not contain a path." },
                "effect": "none",
                "contract": CONTRACT,
            }),
        ));
    }
    let registry = Registry::open(db_path).map_err(|_| registry_unavailable())?;
    let record = registry.inspect(id).map_err(|_| {
        ApiResponse::json(
            404,
            json!({
                "error": { "code": "lifecycle-unmanaged", "message": "this project is not managed by this Forge registry; nothing was changed." },
                "effect": "none",
                "contract": CONTRACT,
            }),
        )
    })?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ApiResponse::json(
            404,
            json!({
                "error": { "code": "path-unavailable", "message": "the registered project directory is not currently available on this host; nothing was changed." },
                "effect": "none",
                "contract": CONTRACT,
            }),
        ));
    }
    Ok((record.id, dir))
}

pub mod delivery;
pub mod graduation;
pub mod intent;
pub mod remediate;
pub mod studio;

pub use delivery::delivery_next_idea;
pub use graduation::{graduation_import, graduation_preview};
pub use intent::{intent_apply, intent_resolve};
pub use remediate::{remediate_apply, remediate_plan};
pub use studio::{studio_refine, studio_spec_save};
