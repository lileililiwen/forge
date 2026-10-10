//! Confirm-gated Forge-owned portfolio writes (`web-portfolio-completion`).
//!
//! Verbatim move of the write half of `super` along the module's own
//! read/write seam (see `portfolio-mod-size-split` for the precedent):
//! every mutation requires `confirm: true` (preview → confirm →
//! apply), refusals echo the current-state preview with `effect:
//! "none"`, and imported evidence stays append-only. No contract
//! string, JSON shape or behavior changes in this move.

use std::path::Path;

use chrono::Utc;
use serde_json::{json, Value};

use crate::api::ApiResponse;
use crate::registry::Registry;

use super::{
    open, optional_str, refuse, require_id, required_str, scrub, scrub_value, typed_refusal,
    CONTRACT_VERSION, WRITE_ACTIONS,
};

// --- write: Forge-owned metadata -----------------------------------------

/// `POST /v1/admin/portfolio/{id}/{action}` — a Forge-owned metadata
/// mutation (`tags`, `relations`, `reviews`, `goals`). Each maps to one
/// typed Core write; nothing is executed, and no source-owned snapshot is
/// touched. Every action requires `confirm: true` (preview → confirm →
/// apply): a missing or false confirm is a `409
/// portfolio-confirm-required` carrying the current-state preview of the
/// exact sub-resource reviewed and `effect: "none"`. There is no manifest
/// digest to bind here — each mutation targets one typed row, so the
/// confirm binds to the reviewed `(project_id, key)` plus the echoed
/// preview instead (see the change design). The `evidence` action is the
/// honest refusal path: imported observations are append-only and cannot
/// be edited from the browser, so the snapshot is preserved unchanged.
pub fn write_item(db_path: &Path, id: &str, action: &str, body: &Value) -> ApiResponse {
    if !WRITE_ACTIONS.contains(&action) {
        return refuse(
            404,
            "portfolio-route-not-found",
            "no portfolio action matches this path",
        );
    }
    if action == "evidence" {
        // Source-owned evidence is never editable through the browser: refuse
        // before opening a write, so the snapshot provably stays unchanged.
        // Append-only imports arrive via `import_evidence` instead.
        return refuse(
            403,
            "portfolio-source-owned",
            "imported evidence is source-owned and append-only; the browser cannot edit it and the stored snapshot was left unchanged.",
        );
    }

    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    if let Err(response) = require_id(&registry, id) {
        return response;
    }
    if !is_confirmed(body) {
        return confirm_required(&registry, id, action);
    }

    let outcome = match action {
        "tags" => {
            let name = match required_str(body, "name") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let color = optional_str(body, "color").map(|value| value.to_string());
            registry
                .portfolio_add_tag(id, &name, color.as_deref())
                .map(|tag| json!({ "tag": tag }))
        }
        "relations" => {
            let to = match required_str(body, "to") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let raw_type = match required_str(body, "type") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let relation_type = match crate::portfolio::RelationType::parse(&raw_type) {
                Ok(value) => value,
                Err(reason) => return refuse(400, "portfolio-invalid", &reason),
            };
            let note = optional_str(body, "note").map(|value| value.to_string());
            registry
                .portfolio_add_relation(id, &to, relation_type, note.as_deref())
                .map(|relation| json!({ "relation": relation }))
        }
        "reviews" => {
            let raw_confidence = match required_str(body, "confidence") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let confidence = match crate::portfolio::Confidence::parse(&raw_confidence) {
                Ok(value) => value,
                Err(reason) => return refuse(400, "portfolio-invalid", &reason),
            };
            let lifecycle = match optional_str(body, "lifecycle") {
                Some(raw) => match crate::portfolio::Lifecycle::parse(raw) {
                    Ok(value) => Some(value),
                    Err(reason) => return refuse(400, "portfolio-invalid", &reason),
                },
                None => None,
            };
            let note = optional_str(body, "note").map(|value| value.to_string());
            let next_action = optional_str(body, "next_action").map(|value| value.to_string());
            let blocker = optional_str(body, "blocker").map(|value| value.to_string());
            let write = crate::registry::PortfolioWrite {
                lifecycle,
                confidence: Some(confidence),
                next_action,
                blocker,
            };
            registry
                .portfolio_write(id, &write)
                .and_then(|profile| {
                    registry
                        .portfolio_record_review(id, confidence, note.as_deref())
                        .map(|review| (profile, review))
                })
                .map(|(profile, review)| json!({ "profile": profile, "review": review }))
        }
        "goals" => {
            let title = match required_str(body, "title") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let raw_status = match required_str(body, "status") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            // Validate the status before touching the registry so a bad
            // status changes nothing (linking a goal would otherwise create
            // it with a forced status first).
            if let Err(reason) = crate::portfolio::validate_goal_status(&raw_status) {
                return refuse(400, "portfolio-invalid", &reason);
            }
            let description = optional_str(body, "description").map(|value| value.to_string());
            registry
                .portfolio_link_goal(&title, id)
                .and_then(|linked| {
                    // Re-stamp the operator's requested status/description
                    // onto the goal, then read it back with its membership.
                    registry.portfolio_add_goal(&title, &raw_status, description.as_deref())?;
                    registry.portfolio_goal(linked.goal_id)
                })
                .map(|goal| json!({ "goal": goal }))
        }
        other => unreachable!("action {other} is not in WRITE_ACTIONS"),
    };

    match outcome {
        Ok(data) => scrub(json!({
            "contract": CONTRACT_VERSION,
            "project_id": id,
            "action": action,
            "effect": "forge-owned-write",
            "actor": "global-admin",
            "recorded_at": Utc::now().to_rfc3339(),
            "result": data,
        })),
        Err(err) => typed_refusal(&err),
    }
}

