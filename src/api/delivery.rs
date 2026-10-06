//! forge — session-gated, confirm- and digest-bound delivery controls
//! (`forge-web-delivery-controls/0.1.0`).
//!
//! This module turns the portfolio **share allowlist → preview → approve →
//! publish** pipeline — the flow the portfolio-controls package deliberately
//! deferred here — into typed browser workflows, plus publication status and
//! reconciliation of ambiguous outcomes. Every mutation reuses the crate's
//! own typed in-process Core functions
//! ([`crate::portfolio::publication`], the registry share audit and
//! [`crate::provider::matrix`]); nothing here is a rename of a shell command.
//!
//! Security boundary (see `.ai-rules/concerns/security.md`):
//! - Nothing here ever runs a shell, `sh -c`, an interpreter, a Git
//!   executable or a publish adapter subprocess. The subprocess adapter
//!   publisher stays CLI-only and is never reachable from these routes:
//!   every browser publication dispatches with `adapter: None`, i.e. the
//!   default-safe local export only.
//! - The browser never supplies a filesystem path. Projects are addressed
//!   only by a validated opaque `id`, the artifact target is read from the
//!   **server-side** environment (`FORGE_SHARE_PUBLISH_TARGET`), and an
//!   unset target is an honest typed prerequisite refusal — never a browser
//!   prompt for a path. Operation keys are validated Core tokens, never
//!   paths. Absolute paths are never serialized: the whole projection is
//!   scrubbed and a publication's recorded `target` is never echoed.
//! - Every mutating route requires `confirm: true` **and** a `plan_digest`
//!   equal to the freshly recomputed manifest hash of the exact state the
//!   operator reviewed (the workbench digest-binding pattern). A changed
//!   or stale digest is refused with `effect: "none"` and a fresh preview.
//! - External delivery effects are only ever claimed through Core's
//!   journaled audit trail (approvals and publication attempts). Success,
//!   failure, partial (`unknown`) and already-published outcomes are
//!   reported as recorded; an unreconciled attempt blocks a new
//!   publication until an operator reconciles it with its exact digest.
//! - Provider evidence is non-live: `matrix(false)` rows report their
//!   honest `not-run`/`disabled` state and no probe runs on page load.

use std::path::Path;

use chrono::Utc;
use serde_json::{json, Value};

use super::{ApiRequest, ApiResponse};
use crate::core::{validate_project_id, ForgeError};
use crate::portfolio::publication::{
    preview_manifest, publish_approved_manifest, PublishPlan, PublishReport,
};
use crate::portfolio::share::{
    self, PublicationStatus, ShareRecord, ShareSurface, ShareWrite, Visibility,
};
use crate::registry::Registry;

/// Versioned delivery-controls contract. Additive only.
pub const CONTRACT_VERSION: &str = "forge-web-delivery-controls/0.1.0";

/// Typed JSON routes this module implements — the honest `web`
/// destinations command-catalog rows in the share family point at.
pub const ROUTE_DELIVERY_OVERVIEW: &str = "GET /v1/admin/delivery";
pub const ROUTE_DELIVERY_PREVIEW: &str = "GET /v1/admin/delivery/preview";
pub const ROUTE_DELIVERY_ALLOWLIST: &str = "POST /v1/admin/delivery/allowlist/{id}";
pub const ROUTE_DELIVERY_ALLOWLIST_REMOVE: &str = "POST /v1/admin/delivery/allowlist/{id}/remove";
pub const ROUTE_DELIVERY_APPROVE: &str = "POST /v1/admin/delivery/approve";
pub const ROUTE_DELIVERY_PUBLISH: &str = "POST /v1/admin/delivery/publish";
pub const ROUTE_DELIVERY_RECONCILE: &str = "POST /v1/admin/delivery/reconcile";
pub const ROUTE_DELIVERY_OPERATION: &str = "GET /v1/admin/delivery/operation/{key}";

/// Server-side configuration of the local publication artifact path. The
/// browser never sends a path; an operator sets this in the API process
/// environment in a terminal, or publishes through the CLI directly.
pub const PUBLISH_TARGET_ENV: &str = "FORGE_SHARE_PUBLISH_TARGET";

