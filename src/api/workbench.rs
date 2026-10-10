//! forge — typed, session-gated single-project workbench
//! (`forge-project-workbench/0.1.0`).
//!
//! The workbench turns a small, fixed set of Core operations into
//! browser workflows for one **managed** project: read-only inspection
//! (manifest, profile, features, doctor health, journal-backed evidence),
//! a side-effect-free upgrade **plan**, and a confirmation-bound upgrade
//! **apply** that returns a journaled operation identity.
//!
//! Security boundary (see `.ai-rules/concerns/security.md`):
//! - Nothing here ever runs a shell, `sh -c`, an interpreter, or the
//!   `forge` executable, and no route accepts arbitrary command text.
//! - The browser never supplies a filesystem path. A project is addressed
//!   only by an opaque, validated `id`; its root is resolved **server-side**
//!   from the registry, so a request cannot reach files outside the
//!   selected registered project root.
//! - Only typed in-process Core functions are invoked
//!   ([`crate::registry::Registry`], [`crate::doctor::run_doctor`],
//!   [`crate::upgrade::plan_upgrade`] / [`crate::upgrade::apply_upgrade`],
//!   [`crate::policy::run_driftwatch`]).
//! - Absolute filesystem paths are never serialized; the record/doctor
//!   `path` fields are stripped and journal `detail` text is redacted of
//!   absolute-path tokens before a response leaves this module.
//! - Workflows whose Core contract needs a native toolchain, an
//!   interactive terminal, a provider credential or a browser file upload
//!   are surfaced through their command-catalog disposition as an honest
//!   `cli_only` / `not_yet_web` state — never as a nonfunctional control.

use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{ApiRequest, ApiResponse};
use crate::core::{validate_project_id, ForgeError};
use crate::doctor::{run_doctor, DoctorReport, RegistryObservation};
use crate::policy::{run_driftwatch, DriftWatchConfig};
use crate::registry::Registry;
use crate::upgrade::{apply_upgrade, plan_upgrade};

/// Versioned workbench contract. Additive only; removing or renaming a
/// stable action id requires a contract migration.
pub const CONTRACT_VERSION: &str = "forge-project-workbench/0.1.0";

/// The typed JSON routes the workbench actually implements today. These are
/// the honest `web` destinations a command-catalog row may point at.
pub const ROUTE_PROJECT_DETAIL: &str = "GET /v1/admin/projects/{id}";
pub const ROUTE_PROJECT_PLAN: &str = "GET /v1/admin/projects/{id}/plan";
pub const ROUTE_PROJECT_APPLY: &str = "POST /v1/admin/projects/{id}/apply";
/// Explicit on-demand full health check. Unlike the detail GET (fast local
/// pass, see [`build_health`]), this route runs the live external policy
/// check plus doctor and returns the complete health document. Read-only
/// effect: no confirm/digest binding, no journal row.
pub const ROUTE_PROJECT_HEALTH_REFRESH: &str = "POST /v1/admin/projects/{id}/health/refresh";

/// Finding id appended by the fast read path when the live policy check
/// did not run for the returned view. The workbench card renders this row
/// with the refresh control instead of a remediate shortcut.
const POLICY_DEFERRED_FINDING: &str = "policy-deferred";

/// Reason shown for a catalog `web` row that is global (id-less) rather than
/// scoped to the resolved project. Such a row is executable from the
/// dashboard's project-management section, not from one project's workbench.
const GLOBAL_ACTION_REASON: &str =
    "this action creates, registers or adopts a project and lives in the dashboard's \"Create or adopt a project\" section, not inside a single project's workbench.";