// --- write: removals and append-only evidence import ------------------------
// These three routes follow the delivery `allowlist/{id}/remove`
// precedent: fixed path segments (never a browser-supplied path),
// id-only addressing, `confirm: true` required, idempotent results.

/// `POST /v1/admin/portfolio/{id}/tags/remove` — detach one tag.
/// Idempotent: removing an absent tag reports `removed: false` with
/// 200 and changes nothing.
pub fn remove_tag(db_path: &Path, id: &str, body: &Value) -> ApiResponse {
    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    if let Err(response) = require_id(&registry, id) {
        return response;
    }
    if !is_confirmed(body) {
        return confirm_required(&registry, id, "tags");
    }
    let name = match required_str(body, "name") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    match registry.portfolio_remove_tag(id, &name) {
        Ok(removed) => scrub(json!({
            "contract": CONTRACT_VERSION,
            "project_id": id,
            "action": "tags-remove",
            "effect": "forge-owned-write",
            "actor": "global-admin",
            "recorded_at": Utc::now().to_rfc3339(),
            "result": { "removed": removed },
        })),
        Err(err) => typed_refusal(&err),
    }
}

/// `POST /v1/admin/portfolio/{id}/relations/remove` — withdraw one
/// declared relation. Idempotent: removing an absent relation reports
/// `removed: false` with 200 and changes nothing.
pub fn remove_relation(db_path: &Path, id: &str, body: &Value) -> ApiResponse {
    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    if let Err(response) = require_id(&registry, id) {
        return response;
    }
    if !is_confirmed(body) {
        return confirm_required(&registry, id, "relations");
    }
    let to = match required_str(body, "to") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let raw_type = match required_str(body, "type") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let relation_type = match crate::portfolio::RelationType::parse(&raw_type) {
        Ok(value) => value,
        Err(reason) => return refuse(400, "portfolio-invalid", &reason),
    };
    match registry.portfolio_remove_relation(id, &to, relation_type) {
        Ok(removed) => scrub(json!({
            "contract": CONTRACT_VERSION,
            "project_id": id,
            "action": "relations-remove",
            "effect": "forge-owned-write",
            "actor": "global-admin",
            "recorded_at": Utc::now().to_rfc3339(),
            "result": { "removed": removed },
        })),
        Err(err) => typed_refusal(&err),
    }
}