/// Cookie carrying the global administrator session, shared with the other
/// `/v1/admin` surfaces; the actor recorded in the audit trail is resolved
/// from it, never from a browser-supplied field.
const SESSION_COOKIE: &str = "forge_admin_session";

/// Upper bound on the publication history echoed to the browser.
const MAX_PUBLICATIONS: usize = 10;
/// Upper bound on manifest findings echoed per preview.
const MAX_FINDINGS: usize = 20;

/// `GET /v1/admin/delivery` — the whole delivery surface in one honest
/// read: the allowlisted share records, the current manifest preview
/// (digest, approvable, findings), the newest approval and publication
/// trail, any unreconciled attempt, the server-side target state and the
/// non-live provider matrix. No probe, no write, no path.
pub fn overview(db_path: &Path) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let records = match registry.share_records() {
        Ok(records) => records.iter().map(project_record).collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    let preview = preview_block(&registry);
    let approval = match registry.share_latest_approval() {
        Ok(approval) => approval.as_ref().map(approval_block),
        Err(_) => None,
    };
    let publications = match registry.share_publications(MAX_PUBLICATIONS) {
        Ok(attempts) => attempts
            .iter()
            .map(publication_attempt_block)
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    let unreconciled = match registry.share_unreconciled_publication() {
        Ok(attempt) => attempt.as_ref().map(publication_attempt_block),
        Err(_) => None,
    };
    let provider = serde_json::to_value(crate::provider::matrix(false))
        .unwrap_or_else(|_| json!({"state": "unavailable"}));

    scrub(json!({
        "contract": CONTRACT_VERSION,
        "generated_at": Utc::now().to_rfc3339(),
        "live_probes": false,
        "allowlist": records,
        "preview": preview,
        "approval": approval,
        "publications": publications,
        "unreconciled": unreconciled,
        "target": target_block(),
        "adapter": {
            "web": false,
            "note": "Subprocess publication adapters are CLI-only by design; a browser dispatch always uses the default-safe local export.",
        },
        "provider": provider,
        "routes": {
            "overview": ROUTE_DELIVERY_OVERVIEW,
            "preview": ROUTE_DELIVERY_PREVIEW,
            "allowlist": ROUTE_DELIVERY_ALLOWLIST,
            "allowlist_remove": ROUTE_DELIVERY_ALLOWLIST_REMOVE,
            "approve": ROUTE_DELIVERY_APPROVE,
            "publish": ROUTE_DELIVERY_PUBLISH,
            "reconcile": ROUTE_DELIVERY_RECONCILE,
            "operation": ROUTE_DELIVERY_OPERATION,
        },
        "repository_operations": {
            "web": false,
            "note": "Commit, push, mirror, release, deploy, provider runs, GitHub and Studio operations keep their command-catalog disposition (cli_only / provider_required): they need native tools, provider credentials or interactive confirmation that the browser never carries. The catalog is the single honest source.",
        },
        "next_step": "Review the preview digest, then confirm mutations against it: allowlist changes, approvals and publications each require confirm plus this exact manifest digest.",
    }))
}

/// `GET /v1/admin/delivery/preview` — the side-effect-free plan every
/// mutation binds to: exact candidate manifest hash, revision, project
/// count, approvable state and findings. Reading it changes nothing.
pub fn preview(db_path: &Path) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let preview = preview_block(&registry);
    scrub(json!({
        "contract": CONTRACT_VERSION,
        "effect": "none",
        "preview": preview,
        "confirmation": {
            "requires": ["confirm", "plan_digest"],
            "note": "Every mutating delivery call must echo this exact plan_digest; a changed or stale digest is refused and nothing is dispatched.",
        },
    }))
}

