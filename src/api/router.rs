//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use chrono::{DateTime, Utc};
use std::net::IpAddr;
use std::path::Path;

use super::contract::API_CONTRACT_VERSION;
use super::handlers_changes::{
    handle_add_feature, handle_agent_transition, handle_create_project, handle_doctor,
    handle_generate_spec, handle_governance, handle_upgrade,
};
use super::handlers_core::{
    authorize, handle_catalog_query, handle_delivery_hermora_retry, handle_delivery_preflight,
    handle_delivery_promote, handle_delivery_stage, handle_delivery_status, handle_github_push,
    handle_inspect_project, handle_list_projects, handle_studio_spec_get,
};
use super::handlers_portfolio::{
    handle_apply_deployment, handle_get_interest, handle_get_operation, handle_get_share,
    handle_import_interest, handle_interest_compare, handle_interest_trend,
    handle_portfolio_evidence, handle_portfolio_project, handle_portfolio_relation,
    handle_portfolio_review, handle_portfolio_tag, handle_remove_share, handle_set_share,
    handle_share_approve, handle_share_audit, handle_share_preview, handle_share_publish,
    handle_share_reconcile,
};
use super::handlers_studio::{
    handle_studio_preview_get, handle_studio_preview_post, handle_studio_refine,
    handle_studio_spec_save,
};
use super::model::{ApiConfig, ApiRequest, ApiResponse, Route};
use super::server::{handle_interest_audit, handle_interest_readiness};

pub(super) fn bind_label(bind: IpAddr) -> String {
    bind.to_string()
}

pub(in crate::api) fn err_status(err: &ForgeError) -> u16 {
    match err.code() {
        "api-unauthorized" | "identity-session-expired" | "identity-session-not-found" => 401,
        "api-project-mismatch"
        | "identity-session-cross-project"
        | "identity-permission-denied" => 403,
        "idempotency-key-conflict"
        | "push-confirm-required"
        | "release-check-failed"
        | "deploy-health-failed"
        | "portfolio-share-conflict"
        | "portfolio-interest-conflict"
        | "delivery-conflict" => 409,
        // The readiness route never constructs the CLI gate error —
        // it answers `200` for both verdicts — but the row keeps a
        // future caller from silently becoming a `500`.
        "portfolio-activation-not-ready" => 409,
        "studio-revision-conflict" => 409,
        "api-invalid"
        | "manifest-invalid"
        | "manifest-not-found"
        | "path-unavailable"
        | "unknown-project"
        | "unknown-profile"
        | "unknown-feature"
        | "incompatible-feature"
        | "feature-ownership-conflict"
        | "spec-invalid"
        | "portfolio-invalid"
        | "portfolio-share-invalid"
        | "portfolio-interest-invalid"
        | "deploy-invalid"
        | "release-invalid"
        | "catalog-invalid"
        | "delivery-invalid"
        | "studio-invalid-spec"
        | "graduation-invalid"
        | "intent-invalid"
        | "intent-ambiguous"
        | "plan-conflict"
        | "plan-apply-failed"
        | "remediation-invalid" => 400,
        "graduation-conflict" | "remediation-conflict" | "plan-stale" => 409,
        // Delivery provider / adapter availability is a transient
        // 503: the registry keeps the existing rows intact and the
        // operator can retry without re-running the stage.
        "delivery-unavailable" => 503,
        "studio-port-unavailable" | "studio-start-timeout" => 503,
        _ => 500,
    }
}