/// `POST /v1/admin/portfolio/{id}/evidence/import` — append one
/// source-owned observation. There is no update or delete path: a
/// later observation is a new row, and the read model shows the
/// newest row per source. The payload is validated, redacted and
/// bounded by Core before it is written; an unknown `status` is a
/// typed 400 with nothing stored. Imported rows stay read-only:
/// this route never edits an existing snapshot.
pub fn import_evidence(db_path: &Path, id: &str, body: &Value) -> ApiResponse {
    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    if let Err(response) = require_id(&registry, id) {
        return response;
    }
    if !is_confirmed(body) {
        return confirm_required(&registry, id, "evidence");
    }
    let source_system = match required_str(body, "source_system") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let source_revision = match required_str(body, "source_revision") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let raw_status = match required_str(body, "status") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let status = match crate::portfolio::EvidenceStatus::parse(raw_status.trim()) {
        Ok(value) => value,
        Err(reason) => return refuse(400, "portfolio-invalid", &reason),
    };
    let observed_at = optional_str(body, "observed_at")
        .map(|value| value.to_string())
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let stale_after = optional_str(body, "stale_after").map(|value| value.to_string());
    let evidence_json = match body.get("evidence") {
        None => "{}".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => serde_json::to_string(other).unwrap_or_else(|_| "{}".to_string()),
    };
    let write = crate::registry::SnapshotWrite {
        source_system,
        source_revision,
        observed_at,
        status,
        stale_after,
        evidence_json,
    };
    match registry.portfolio_import_snapshot(id, &write) {
        Ok(snapshot) => scrub(json!({
            "contract": CONTRACT_VERSION,
            "project_id": id,
            "action": "evidence-import",
            "effect": "forge-owned-write",
            "actor": "global-admin",
            "recorded_at": Utc::now().to_rfc3339(),
            "result": { "snapshot": snapshot },
        })),
        Err(err) => typed_refusal(&err),
    }
}

// --- write: confirm gate ----------------------------------------------------

/// The confirm gate every portfolio mutation runs through (the
/// workbench/delivery `confirm: true` pattern, minus the manifest
/// digest — portfolio has no manifest to hash, so the confirm binds
/// to the reviewed `(project_id, key)` plus the echoed preview).
fn is_confirmed(body: &Value) -> bool {
    body.get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Refuse an unconfirmed mutation with the current-state preview of
/// the exact sub-resource the operator reviewed. Nothing is written.
fn confirm_required(registry: &Registry, id: &str, action: &str) -> ApiResponse {
    let preview = current_preview(registry, id, action);
    ApiResponse::json(
        409,
        scrub_value(json!({
            "contract": CONTRACT_VERSION,
            "error": {
                "code": "portfolio-confirm-required",
                "message": "this mutation requires `confirm: true`; refusing implicit portfolio mutation. Review the preview and confirm it.",
            },
            "effect": "none",
            "preview": preview,
            "confirmation": {
                "requires": ["confirm"],
                "note": "This preview writes nothing. Confirm by sending `confirm: true` with the same fields; a missing or false confirm changes nothing.",
            },
        })),
    )
}

/// The current rows of the sub-resource a write action targets, echoed
/// back on confirm refusal so the operator reviews exact state.
fn current_preview(registry: &Registry, id: &str, action: &str) -> Value {
    match action {
        "tags" => registry
            .portfolio_tags_for(id)
            .ok()
            .and_then(|tags| serde_json::to_value(&tags).ok())
            .unwrap_or(Value::Null),
        "relations" => registry
            .portfolio_relations_for(id)
            .ok()
            .and_then(|relations| serde_json::to_value(&relations).ok())
            .unwrap_or(Value::Null),
        "reviews" => registry
            .portfolio_reviews_for(id, 50)
            .ok()
            .and_then(|reviews| serde_json::to_value(&reviews).ok())
            .unwrap_or(Value::Null),
        "goals" => registry
            .portfolio_goals()
            .ok()
            .map(|goals| {
                goals
                    .into_iter()
                    .filter(|goal| goal.projects.iter().any(|p| p == id))
                    .collect::<Vec<_>>()
            })
            .and_then(|goals| serde_json::to_value(&goals).ok())
            .unwrap_or(Value::Null),
        "evidence" => registry
            .portfolio_current_snapshots(id)
            .ok()
            .map(|snapshots| {
                let now = Utc::now();
                snapshots
                    .iter()
                    .map(|snapshot| {
                        let status = crate::portfolio::effective_status(
                            snapshot.status,
                            snapshot.stale_after.as_deref(),
                            now,
                        );
                        json!({
                            "source_system": snapshot.source_system,
                            "source_revision": snapshot.source_revision,
                            "observed_at": snapshot.observed_at,
                            "status": status.label(),
                            "editable": false,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .and_then(|items| serde_json::to_value(&items).ok())
            .unwrap_or(Value::Null),
        _ => Value::Null,
    }
}