/// Command-catalog ids that name a workflow in the project scope. Kept as
/// data so the disposition view and the catalog never disagree: each row's
/// availability/reason/route is read straight from the catalog.
const SCOPED_COMMANDS: &[&str] = &[
    "inspect",
    "doctor",
    "upgrade",
    // Honest dispositions surfaced next to the available actions so the
    // operator sees the whole project surface, not only what is wired.
    // This mirrors the design's command-in-scope list one-for-one.
    "register",
    "import",
    "new",
    "graduation.preview",
    "graduation.import",
    "profile.list",
    "profile.inspect",
    "profile.resolve",
    "profile.preflight",
    "kit",
    "kit.pack",
    "kit.verify",
    "kit.upgrade",
    "feature.list",
    "feature.inspect",
    "feature.resolve",
    "feature.add",
    "feature.remove",
    "feature.upgrade",
    "component.list",
    "component.inspect",
    "component.resolve",
    "component.qualify",
    "ui-pattern.list",
    "ui-pattern.inspect",
    "ui-pattern.resolve",
    "ui-pattern.install",
    "intent.validate",
    "intent.resolve",
    "intent.apply",
    "intent.list",
    "procedure.list",
    "procedure.inspect",
    "procedure.validate",
    "standard.list",
    "standard.inspect",
    "standard.check",
    "standard.diff",
    "standard.upgrade",
    "spec.generate",
    "spec.list",
    "spec.inspect",
    "spec.route",
    "spec.apply",
    "remediate",
    "describe",
    "classify",
    "identity",
    "identity.session-validate",
    "identity.complete-auth",
    "agent.start",
    "agent.pause",
    "agent.takeover",
    "agent.resume",
    "agent.restart",
    "gate",
    "test",
    "check",
];

/// Upper bound on journal evidence echoed to the browser for one project,
/// newest first, so the payload stays small regardless of history length.
const MAX_JOURNAL_ROWS: usize = 20;