/// `POST /v1/admin/delivery/allowlist/{id}` — put or replace one project's
/// share record (the allowlist entry that makes it a publish candidate).
/// Requires `confirm: true` and a `plan_digest` matching the manifest the
/// operator reviewed; a stale digest refuses with `effect: "none"`. Core's
/// `validate_share` remains the single record gate — the browser cannot
/// make a record public, only a confirmed candidate.
pub fn allowlist_set(db_path: &Path, id: &str, body: &Value, request: &ApiRequest) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    if let Err(response) = require_managed(&registry, id) {
        return response;
    }
    let digest_response = confirmed_digest(&registry, body, "allowlisting this project");
    let current_digest = match digest_response {
        Ok(digest) => digest,
        Err(response) => return response,
    };
    let write = match parse_share_write(body) {
        Ok(write) => write,
        Err(response) => return response,
    };
    match registry.share_upsert_record(id, &write) {
        Ok(record) => {
            let next_preview = preview_block(&registry);
            let next_digest = next_preview
                .as_ref()
                .and_then(|p| p["manifest_sha256"].as_str())
                .unwrap_or("")
                .to_string();
            scrub(json!({
                "contract": CONTRACT_VERSION,
                "effect": "forge-owned-write",
                "actor": actor_for(db_path, request),
                "project_id": record.project_id,
                "record": project_record(&record),
                "reviewed_digest_consumed": current_digest,
                "preview": next_preview,
                "note": if next_digest == current_digest {
                    "The manifest digest is unchanged; this edit did not alter the publish candidate set. Preview and approve the digest you want to publish."
                } else {
                    "The manifest changed. Nothing is public yet: preview and approve the new digest, then publish it."
                },
            }))
        }
        Err(err) => typed_refusal(&err),
    }
}

/// `POST /v1/admin/delivery/allowlist/{id}/remove` — withdraw one share
/// record. Same confirm- and digest-binding as the set route; an unknown
/// record reports `removed: false` honestly rather than faking an effect.
pub fn allowlist_remove(
    db_path: &Path,
    id: &str,
    body: &Value,
    request: &ApiRequest,
) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    if let Err(response) = require_managed(&registry, id) {
        return response;
    }
    if let Err(response) = confirmed_digest(&registry, body, "withdrawing this project") {
        return response;
    }
    match registry.share_remove_record(id) {
        Ok(removed) => {
            let preview = preview_block(&registry);
            scrub(json!({
                "contract": CONTRACT_VERSION,
                "effect": if removed { "forge-owned-write" } else { "none" },
                "actor": actor_for(db_path, request),
                "project_id": id,
                "removed": removed,
                "preview": preview,
                "note": if removed {
                    "The record no longer joins the publish candidate set; the previously approved manifest is unchanged until a new digest is approved."
                } else {
                    "This project had no share record; nothing was changed."
                },
            }))
        }
        Err(err) => typed_refusal(&err),
    }
}

/// `POST /v1/admin/delivery/approve` — approve the exact manifest hash the
/// operator reviewed. Requires `confirm: true` plus `plan_digest` equal to
/// the freshly recomputed manifest hash; Core re-derives the draft inside
/// its transaction as a second layer, so a record edited between preview
/// and approval is refused and no approval row is created. A manifest with
/// findings is refused as a typed blocker, never approved partially.
pub fn approve(db_path: &Path, body: &Value, request: &ApiRequest) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let current_digest = match confirmed_digest(&registry, body, "approving this manifest") {
        Ok(digest) => digest,
        Err(response) => return response,
    };
    match registry.share_approve(&current_digest, &actor_for(db_path, request)) {
        Ok(approval) => scrub(json!({
            "contract": CONTRACT_VERSION,
            "effect": "forge-owned-write",
            "actor": approval.actor.clone(),
            "approval": approval_block(&approval),
            "note": "The manifest is approved for this exact hash. Publishing it is a separate confirmed action.",
        })),
        Err(err) => typed_refusal(&err),
    }
}