pub fn route_request(method: &str, path: &str) -> Option<Route> {
    let path = path.split('?').next().unwrap_or(path);
    let path = path.trim_end_matches('/');
    let normalized = if path.is_empty() { "/" } else { path };
    let segments: Vec<&str> = normalized.trim_start_matches('/').split('/').collect();
    match (method, segments.as_slice()) {
        ("GET", ["v1", "admin", "session"]) => Some(Route::AdminSessionGet),
        ("POST", ["v1", "admin", "session"]) => Some(Route::AdminSessionPost),
        ("DELETE", ["v1", "admin", "session"]) => Some(Route::AdminSessionDelete),
        ("GET", ["v1", "admin", "projects"]) => Some(Route::AdminProjects),
        ("GET", ["v1", "admin", "commands"]) => Some(Route::AdminCommands),
        // Fleet readiness: a three-segment admin path. It precedes the
        // generic `["v1","admin",_]` OPTIONS wildcard below, which never
        // matches a GET anyway.
        ("GET", ["v1", "admin", "status"]) => Some(Route::AdminFleetStatus),
        // Project creation/registration: literal four-segment paths addressed
        // by a validated project name that the server joins to its own
        // configured root. The browser never supplies a filesystem path. These
        // precede the generic `{id}` and OPTIONS wildcards so a reserved
        // literal is never read as a project id.
        ("POST", ["v1", "admin", "projects", "new"]) => Some(Route::AdminProjectNew),
        ("POST", ["v1", "admin", "projects", "import"]) => Some(Route::AdminProjectImport),
        ("POST", ["v1", "admin", "projects", "register"]) => Some(Route::AdminProjectRegister),
        ("GET", ["v1", "admin", "workspace", "candidates"]) => {
            Some(Route::AdminWorkspaceCandidates)
        }
        ("POST", ["v1", "admin", "workspace", "onboard"]) => Some(Route::AdminWorkspaceOnboard),
        // Workbench routes are addressed by a validated project id resolved
        // server-side; the browser never sends a filesystem path. These arms
        // precede the generic admin OPTIONS handling so a `{id}` segment is
        // captured rather than swallowed by the `_` wildcard below.
        ("GET", ["v1", "admin", "projects", id]) => Some(Route::AdminProjectDetail {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "admin", "projects", id, "plan"]) => Some(Route::AdminProjectPlan {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "admin", "projects", id, "apply"]) => Some(Route::AdminProjectApply {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "admin", "projects", id, "feature"]) => Some(Route::AdminProjectFeature {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "admin", "projects", id, "spec"]) => Some(Route::AdminProjectSpec {
            id: (*id).to_string(),
        }),
        // Deeper lifecycle-write paths: `feature/remove`, `feature/upgrade`
        // and `spec/apply` are six-segment arms. The two-segment `feature`/
        // `spec` arms above are fixed-length slice patterns and still resolve
        // for the five-segment paths; these longer arms match only the six-
        // segment forms, so no existing route is shadowed.
        ("POST", ["v1", "admin", "projects", id, "feature", "remove"]) => {
            Some(Route::AdminProjectFeatureRemove {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "feature", "upgrade"]) => {
            Some(Route::AdminProjectFeatureUpgrade {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "spec", "apply"]) => {
            Some(Route::AdminProjectSpecApply {
                id: (*id).to_string(),
            })
        }
        // Deploy routes: `POST …/deploy` is a five-segment arm and
        // `GET …/deploy/plan` a six-segment arm. The literal `deploy`
        // segment never collides with the `feature`/`spec`/`plan`/`apply`
        // arms above, so no existing route is shadowed.
        ("POST", ["v1", "admin", "projects", id, "deploy"]) => Some(Route::AdminProjectDeploy {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "admin", "projects", id, "deploy", "plan"]) => {
            Some(Route::AdminProjectDeployPlan {
                id: (*id).to_string(),
            })
        }
        // Release routes: `POST …/release` is a five-segment arm and
        // `GET …/release/plan` a six-segment arm. The literal `release`
        // segment never collides with the `deploy`/`feature`/`spec`/`plan`/
        // `apply` arms above, so no existing route is shadowed.
        ("POST", ["v1", "admin", "projects", id, "release"]) => Some(Route::AdminProjectRelease {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "admin", "projects", id, "release", "plan"]) => {
            Some(Route::AdminProjectReleasePlan {
                id: (*id).to_string(),
            })
        }
        // Publish routes: `POST …/publish` is a five-segment arm and
        // `GET …/publish/plan` a six-segment arm. The literal `publish`
        // segment never collides with the `release`/`deploy`/`feature`/`spec`/
        // `plan`/`apply` arms above, so no existing route is shadowed.
        ("POST", ["v1", "admin", "projects", id, "publish"]) => Some(Route::AdminProjectPublish {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "admin", "projects", id, "publish", "plan"]) => {
            Some(Route::AdminProjectPublishPlan {
                id: (*id).to_string(),
            })
        }
        // Project delivery routes: the five-segment status arm and the
        // six-segment staged-mutation arms. The literal `delivery` segment
        // never collides with the lifecycle arms above.
        ("GET", ["v1", "admin", "projects", id, "delivery", "status"]) => {
            Some(Route::AdminProjectDeliveryStatus {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "delivery", "preflight"]) => {
            Some(Route::AdminProjectDeliveryPreflight {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "delivery", "stage"]) => {
            Some(Route::AdminProjectDeliveryStage {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "delivery", "promote"]) => {
            Some(Route::AdminProjectDeliveryPromote {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "delivery", "hermora-retry"]) => {
            Some(Route::AdminProjectDeliveryHermoraRetry {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "graduation", "preview"]) => Some(Route::AdminGraduationPreview),
        ("POST", ["v1", "admin", "graduation", "import"]) => Some(Route::AdminGraduationImport),
        ("POST", ["v1", "admin", "projects", id, "intent", "resolve"]) => {
            Some(Route::AdminProjectIntentResolve {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "intent", "apply"]) => {
            Some(Route::AdminProjectIntentApply {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "remediate", "plan"]) => {
            Some(Route::AdminProjectRemediatePlan {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "remediate", "apply"]) => {
            Some(Route::AdminProjectRemediateApply {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "delivery", "next-idea"]) => {
            Some(Route::AdminProjectDeliveryNextIdea {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "studio", "spec-save"]) => {
            Some(Route::AdminProjectStudioSpecSave {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "studio", "refine-admin"]) => {
            Some(Route::AdminProjectStudioRefine {
                id: (*id).to_string(),
            })
        }
        // Project status is a five-segment read. Its literal `status`
        // segment never collides with the `plan`/`apply`/`feature`/`spec`/
        // `deploy` arms above, so no existing route is shadowed.
        ("GET", ["v1", "admin", "projects", id, "status"]) => Some(Route::AdminProjectStatus {
            id: (*id).to_string(),
        }),
        // Maintainer surface: the read-only per-project projection plus
        // the three preview → confirm → apply decision routes. Literal
        // `maintain`/`classify` segments never collide with the
        // lifecycle arms above, so no existing route is shadowed.
        ("GET", ["v1", "admin", "projects", id, "maintain"]) => Some(Route::AdminProjectMaintain {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "admin", "projects", id, "classify", "approve"]) => {
            Some(Route::AdminProjectClassifyApprove {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "classify", "reject"]) => {
            Some(Route::AdminProjectClassifyReject {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "classify", "apply"]) => {
            Some(Route::AdminProjectClassifyApply {
                id: (*id).to_string(),
            })
        }
        // Portfolio routes: `/evidence` is a reserved second segment and is
        // matched before the generic `{id}` arm so a literal path never reads
        // as a project id. `{kind}`/`{action}` are validated keys, not paths.
        ("GET", ["v1", "admin", "portfolio"]) => Some(Route::AdminPortfolioList),
        ("GET", ["v1", "admin", "portfolio", "evidence"]) => Some(Route::AdminPortfolioEvidence),
        ("GET", ["v1", "admin", "portfolio", id]) => Some(Route::AdminPortfolioProject {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "admin", "portfolio", id, kind]) => Some(Route::AdminPortfolioRead {
            id: (*id).to_string(),
            kind: (*kind).to_string(),
        }),
        ("POST", ["v1", "admin", "portfolio", id, action]) => Some(Route::AdminPortfolioWrite {
            id: (*id).to_string(),
            action: (*action).to_string(),
        }),
        // Delivery controls: `preview`, `approve`, `publish` and
        // `reconcile` are reserved literal segments under
        // `/v1/admin/delivery`; `allowlist/{id}` and `operation/{key}`
        // capture only validated opaque tokens — never a path the server
        // would open. The arms precede the generic OPTIONS wildcards.
        ("GET", ["v1", "admin", "delivery"]) => Some(Route::AdminDelivery),
        ("GET", ["v1", "admin", "delivery", "preview"]) => Some(Route::AdminDeliveryPreview),
        ("GET", ["v1", "admin", "delivery", "operation", key]) => {
            Some(Route::AdminDeliveryOperation {
                key: (*key).to_string(),
            })
        }
        ("POST", ["v1", "admin", "delivery", "approve"]) => Some(Route::AdminDeliveryApprove),
        ("POST", ["v1", "admin", "delivery", "publish"]) => Some(Route::AdminDeliveryPublish),
        ("POST", ["v1", "admin", "delivery", "reconcile"]) => Some(Route::AdminDeliveryReconcile),
        ("POST", ["v1", "admin", "delivery", "allowlist", id]) => {
            Some(Route::AdminDeliveryAllowlist {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "delivery", "allowlist", id, "remove"]) => {
            Some(Route::AdminDeliveryAllowlistRemove {
                id: (*id).to_string(),
            })
        }
        // CORS preflight for the deeper admin paths: the generic
        // `["v1","admin",_]` arm below only matches the three-segment admin
        // paths, so the workbench's four- and five-segment paths need their
        // own OPTIONS arms.
        ("OPTIONS", ["v1", "admin", "projects", _]) => Some(Route::AdminOptions),
        ("OPTIONS", ["v1", "admin", "projects", _, _]) => Some(Route::AdminOptions),
        // The lifecycle-write routes (`feature/remove`, `feature/upgrade`,
        // `spec/apply`) are six-segment paths, so their CORS preflight needs a
        // matching six-segment OPTIONS arm; the four- and five-segment arms
        // above never match a six-segment request.
        ("OPTIONS", ["v1", "admin", "projects", _, _, _]) => Some(Route::AdminOptions),
        ("OPTIONS", ["v1", "admin", "portfolio", _]) => Some(Route::AdminOptions),
        ("OPTIONS", ["v1", "admin", "portfolio", _, _]) => Some(Route::AdminOptions),
        // Delivery preflights run at three to six segments
        // (`allowlist/{id}/remove`), so one slice-tail arm covers them
        // before the generic three-segment admin wildcard below.
        ("OPTIONS", ["v1", "admin", "delivery", ..]) => Some(Route::AdminOptions),
        ("OPTIONS", ["v1", "admin", "workspace", ..]) => Some(Route::AdminOptions),
        ("OPTIONS", ["v1", "admin", "graduation", ..]) => Some(Route::AdminOptions),
        ("OPTIONS", ["v1", "admin", _]) => Some(Route::AdminOptions),
        ("GET", ["healthz"]) => Some(Route::Healthz),
        ("GET", ["v1", "projects"]) => Some(Route::ListProjects),
        ("POST", ["v1", "projects"]) => Some(Route::CreateProject),
        // `catalog` is a reserved path segment, not a project id, so
        // this arm must precede the generic `["v1", "projects", id]`
        // inspect match below. The same applies to the per-id
        // catalog inspect at the end of the table.
        ("GET", ["v1", "projects", "catalog"]) => Some(Route::CatalogQuery { id: None }),
        ("GET", ["v1", "projects", id]) => Some(Route::InspectProject {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "doctor"]) => Some(Route::Doctor {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "projects", id, "governance"]) => Some(Route::Governance {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "features"]) => Some(Route::AddFeature {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "upgrade"]) => Some(Route::UpgradeProject {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "specs"]) => Some(Route::GenerateSpec {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "agents"]) => Some(Route::AgentTransition {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "deployments"]) => Some(Route::ApplyDeployment {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "publish", "github"]) => Some(Route::GitHubPush),
        ("GET", ["v1", "operations", op_id]) => op_id
            .parse::<i64>()
            .ok()
            .map(|id| Route::GetOperation { op_id: id }),
        ("GET", ["v1", "projects", id, "portfolio"]) => Some(Route::PortfolioProject {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "portfolio", "tags"]) => Some(Route::PortfolioTag {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "portfolio", "relations"]) => {
            Some(Route::PortfolioRelation {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "projects", id, "portfolio", "reviews"]) => Some(Route::PortfolioReview {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "portfolio", "evidence"]) => {
            Some(Route::PortfolioEvidence {
                id: (*id).to_string(),
            })
        }
        ("GET", ["v1", "projects", id, "share"]) => Some(Route::GetShare {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "share"]) => Some(Route::SetShare {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "share", "remove"]) => Some(Route::RemoveShare {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "share", "manifest"]) => Some(Route::SharePreview),
        ("POST", ["v1", "share", "approve"]) => Some(Route::ShareApprove),
        ("POST", ["v1", "share", "publish"]) => Some(Route::SharePublish),
        ("POST", ["v1", "share", "reconcile"]) => Some(Route::ShareReconcile),
        ("GET", ["v1", "share", "audit"]) => Some(Route::ShareAudit),
        ("GET", ["v1", "projects", id, "interest"]) => Some(Route::GetInterest {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "interest"]) => Some(Route::ImportInterest {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "interest", "compare"]) => Some(Route::InterestCompare),
        ("GET", ["v1", "interest", "trend"]) => Some(Route::InterestTrend),
        ("GET", ["v1", "interest", "audit"]) => Some(Route::InterestAudit),
        ("GET", ["v1", "interest", "readiness"]) => Some(Route::InterestReadiness),
        ("GET", ["v1", "projects", id, "catalog"]) => Some(Route::CatalogQuery {
            id: Some((*id).to_string()),
        }),
        ("GET", ["v1", "projects", id, "delivery"]) => Some(Route::DeliveryStatus {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "delivery", "preflight"]) => {
            Some(Route::DeliveryPreflight {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "projects", id, "delivery", "stage"]) => Some(Route::DeliveryStage {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "delivery", "promote"]) => Some(Route::DeliveryPromote {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "delivery", "hermora", "retry"]) => {
            Some(Route::DeliveryHermoraRetry {
                id: (*id).to_string(),
            })
        }
        ("GET", ["v1", "projects", id, "studio", "spec"]) => Some(Route::StudioSpec {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "studio", "spec"]) => Some(Route::StudioSpecSave {
            id: (*id).to_string(),
        }),
        ("GET", ["v1", "projects", id, "studio", "preview"]) => Some(Route::StudioPreviewGet {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "studio", "preview"]) => Some(Route::StudioPreviewPost {
            id: (*id).to_string(),
        }),
        ("POST", ["v1", "projects", id, "studio", "refine"]) => Some(Route::StudioRefine {
            id: (*id).to_string(),
        }),
        _ => None,
    }
}

fn method_not_allowed() -> ApiResponse {
    ApiResponse::json(
        405,
        serde_json::json!({
            "error": {
                "code": "method-not-allowed",
                "message": "method not allowed for this route; see /healthz for the advertised contract"
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

fn not_found() -> ApiResponse {
    ApiResponse::json(
        404,
        serde_json::json!({
            "error": {
                "code": "route-not-found",
                "message": "no API route matches the request"
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

pub(super) fn bad_request(reason: &str) -> ApiResponse {
    ApiResponse::json(
        400,
        serde_json::json!({
            "error": {
                "code": "api-invalid",
                "message": reason
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// Decide whether the request requires a session and, if
/// so, what permission is needed for the action.
pub(super) fn required_permission(route: &Route) -> Option<&'static str> {
    match route {
        Route::Healthz | Route::GetOperation { .. } | Route::GitHubPush => None,
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
        | Route::AdminOptions => None,
        Route::ListProjects
        | Route::InspectProject { .. }
        | Route::Doctor { .. }
        | Route::Governance { .. } => None,
        Route::PortfolioProject { .. } => None,
        Route::CreateProject
        | Route::AddFeature { .. }
        | Route::UpgradeProject { .. }
        | Route::GenerateSpec { .. }
        | Route::AgentTransition { .. }
        | Route::PortfolioTag { .. }
        | Route::PortfolioRelation { .. }
        | Route::PortfolioReview { .. }
        | Route::PortfolioEvidence { .. }
        | Route::ApplyDeployment { .. }
        // Every share route, preview included, demands admin:access:
        // previewing the candidate manifest reveals which projects an
        // operator considers publishable, which is itself private.
        | Route::GetShare { .. }
        | Route::SetShare { .. }
        | Route::RemoveShare { .. }
        | Route::SharePreview
        | Route::ShareApprove
        | Route::SharePublish
        | Route::ShareReconcile
        | Route::ShareAudit => Some("admin:access"),
        // Every interest route, the reads included, demands
        // admin:access: what an operator learns about which projects
        // draw interest, and which of them are drawing none, is the
        // private half of a portfolio decision.
        | Route::GetInterest { .. }
        | Route::ImportInterest { .. }
        | Route::InterestCompare
        | Route::InterestTrend
        | Route::InterestAudit
        | Route::InterestReadiness => Some("admin:access"),
        // Catalog query is a read-only fleet projection; same
        // authorization posture as `Route::ListProjects` and the
        // project detail page (any session, no extra permission).
        | Route::CatalogQuery { .. } => None,
        // Delivery routes are admin-gated: preflight / stage / promote
        // trigger provider invocations, and the status projection
        // surfaces the same private data the per-project journal
        // shows. The reads are deliberately as restricted as the writes.
        | Route::DeliveryStatus { .. }
        | Route::DeliveryPreflight { .. }
        | Route::DeliveryStage { .. }
        | Route::DeliveryPromote { .. }
        | Route::DeliveryHermoraRetry { .. } => Some("admin:access"),
        // Studio routes are project-scoped: a session for the
        // matching project can read the spec/preview state and
        // submit a refinement. The reads are deliberately as
        // restricted as the writes.
        | Route::StudioSpec { .. }
        | Route::StudioSpecSave { .. }
        | Route::StudioPreviewGet { .. }
        | Route::StudioPreviewPost { .. }
        | Route::StudioRefine { .. } => None,
    }
}

/// Decide whether the route is a mutating call. Mutating
/// routes journal a `pending` operation up front and
/// return `202 Accepted`; read-only routes return
/// `200 OK` synchronously. Surfaced for the
/// authorization layer so it can pick the right session
/// permission.
#[allow(dead_code)]
pub(super) fn is_mutating(route: &Route) -> bool {
    matches!(
        route,
        Route::CreateProject
            | Route::AddFeature { .. }
            | Route::UpgradeProject { .. }
            | Route::GenerateSpec { .. }
            | Route::AgentTransition { .. }
            | Route::ApplyDeployment { .. }
    )
}

/// Dispatch one request through Core. The caller is
/// responsible for parsing the wire bytes into
/// [`ApiRequest`] and for rendering the returned
/// [`ApiResponse`] back on the socket; this function
/// holds the entire business contract.
pub fn handle(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    let route = match route_request(&request.method, &request.path) {
        Some(route) => route,
        None => {
            // The path matched no route at all, but a path
            // like `/v1/projects/{id}/doctor` with the
            // wrong method should still surface as 405
            // rather than 404 when the path shape is
            // recognised.
            if let Some(alt) = route_request(alt_method(&request.method), &request.path) {
                let _ = alt;
                return method_not_allowed();
            }
            return not_found();
        }
    };

    if matches!(route, Route::AdminOptions) {
        return super::admin::handle_preflight(config, request);
    }

    if matches!(
        route,
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
    ) {
        return super::admin::handle(config, db_path, request, &route);
    }

    // 1. Authorization: every route (other than /healthz
    // and /v1/operations/{id}) demands a session. Read
    // routes demand any valid session; mutating routes
    // demand admin:access. A token for project A cannot
    // authorize project B.
    let actor = match authorize(config, db_path, &route, request, now) {
        Ok(actor) => actor,
        Err(response) => return response,
    };

    // 2. Dispatch.
    match route.clone() {
        Route::Healthz => ApiResponse::json(
            200,
            serde_json::json!({
                "status": "ok",
                "contract": API_CONTRACT_VERSION,
                "bind": request
                    .remote_addr
                    .map(|a| a.ip().to_string())
                    .unwrap_or_default(),
            }),
        ),
        Route::AdminSessionGet
        | Route::AdminSessionPost
        | Route::AdminSessionDelete
        | Route::AdminProjects
        | Route::AdminCommands
        | Route::AdminOptions => super::admin::handle(config, db_path, request, &route),
        Route::ListProjects => handle_list_projects(db_path),
        Route::CreateProject => handle_create_project(db_path, request, now),
        Route::InspectProject { id } => handle_inspect_project(db_path, &id),
        Route::Doctor { id } => handle_doctor(db_path, &id, request, now),
        Route::Governance { id } => handle_governance(db_path, &id),
        Route::AddFeature { id } => handle_add_feature(db_path, &id, request, now),
        Route::UpgradeProject { id } => handle_upgrade(db_path, &id, request, now),
        Route::GenerateSpec { id } => handle_generate_spec(db_path, &id, request, now),
        Route::AgentTransition { id } => handle_agent_transition(db_path, &id, request, now),
        Route::ApplyDeployment { id } => handle_apply_deployment(db_path, &id, request, now),
        Route::GitHubPush => handle_github_push(db_path, request),
        Route::GetOperation { op_id } => handle_get_operation(db_path, op_id),
        Route::PortfolioProject { id } => handle_portfolio_project(db_path, &id, now),
        Route::PortfolioTag { id } => handle_portfolio_tag(db_path, request, &id),
        Route::PortfolioRelation { id } => handle_portfolio_relation(db_path, request, &id),
        Route::PortfolioReview { id } => handle_portfolio_review(db_path, request, &id),
        Route::PortfolioEvidence { id } => handle_portfolio_evidence(db_path, request, &id),
        Route::GetShare { id } => handle_get_share(db_path, &id),
        Route::SetShare { id } => handle_set_share(db_path, request, &id),
        Route::RemoveShare { id } => handle_remove_share(db_path, &id),
        Route::SharePreview => handle_share_preview(db_path, now),
        Route::ShareApprove => handle_share_approve(db_path, request, &actor),
        Route::SharePublish => handle_share_publish(db_path, request, &actor, now),
        Route::ShareReconcile => handle_share_reconcile(db_path, request, &actor),
        Route::ShareAudit => handle_share_audit(db_path, request),
        Route::GetInterest { id } => handle_get_interest(db_path, request, &id, now),
        Route::ImportInterest { id } => handle_import_interest(db_path, request, &id, &actor, now),
        Route::InterestCompare => handle_interest_compare(db_path, request, now),
        Route::InterestTrend => handle_interest_trend(db_path, request, now),
        Route::InterestAudit => handle_interest_audit(db_path, request),
        Route::InterestReadiness => handle_interest_readiness(db_path, request, now),
        Route::CatalogQuery { id } => handle_catalog_query(db_path, request, id.as_deref(), now),
        Route::DeliveryStatus { id } => handle_delivery_status(db_path, &id, now),
        Route::DeliveryPreflight { id } => handle_delivery_preflight(db_path, &id, now),
        Route::DeliveryStage { id } => handle_delivery_stage(db_path, request, &id, now),
        Route::DeliveryPromote { id } => handle_delivery_promote(db_path, request, &id, now),
        Route::DeliveryHermoraRetry { id } => {
            handle_delivery_hermora_retry(db_path, request, &id, now)
        }
        Route::StudioSpec { id } => handle_studio_spec_get(db_path, &id),
        Route::StudioSpecSave { id } => handle_studio_spec_save(db_path, request, &id, now),
        Route::StudioPreviewGet { id } => handle_studio_preview_get(db_path, &id),
        Route::StudioPreviewPost { id } => {
            handle_studio_preview_post(config, db_path, request, &id, now)
        }
        Route::StudioRefine { id } => handle_studio_refine(db_path, request, &id, now),
        // The workbench admin routes are dispatched by the `admin::handle`
        // short-circuit above (after the global-session gate and exact-origin
        // CORS check), so these arms are unreachable in practice. They exist
        // for exhaustiveness and answer `404` rather than a bearer-scoped
        // handler, since a request that reached here did not go through the
        // admin session gate and must not be served workbench data.
        Route::AdminFleetStatus
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
        | Route::AdminProjectStudioRefine { .. } => not_found(),
    }
}

fn alt_method(method: &str) -> &'static str {
    if method.eq_ignore_ascii_case("GET") {
        "POST"
    } else {
        "GET"
    }
}