/// Result of resolving a request against the registry: the managed record,
/// or a typed, honest refusal the handler renders verbatim.
pub(super) enum Resolved {
    /// Boxed: `ProjectRecord` is ~496 bytes and the refusal arm is tiny;
    /// keeping the enum narrow avoids copying the whole record on every
    /// validated-but-unmanaged request.
    Managed(Box<crate::registry::ProjectRecord>),
    /// The id failed shape validation, or is not present in this Forge
    /// registry (observed-only or unknown).
    Refused { status: u16, reason: &'static str },
}

/// Validate the opaque id and resolve the registered root **server-side**.
/// A cross-project or path-bearing reference is impossible because only a
/// validated id reaches `registry.inspect`; the browser never sends a path.
pub(super) fn resolve(registry: &Registry, id: &str) -> Resolved {
    if validate_project_id(id).is_err() {
        return Resolved::Refused {
            status: 400,
            reason: "the project id is not a valid identifier; it may not contain a path.",
        };
    }
    match registry.inspect(id) {
        Ok(record) => Resolved::Managed(Box::new(record)),
        Err(_) => Resolved::Refused {
            status: 404,
            reason: "this project is not managed by this Forge registry; it is either unknown or observed-only from an external source, so it cannot be inspected or operated in the browser. Register it with `forge register <path>` in a terminal first.",
        },
    }
}

/// Hex SHA-256 of the canonical plan bytes. The plan document contains no
/// absolute path, so hashing it binds confirmation to the exact project,
/// operation and step set without echoing anything sensitive.
fn plan_digest(plan: &Value) -> String {
    let bytes = serde_json::to_vec(plan).unwrap_or_default();
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

/// Derive the honest workflow disposition for the scoped command set from
/// the command catalog — a single source of truth. A row is actionable here
/// only when it is a `web` route that addresses **this** project (its route
/// carries `{id}`); a global, id-less `web` row (project creation,
/// registration, adoption) stays catalog-visible but reads not-actionable
/// inside a single project, with a reason that points at the dashboard's
/// project-management section. Every other row carries the catalog's own
/// plain-language reason.
fn disposition() -> Vec<Value> {
    let catalog = super::command_catalog::rows();
    let mut out = Vec::new();
    for id in SCOPED_COMMANDS {
        if let Some(row) = catalog.iter().find(|row| row.id == *id) {
            let scoped = row.route.is_some_and(|route| route.contains("{id}"));
            let available = row.availability == "web" && scoped;
            let reason = match (available, row.reason) {
                (true, _) => None,
                (false, Some(reason)) => Some(reason.to_string()),
                (false, None) => Some(GLOBAL_ACTION_REASON.to_string()),
            };
            out.push(json!({
                "id": row.id,
                "label": row.label,
                "summary": row.summary,
                "availability": row.availability,
                "available": available,
                "route": row.route,
                "reason": reason,
            }));
        }
    }
    out
}

/// The read-only workbench projection for one managed project: manifest +
/// profile + features, a bounded doctor health report, the newest journal
/// evidence, and the honest per-workflow disposition. No side effects, no
/// absolute path.
pub fn detail(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let record = match resolve(&registry, id) {
        Resolved::Managed(record) => record,
        Resolved::Refused { status, reason } => return refuse(status, reason),
    };

    let mut manifest = serde_json::to_value(&record).unwrap_or(Value::Null);
    if let Some(object) = manifest.as_object_mut() {
        // Never leak the registered filesystem location to the browser.
        object.remove("path");
    }

    let project_dir = Path::new(&record.path);
    let health = build_health(&record, project_dir);
    let operations = journal_evidence(&registry, id);

    ApiResponse::json(
        200,
        json!({
            "contract": CONTRACT_VERSION,
            "project_id": record.id,
            "management": management_label(&record.id),
            "manifest": manifest,
            "health": health,
            "operations": operations,
            "workflows": disposition(),
        }),
    )
}

fn management_label(id: &str) -> &'static str {
    if id == "forge" {
        "self"
    } else {
        "managed"
    }
}

/// Read-only doctor report for the resolved project directory, with the
/// absolute `path` stripped. A project whose root is not currently present
/// reports an honest `unavailable` health state rather than a fake pass.
///
/// Latency contract (`workbench-health-latency`): this is the **fast**
/// path. It runs the local doctor pass only — no external policy process
/// — and appends an explicit `unavailable` [`POLICY_DEFERRED_FINDING`]
/// finding, so a locally-clean project reports `deferred`, never
/// `healthy`. The full live pass runs only behind [`refresh_health`].
fn build_health(record: &crate::registry::ProjectRecord, dir: &Path) -> Value {
    if !record.available || !dir.is_dir() {
        return unavailable_health();
    }
    let observation = Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at.clone()),
    });
    match run_doctor(dir, None, observation.as_ref(), None) {
        Ok(report) => finish_health(report, false),
        Err(_) => json!({
            "state": "unavailable",
            "note": "the health inspection could not complete for this project; no files were changed. Run `forge doctor` in a terminal for the full report.",
        }),
    }
}

/// Explicit full health check: the live external policy pass plus doctor,
/// exactly as the detail GET computed before `workbench-health-latency`.
/// Read-only effect — the returned document is the evidence, so no
/// confirm/digest binding and no journal row.
pub fn refresh_health(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let record = match resolve(&registry, id) {
        Resolved::Managed(record) => record,
        Resolved::Refused { status, reason } => return refuse(status, reason),
    };
    let project_dir = Path::new(&record.path);
    if !record.available || !project_dir.is_dir() {
        return ApiResponse::json(
            200,
            json!({
                "contract": CONTRACT_VERSION,
                "project_id": record.id,
                "health": unavailable_health(),
            }),
        );
    }
    let observation = Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at.clone()),
    });
    let policy_outcome = run_driftwatch(project_dir, &DriftWatchConfig::from_env());
    match run_doctor(
        project_dir,
        None,
        observation.as_ref(),
        Some(&policy_outcome),
    ) {
        Ok(report) => ApiResponse::json(
            200,
            json!({
                "contract": CONTRACT_VERSION,
                "project_id": record.id,
                "health": finish_health(report, true),
            }),
        ),
        Err(_) => ApiResponse::json(
            200,
            json!({
                "contract": CONTRACT_VERSION,
                "project_id": record.id,
                "health": {
                    "state": "unavailable",
                    "note": "the health inspection could not complete for this project; no files were changed. Run `forge doctor` in a terminal for the full report.",
                },
            }),
        ),
    }
}