/// `POST /v1/admin/delivery/publish` — publish the approved manifest
/// through the default-safe local export. Requires `confirm: true`, the
/// approved `plan_digest` and a validated `operation_key` (the idempotency
/// identity: a retry under the same key reconciles against the recorded
/// attempt and never publishes twice). The artifact path comes from the
/// server-side environment; the browser never names a path, and the
/// subprocess adapter is not reachable from this route. Core refuses an
/// unreconciled `unknown` attempt before dispatch, so an ambiguous
/// outcome must be reconciled first.
pub fn publish(db_path: &Path, body: &Value, request: &ApiRequest) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let confirm = body
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !confirm {
        return refuse(
            409,
            "delivery-confirm-required",
            "publishing makes a manifest artifact public; refusing implicit external delivery. Send `confirm: true` with the approved digest and operation key.",
        );
    }
    let supplied_digest = body
        .get("plan_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if supplied_digest.is_empty() {
        return refuse(
            400,
            "delivery-invalid",
            "publish requires the `plan_digest` of the approved manifest you reviewed.",
        );
    }
    let raw_key = body
        .get("operation_key")
        .and_then(Value::as_str)
        .unwrap_or("");
    let operation_key = match share::validate_operation_key(raw_key) {
        Ok(key) => key,
        Err(reason) => {
            return refuse(
                400,
                "delivery-invalid",
                &format!("operation key refused: {reason}"),
            )
        }
    };
    let Some(target) = configured_target() else {
        return refuse(
            409,
            "delivery-prerequisite",
            &format!(
                "no publication target is configured on this host (`{PUBLISH_TARGET_ENV}` is unset in the API process). The browser never accepts a filesystem path; configure the target in a terminal or publish with `forge portfolio share publish` there. Nothing was dispatched."
            ),
        );
    };
    let approval = match registry.share_latest_approval() {
        Ok(Some(approval)) => approval,
        Ok(None) => {
            return refuse(
                409,
                "delivery-prerequisite",
                "no manifest has been approved yet; preview and approve the digest you want to publish. Nothing was dispatched.",
            )
        }
        Err(_) => return unavailable(),
    };
    if approval.state == "published"
        && registry
            .share_publication_by_key(&operation_key)
            .map(|attempt| attempt.is_none())
            .unwrap_or(false)
    {
        return refuse(
            409,
            "delivery-plan-stale",
            "the approved manifest is already published; only a retry of its own operation key is accepted. Review the preview and approve a new revision to publish further changes. Nothing was dispatched.",
        );
    }
    if approval.manifest_sha256 != supplied_digest {
        return ApiResponse::json(
            409,
            json!({
                "contract": CONTRACT_VERSION,
                "error": {
                    "code": "delivery-plan-stale",
                    "message": "the confirmed digest does not match the newest approval; publish dispatches only the exact approved manifest. Nothing was dispatched.",
                },
                "effect": "none",
                "approved_digest": approval.manifest_sha256,
                "approval_revision": approval.revision,
            }),
        );
    }
    let plan = PublishPlan {
        operation_key,
        target,
        actor: actor_for(db_path, request),
        // The browser never reaches the subprocess adapter: the typed
        // default-safe local export is the only web dispatch.
        adapter: None,
    };
    match publish_approved_manifest(&registry, &plan, Utc::now()) {
        Ok(report) => scrub(json!({
            "contract": CONTRACT_VERSION,
            "accepted": true,
            "operation_id": report.publication_id,
            "publication": publication_block_from_report(&report),
            "note": publication_note(&report),
        })),
        Err(err) => typed_refusal(&err),
    }
}