/// Shared shape finisher for both health paths: strip the absolute path,
/// label honestly, and — on the fast path only — append the deferred
/// policy finding so no row claims a pass that was not computed.
fn finish_health(report: DoctorReport, policy_ran: bool) -> Value {
    let mut value = serde_json::to_value(&report).unwrap_or(Value::Null);
    if let Some(object) = value.as_object_mut() {
        object.remove("path");
        if !policy_ran {
            if let Some(findings) = object.get_mut("findings").and_then(|f| f.as_array_mut()) {
                findings.push(json!({
                    "id": POLICY_DEFERRED_FINDING,
                    "status": "unavailable",
                    "evidence": ["the live policy check runs only on explicit refresh, not on read"],
                    "applicable": false,
                    "remediation": "manual",
                    "detail": "the live policy check has not run for this view; run the full health check to compute it",
                }));
            }
        }
    }
    let label = if !report.healthy {
        "issues"
    } else if report.stale {
        "stale"
    } else if policy_ran {
        "healthy"
    } else {
        "deferred"
    };
    value["state"] = json!(label);
    value
}

fn unavailable_health() -> Value {
    json!({
        "state": "unavailable",
        "note": "the registered project directory is not currently readable on this host; no health report can be produced. Nothing was changed.",
    })
}

/// Newest-first journal rows for this project only, with timestamps. Only
/// safe logical fields are projected; no filesystem path or credential.
/// Failure details recorded by Core can name the offending absolute path,
/// so every projected token is redacted first.
fn journal_evidence(registry: &Registry, id: &str) -> Vec<Value> {
    let entries = match registry.journal_entries() {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|entry| entry.project_id == id)
        .rev()
        .take(MAX_JOURNAL_ROWS)
        .map(|entry| {
            json!({
                "operation_id": entry.op_id,
                "kind": entry.kind,
                "state": entry.state,
                "started_at": entry.started_at,
                "finished_at": entry.finished_at,
                "detail": entry.detail.as_deref().map(redact_local_paths),
            })
        })
        .collect()
}

/// Replace whitespace-separated tokens that look like absolute local
/// filesystem paths (`/home/…`, `C:\…`, `key=/value`) with a fixed marker.
/// Journal `detail` text is Core's `Display` output, which legitimately
/// names paths on failure; the browser only ever needs the logical reason.
pub(super) fn redact_local_paths(detail: &str) -> String {
    detail
        .split_whitespace()
        .map(|token| {
            let is_abs = token.starts_with('/')
                || (token.len() > 2
                    && token.as_bytes()[1] == b':'
                    && (token.as_bytes()[2] == b'\\' || token.as_bytes()[2] == b'/'))
                || token.contains("=/")
                || token.contains(":\\");
            if is_abs {
                "[local path]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A side-effect-free upgrade plan plus the digest a later confirmation
/// must echo. Reads the registry and project files only; writes nothing to
/// disk, the registry or the journal.
pub fn plan(db_path: &Path, id: &str, requested: Option<&str>) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let record = match resolve(&registry, id) {
        Resolved::Managed(record) => record,
        Resolved::Refused { status, reason } => return refuse(status, reason),
    };
    if let Some(response) = absent_dir_refusal(&record) {
        return response;
    }
    match plan_upgrade(&registry, &record.path, requested) {
        Ok(plan) => {
            let value = serde_json::to_value(&plan).unwrap_or(Value::Null);
            let digest = plan_digest(&value);
            ApiResponse::json(
                200,
                json!({
                    "contract": CONTRACT_VERSION,
                    "project_id": record.id,
                    "effect": "none",
                    "plan": value,
                    "plan_digest": digest,
                    "confirmation": {
                        "requires": ["confirm", "plan_digest", "project_id"],
                        "note": "Applying this plan writes to the project files. Confirm by sending `confirm: true` with this exact plan_digest and project_id; a changed or stale digest is refused and a fresh plan is returned.",
                    },
                }),
            )
        }
        Err(err) => typed_refusal(&err),
    }
}

/// Journal kind recorded for confirmed workbench applies. Shared by the
/// apply handler and its replay pre-check so they can never drift.
const APPLY_KIND: &str = "workbench.upgrade.apply";

/// Confirm- and digest-bound upgrade apply. Never trusts a browser-supplied
/// path: the root is re-resolved from the registry, the plan is recomputed,
/// and the write only runs when the client digest still matches. Failures
/// leave the project and journal unchanged.
pub fn apply(db_path: &Path, id: &str, body: &Value, request: &ApiRequest) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let record = match resolve(&registry, id) {
        Resolved::Managed(record) => record,
        Resolved::Refused { status, reason } => return refuse(status, reason),
    };

    // An idempotent retry of a request that already ran must return the
    // journaled operation, not a stale-plan refusal: after a successful
    // apply the recomputed digest legitimately differs, because the first
    // attempt moved the project on. Checked before the digest gate so a
    // retry never re-plans against changed state.
    if let Some(key) = request.idempotency_key.as_deref() {
        if let Ok(Some(entry)) = registry.operation_by_idempotency(APPLY_KIND, key) {
            if entry.project_id == id {
                let status = if entry.state == "pending" { 202 } else { 200 };
                return ApiResponse::json(
                    status,
                    json!({
                        "contract": CONTRACT_VERSION,
                        "accepted": true,
                        "replay": true,
                        "operation_id": entry.op_id,
                        "project_id": id,
                        "state": entry.state,
                        "started_at": entry.started_at,
                        "finished_at": entry.finished_at,
                        "detail": entry.detail.as_deref().map(redact_local_paths),
                    }),
                );
            }
        }
    }

    let confirm = body
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !confirm {
        return refuse(
            409,
            "applying this operation requires `confirm: true`; refusing implicit project mutation.",
        );
    }
    let supplied_digest = body
        .get("plan_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if supplied_digest.is_empty() {
        return refuse(
            400,
            "apply requires the `plan_digest` of the exact plan you reviewed.",
        );
    }
    let requested = body.get("feature").and_then(Value::as_str);

    // A registered project whose directory is gone must not be re-planned
    // against a stale root: refuse with the honest path state first.
    if let Some(response) = absent_dir_refusal(&record) {
        return response;
    }

    // Recompute the plan the operator reviewed and bind confirmation to it.
    let fresh_plan = match plan_upgrade(&registry, &record.path, requested) {
        Ok(plan) => plan,
        Err(err) => return typed_refusal(&err),
    };
    let fresh_value = serde_json::to_value(&fresh_plan).unwrap_or(Value::Null);
    let fresh_digest = plan_digest(&fresh_value);
    if fresh_digest != supplied_digest {
        // Stale or forged digest: refuse, change nothing, return a new plan.
        return ApiResponse::json(
            409,
            json!({
                "contract": CONTRACT_VERSION,
                "error": {
                    "code": "workbench-plan-stale",
                    "message": "the project no longer matches the confirmed plan; nothing was written. Review the refreshed plan and confirm its new digest.",
                },
                "effect": "none",
                "plan": fresh_value,
                "plan_digest": fresh_digest,
            }),
        );
    }

    // Reuse the shared journaled-operation boundary the bearer `/v1`
    // transport uses, so an accepted write returns a durable operation id
    // with per-stage status and honours the idempotency key.
    let dir = record.path.clone();
    match super::run_with_operation(db_path, APPLY_KIND, id, request, |_op_id, _registry| {
        let mut registry = Registry::open(db_path)?;
        let outcome = apply_upgrade(&mut registry, &dir, requested)?;
        let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
            reason: err.to_string(),
        })?;
        Ok((value, _op_id, id.to_string()))
    }) {
        Ok((outcome, op_id, _project_id)) => ApiResponse::json(
            202,
            json!({
                "contract": CONTRACT_VERSION,
                "accepted": true,
                "operation_id": op_id,
                "project_id": id,
                "outcome": outcome,
                "now": request.started_at.to_rfc3339(),
            }),
        ),
        Err(response) => response,
    }
}