/// `POST /v1/admin/delivery/reconcile` — record the operator-observed
/// outcome of a publication attempt Core could not vouch for (`unknown`).
/// Requires `confirm: true`, the attempt id, an explicit `published` or
/// `failed` outcome and the attempt's recorded digest: a reconciliation
/// that does not match the ambiguous attempt is refused, and only an
/// `unknown` attempt can be reconciled — so an ambiguous dispatch is
/// resolved before anything retries.
pub fn reconcile(db_path: &Path, body: &Value, request: &ApiRequest) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let confirm = body
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !confirm {
        return refuse(
            409,
            "delivery-confirm-required",
            "reconciling records an external outcome as your explicit statement; refusing implicit audit writes. Send `confirm: true` with the attempt id, observed outcome and its digest.",
        );
    }
    let Some(publication_id) = body.get("publication_id").and_then(Value::as_i64) else {
        return refuse(
            400,
            "delivery-invalid",
            "reconcile requires the numeric `publication_id` of the ambiguous attempt.",
        );
    };
    let raw_status = body.get("status").and_then(Value::as_str).unwrap_or("");
    let status = match PublicationStatus::parse(raw_status) {
        Ok(status) if status != PublicationStatus::Unknown => status,
        Ok(_) => {
            return refuse(
                400,
                "delivery-invalid",
                "reconciliation records what you observed: `published` or `failed`. `unknown` is not a reconciliation outcome.",
            )
        }
        Err(reason) => return refuse(400, "delivery-invalid", &reason),
    };
    let supplied_digest = body
        .get("plan_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if supplied_digest.is_empty() {
        return refuse(
            400,
            "delivery-invalid",
            "reconcile requires the `plan_digest` recorded on the ambiguous attempt.",
        );
    }
    let attempt = match registry.share_publication(publication_id) {
        Ok(Some(attempt)) => attempt,
        Ok(None) => {
            return refuse(
                404,
                "delivery-operation-not-found",
                "no publication attempt with that id is recorded here; nothing was changed.",
            )
        }
        Err(_) => return unavailable(),
    };
    if attempt.manifest_sha256 != supplied_digest {
        return ApiResponse::json(
            409,
            json!({
                "contract": CONTRACT_VERSION,
                "error": {
                    "code": "delivery-plan-stale",
                    "message": "the confirmed digest does not match the recorded attempt; reconciliation binds to the exact ambiguous publication. Nothing was changed.",
                },
                "effect": "none",
                "attempted_digest": attempt.manifest_sha256,
            }),
        );
    }
    match registry.share_reconcile_publication(publication_id, status, &actor_for(db_path, request))
    {
        Ok(reconciled) => scrub(json!({
            "contract": CONTRACT_VERSION,
            "effect": "forge-owned-write",
            "actor": reconciled.actor.clone(),
            "publication": publication_attempt_block(&reconciled),
            "note": "The outcome is recorded as the operator's explicit statement. A new publication is reachable again; a retry of the same operation key still resolves against this row.",
        })),
        Err(err) => typed_refusal(&err),
    }
}

/// `GET /v1/admin/delivery/operation/{key}` — journal-backed status of one
/// publication by its operation key. The key is a validated Core token
/// (letters, digits, dash, underscore, dot), looked up by parameterized
/// read only; it is never a path and nothing is dispatched by a status
/// read. `unknown` answers with reconciliation guidance, never a guess.
pub fn operation(db_path: &Path, key: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let operation_key = match share::validate_operation_key(key) {
        Ok(key) => key,
        Err(reason) => {
            return refuse(
                400,
                "delivery-invalid",
                &format!("operation key refused: {reason}"),
            )
        }
    };
    match registry.share_publication_by_key(&operation_key) {
        Ok(Some(attempt)) => {
            let status = attempt.status.clone();
            scrub(json!({
                "contract": CONTRACT_VERSION,
                "operation_id": attempt.publication_id,
                "publication": publication_attempt_block(&attempt),
                "guidance": status_guidance(&status),
            }))
        }
        Ok(None) => refuse(
            404,
            "delivery-operation-not-found",
            "no publication attempt with that operation key is recorded here; nothing was dispatched by this read.",
        ),
        Err(err) => typed_refusal(&err),
    }
}

// --- shared helpers -------------------------------------------------------

/// The confirm gate plus the digest binding every mutation runs through:
/// `confirm: true` first, then the freshly recomputed manifest hash. Any
/// mismatch is a `409` carrying the current digest and the promise that
/// nothing changed — the workbench stale-plan pattern, one-for-one.
fn confirmed_digest(
    registry: &Registry,
    body: &Value,
    intent: &str,
) -> Result<String, ApiResponse> {
    let confirm = body
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !confirm {
        return Err(refuse(
            409,
            "delivery-confirm-required",
            &format!(
                "{intent} changes what may become public; refusing implicit delivery mutation. Send `confirm: true` with the reviewed plan_digest."
            ),
        ));
    }
    let supplied = body
        .get("plan_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if supplied.is_empty() {
        return Err(refuse(
            400,
            "delivery-invalid",
            "the request requires the `plan_digest` of the manifest preview you reviewed.",
        ));
    }
    let draft = match preview_manifest(registry) {
        Ok(draft) => draft,
        Err(err) => return Err(typed_refusal(&err)),
    };
    let current = draft.manifest_sha256();
    if current != supplied {
        return Err(ApiResponse::json(
            409,
            json!({
                "contract": CONTRACT_VERSION,
                "error": {
                    "code": "delivery-plan-stale",
                    "message": "the registry no longer matches the confirmed manifest; nothing was changed. Review the refreshed preview digest and confirm again.",
                },
                "effect": "none",
                "preview": preview_block(registry),
            }),
        ));
    }
    Ok(current)
}