/// Map a typed Core error to the workbench's safe error envelope. The
/// Core `Display` text can name an absolute filesystem path (e.g.
/// `PathUnavailable`), so it is never echoed to the browser: only the
/// stable machine code and a code-keyed, path-free message leave here. The
/// code families match the CLI/Core so the failure boundary is stable.
fn typed_refusal(err: &ForgeError) -> ApiResponse {
    let code = err.code();
    let (status, message) = match code {
        "unknown-project" => (
            404,
            "the selected project is not managed by this Forge registry; nothing was changed.",
        ),
        "path-unavailable" => (
            404,
            "the registered project directory is not currently available on this host; nothing was changed.",
        ),
        "unknown-feature" => (
            400,
            "the requested feature is not a known catalog feature; nothing was changed.",
        ),
        "manifest-invalid" | "manifest-not-found" => (
            400,
            "the project manifest could not be read as a valid schema-1 manifest; nothing was changed.",
        ),
        _ => (
            409,
            "this operation cannot be completed for the selected project right now; nothing was changed.",
        ),
    };
    ApiResponse::json(
        status,
        json!({
            "contract": CONTRACT_VERSION,
            "error": { "code": code, "message": message },
            "effect": "none",
        }),
    )
}

/// A recorded project whose root directory is currently gone must not be
/// re-read by Core (which would re-interpret the stale path as an unknown
/// project). The workbench names that honest state itself, path-free,
/// before any plan or apply runs; nothing is changed either way.
fn absent_dir_refusal(record: &crate::registry::ProjectRecord) -> Option<ApiResponse> {
    if record.available && Path::new(&record.path).is_dir() {
        return None;
    }
    Some(typed_refusal(&ForgeError::PathUnavailable {
        path: record.path.clone(),
    }))
}

fn refuse(status: u16, reason: &str) -> ApiResponse {
    ApiResponse::json(
        status,
        json!({
            "contract": CONTRACT_VERSION,
            "error": { "code": code_for(status), "message": reason },
            "effect": "none",
        }),
    )
}

fn unavailable() -> ApiResponse {
    refuse(
        503,
        "the workbench could not open the registry; no files, registry rows or journal entries were changed.",
    )
}

fn code_for(status: u16) -> &'static str {
    match status {
        400 => "workbench-invalid",
        404 => "workbench-unmanaged-project",
        409 => "workbench-confirm-required",
        503 => "workbench-unavailable",
        _ => "workbench-unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disposition_uses_the_catalog_as_its_single_source_of_truth() {
        let rows = disposition();
        // Every scoped command-tree id must resolve to a real catalog row —
        // a renamed or removed command is a task-3.1 completeness failure.
        assert_eq!(
            rows.len(),
            SCOPED_COMMANDS.len(),
            "every scoped id must exist in the command catalog"
        );
        let find = |id: &str| rows.iter().find(|row| row["id"] == id).expect("scoped row");
        for web_id in ["inspect", "doctor", "upgrade"] {
            let row = find(web_id);
            assert_eq!(row["available"], json!(true), "{web_id} must be web");
            assert_eq!(row["availability"], json!("web"));
            assert!(
                row["route"]
                    .as_str()
                    .unwrap()
                    .starts_with("GET /v1/admin/projects"),
                "{web_id} must name its typed GET route: {row}"
            );
        }
        // Global, id-less `web` rows (creation/registration/adoption) are
        // catalog-visible but not actionable inside one project's workbench:
        // they resolve to a dashboard route, not a per-project one, and carry
        // a synthesized reason that points the operator there.
        for global_id in ["register", "import", "new"] {
            let row = find(global_id);
            assert_eq!(
                row["available"],
                json!(false),
                "{global_id} is not per-project"
            );
            assert_eq!(row["availability"], json!("web"));
            assert!(
                row["route"]
                    .as_str()
                    .unwrap()
                    .starts_with("POST /v1/admin/projects/"),
                "{global_id} must name its global POST route: {row}"
            );
            assert!(
                row["reason"].as_str().unwrap().chars().count() > 10,
                "{global_id} must carry a plain-language reason"
            );
        }
        for cli_id in ["gate", "test"] {
            let row = find(cli_id);
            assert_eq!(row["available"], json!(false), "{cli_id} must stay honest");
            assert!(!row["availability"].as_str().unwrap().eq("web"));
            assert!(
                !row["reason"].as_str().unwrap_or("").is_empty(),
                "{cli_id} must carry the catalog's plain-language reason"
            );
            assert_eq!(row["route"], Value::Null);
        }
    }

    #[test]
    fn plan_digest_is_stable_lowercase_hex_and_binds_the_exact_plan() {
        let plan = json!({"project_id": "demo", "steps": [{"feature": "auth"}]});
        let digest = plan_digest(&plan);
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(digest, plan_digest(&plan), "same bytes must hash same");
        assert_ne!(digest, plan_digest(&json!({"project_id": "demo"})));
    }

    #[test]
    fn shell_metacharacter_ids_are_refused_before_any_filesystem_access() {
        let dir = tempfile::tempdir().unwrap();
        let registry = Registry::open(&dir.path().join("registry.db")).unwrap();
        for id in [
            "proj; rm -rf /",
            "$(whoami)",
            "../../etc/passwd",
            "a|b",
            "c\nd",
            "proj && curl evil.test",
            "back`tick`",
            "UPPER-case",
            "",
        ] {
            match resolve(&registry, id) {
                Resolved::Refused { status, reason } => {
                    assert_eq!(status, 400, "id {id:?} must be a typed 400 refusal");
                    assert!(
                        !reason.contains(id) || id.is_empty(),
                        "the refused id must never be echoed back"
                    );
                }
                Resolved::Managed(_) => panic!("shell-ish id {id:?} must never resolve"),
            }
        }
    }

    #[test]
    fn unknown_valid_ids_refuse_with_the_honest_unmanaged_reason() {
        let dir = tempfile::tempdir().unwrap();
        let registry = Registry::open(&dir.path().join("registry.db")).unwrap();
        match resolve(&registry, "ghost-project") {
            Resolved::Refused { status, reason } => {
                assert_eq!(status, 404);
                assert!(reason.contains("not managed by this Forge registry"));
                assert!(!reason.contains("ghost-project"), "the id is not echoed");
            }
            Resolved::Managed(_) => panic!("unknown id must not resolve"),
        }
    }

    #[test]
    fn journal_detail_redaction_removes_absolute_paths_and_keeps_safe_text() {
        let raw = "project path unavailable: /home/operator/projects/demo/forge.yaml";
        let red = redact_local_paths(raw);
        assert!(!red.contains("/home"), "absolute path leaked: {red}");
        assert!(red.contains("project path unavailable"));
        assert_eq!(
            redact_local_paths("workbench.upgrade.apply completed"),
            "workbench.upgrade.apply completed"
        );
        assert!(redact_local_paths("at C:\\Users\\op\\proj").contains("[local path]"));
        assert!(redact_local_paths("path=/srv/forge/db.sqlite").contains("[local path]"));
    }
}