/// Read-only manifest preview block shared by every response; `None` only
/// when the registry read itself failed, which is reported honestly.
fn preview_block(registry: &Registry) -> Option<Value> {
    let draft = preview_manifest(registry).ok()?;
    Some(json!({
        "manifest_revision": draft.body.manifest_revision,
        "manifest_sha256": draft.manifest_sha256(),
        "project_count": draft.project_count(),
        "approvable": draft.approvable(),
        "findings": draft
            .findings
            .iter()
            .take(MAX_FINDINGS)
            .map(|finding| json!({
                "project_id": finding.project_id,
                "field": finding.field,
                "code": finding.code,
                "detail": finding.detail,
            }))
            .collect::<Vec<_>>(),
    }))
}

/// The recorded state of the server-side publication target. The path
/// itself never leaves the server; the browser only learns whether a
/// dispatch is possible at all.
fn target_block() -> Value {
    match std::env::var(PUBLISH_TARGET_ENV) {
        Ok(value) if !value.trim().is_empty() => json!({
            "configured": true,
            "publisher": "local-file export",
            "note": "Publishing from the browser writes the approved manifest to this server-configured artifact path. Adapter publication stays CLI-only.",
        }),
        _ => json!({
            "configured": false,
            "publisher": Value::Null,
            "note": format!("`{PUBLISH_TARGET_ENV}` is not set on the API process, so a browser publish is refused as a typed prerequisite. Configure it in a terminal or publish with `forge portfolio share publish`."),
        }),
    }
}

fn configured_target() -> Option<String> {
    std::env::var(PUBLISH_TARGET_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Validate the opaque id shape and confirm the project is registered —
/// share records attach to managed projects only (Core enforces this
/// again inside the write). The id is never echoed back.
fn require_managed(registry: &Registry, id: &str) -> Result<(), ApiResponse> {
    if validate_project_id(id).is_err() {
        return Err(refuse(
            400,
            "delivery-invalid",
            "the project id is not a valid identifier; it may not contain a path.",
        ));
    }
    if registry.inspect(id).is_ok() {
        return Ok(());
    }
    Err(refuse(
        404,
        "delivery-unmanaged-project",
        "this project is not managed by this Forge registry; it cannot be allowlisted for delivery from the browser.",
    ))
}

/// The audit actor: the global administrator's own address resolved from
/// the validated session, never from a browser-supplied field. Falls back
/// to a fixed persona label if the identity read fails.
fn actor_for(db_path: &Path, request: &ApiRequest) -> String {
    let token = request
        .cookies
        .get(SESSION_COOKIE)
        .map(String::as_str)
        .unwrap_or("");
    crate::identity::global::session_actor(db_path, token)
        .ok()
        .flatten()
        .unwrap_or_else(|| "global-admin".to_string())
}

/// Parse one share-record form into the typed Core write. Text limits and
/// URL/private-surface rules are deliberately **not** re-implemented here:
/// the registry's `validate_share` is the single gate, so a browser
/// refusal and a CLI refusal look the same.
fn parse_share_write(body: &Value) -> Result<ShareWrite, ApiResponse> {
    let title = required_str(body, "title")?;
    let summary = required_str(body, "summary")?;
    let category = required_str(body, "category")?;
    let source_url = required_str(body, "source_url")?;
    let visibility = match optional_str(body, "visibility") {
        Some(raw) => {
            Visibility::parse(raw).map_err(|reason| refuse(400, "delivery-invalid", &reason))?
        }
        None => Visibility::Public,
    };
    let showcase_status = match optional_str(body, "showcase_status") {
        Some(raw) => match share::ShowcaseStatus::parse(raw) {
            Ok(value) => value,
            Err(reason) => return Err(refuse(400, "delivery-invalid", &reason)),
        },
        None => share::ShowcaseStatus::Unknown,
    };
    let mut surfaces = Vec::new();
    if let Some(entries) = body.get("surfaces").and_then(Value::as_array) {
        for entry in entries {
            surfaces.push(ShareSurface::new(
                entry
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                entry.get("url").and_then(Value::as_str).unwrap_or_default(),
            ));
        }
    }
    Ok(ShareWrite {
        title: title.to_string(),
        summary: summary.to_string(),
        category: category.to_string(),
        source_url: source_url.to_string(),
        demo_url: optional_str(body, "demo_url").map(str::to_string),
        visibility,
        featured: body
            .get("featured")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        showcase_status,
        status_evidence: optional_str(body, "status_evidence").map(str::to_string),
        surfaces,
    })
}

fn project_record(record: &ShareRecord) -> Value {
    json!({
        "project_id": record.project_id,
        "title": record.title,
        "summary": record.summary,
        "category": record.category,
        "source_url": record.source_url,
        "demo_url": record.demo_url,
        "visibility": record.visibility,
        "featured": record.featured,
        "showcase_status": record.showcase_status,
        "state": record.state.label(),
        "revision": record.revision,
        "updated_at": record.updated_at,
        "surfaces": record.surfaces.iter().map(|s| json!({
            "label": s.label,
            "url": s.url,
        })).collect::<Vec<_>>(),
    })
}

fn approval_block(approval: &share::ShareApproval) -> Value {
    json!({
        "revision": approval.revision,
        "manifest_sha256": approval.manifest_sha256,
        "project_count": approval.project_count,
        "actor": approval.actor,
        "approved_at": approval.approved_at,
        "state": approval.state,
    })
}

fn publication_attempt_block(attempt: &share::PublicationAttempt) -> Value {
    json!({
        "publication_id": attempt.publication_id,
        "operation_key": attempt.operation_key,
        "manifest_revision": attempt.manifest_revision,
        "manifest_sha256": attempt.manifest_sha256,
        "status": attempt.status,
        "published_revision": attempt.published_revision,
        "error_code": attempt.error_code,
        "actor": attempt.actor,
        "attempted_at": attempt.attempted_at,
        "finished_at": attempt.finished_at,
    })
}

/// A publish report projected without the recorded target path — the
/// browser sees the publisher kind and outcome, never the filesystem
/// location configured on the server. Core's publisher `label()` is
/// `local-file:<absolute path>`, so the path-bearing tail is dropped
/// here rather than scrubbed, and the subprocess label never reaches
/// this route (the web always dispatches `adapter: None`).
fn publication_block_from_report(report: &PublishReport) -> Value {
    let publisher_kind = match report.publisher.split_once(':') {
        Some(("local-file", _)) => "local-file export",
        Some((kind, _)) => kind,
        None => "local-file export",
    };
    json!({
        "publication_id": report.publication_id,
        "operation_key": report.operation_key,
        "manifest_revision": report.manifest_revision,
        "manifest_sha256": report.manifest_sha256,
        "status": report.status.label(),
        "publisher": publisher_kind,
        "project_count": report.project_count,
        "published_revision": report.published_revision,
        "error_code": report.error_code,
        "already_present": report.already_present,
    })
}

/// Honest outcome copy: what landed is stated exactly as Core recorded it;
/// nothing is claimed beyond the artifact this host wrote.
fn publication_note(report: &PublishReport) -> &'static str {
    match report.status {
        PublicationStatus::Published if report.already_present => {
            "The target already held exactly these bytes; this retry reconciled the recorded attempt and created no second publication."
        }
        PublicationStatus::Published => {
            "The approved manifest artifact was written through the default-safe local export. This confirms the artifact on this host only; promotion beyond it stays with the configured CLI provider."
        }
        PublicationStatus::Unknown => {
            "The outcome of this dispatch is unknown to Forge. Reconcile it by publication id before publishing a new revision; do not blindly retry."
        }
        PublicationStatus::Failed => {
            "The publication attempt failed and is recorded; the approval is retained so a retry under the same operation key reconciles the same attempt."
        }
    }
}

fn status_guidance(status: &str) -> &'static str {
    match status {
        "published" => "The attempt landed and is journaled.",
        "failed" => "The attempt failed and is recorded; the approval is retained. Retry the same operation key resolves against this row.",
        "unknown" => "Forge cannot vouch for this dispatch. Reconcile it by publication id with its exact digest before publishing anything new.",
        "pending" => "An attempt is recorded but was never closed: the process may have died mid-publish. Retry the same operation key — the retry re-renders the recorded bytes and reconciles this row rather than publishing twice.",
        _ => "The attempt is recorded as shown.",
    }
}

fn required_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, ApiResponse> {
    body.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            refuse(
                400,
                "delivery-invalid",
                &format!("the share record requires a `{field}` field"),
            )
        })
}

fn optional_str<'a>(body: &'a Value, field: &str) -> Option<&'a str> {
    body.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

/// Map a typed Core error to the delivery refusal envelope. Core `Display`
/// can name an absolute path (an unavailable target directory), so it is
/// never echoed verbatim: only path-scrubbed, code-keyed text leaves here.
fn typed_refusal(err: &ForgeError) -> ApiResponse {
    let code = err.code();
    let (status, message): (u16, String) = match code {
        "unknown-project" => (
            404,
            "the selected project is not managed by this Forge registry; nothing was changed."
                .to_string(),
        ),
        "path-unavailable" => (
            409,
            "the configured publication target is not usable on this host; the attempt is recorded honestly and no artifact was claimed as published."
                .to_string(),
        ),
        "portfolio-share-invalid" => (400, status_reason(err)),
        "portfolio-share-conflict" => (
            409,
            format!(
                "delivery conflict: {}; nothing was dispatched or the attempt stayed as recorded.",
                status_reason(err)
            ),
        ),
        _ => (
            503,
            "the delivery service could not complete this operation; nothing was changed."
                .to_string(),
        ),
    };
    ApiResponse::json(
        status,
        json!({
            "contract": CONTRACT_VERSION,
            "error": { "code": code, "message": redact_local_paths(&message) },
            "effect": "none",
        }),
    )
}

/// A short, path-scrubbed human reason from a Core error. Validation and
/// conflict text names fields, hashes and ids — never secrets; anything
/// that still looks like a local path is redacted.
fn status_reason(err: &ForgeError) -> String {
    err.to_string()
        .split(';')
        .next()
        .unwrap_or("the operation could not be completed")
        .to_string()
}

fn refuse(status: u16, code: &str, reason: &str) -> ApiResponse {
    let reason = redact_local_paths(reason);
    ApiResponse::json(
        status,
        json!({
            "contract": CONTRACT_VERSION,
            "error": { "code": code, "message": reason },
            "effect": "none",
        }),
    )
}

fn unavailable() -> ApiResponse {
    refuse(
        503,
        "delivery-unavailable",
        "the delivery service could not open the registry; no rows, approvals or artifacts were changed.",
    )
}

/// Replace whitespace-separated tokens that look like absolute local paths
/// (`/home/…`, `C:\…`, `key=/value`) with a fixed marker. API route strings
/// (which always begin with the `/v1/` namespace) are left intact: they are
/// self-authored endpoints, never filesystem locations.
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

/// Recursively scrub every string in the projection of absolute-path
/// tokens, so no filesystem location can ever leave this module.
fn scrub(value: Value) -> ApiResponse {
    ApiResponse::json(200, scrub_value(value))
}

fn scrub_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_local_paths(&text)),
        Value::Array(items) => Value::Array(items.into_iter().map(scrub_value).collect()),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, item) in map {
                out.insert(key, scrub_value(item));
            }
            Value::Object(out)
        }
        other => other,
    }
}
