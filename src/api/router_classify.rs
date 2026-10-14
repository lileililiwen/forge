//! Per-variant classification of the `Route` enum (`forge-api-router-table-driven`).
//!
//! Three exhaustive `match` expressions answer the three questions the
//! cross-cutting `match` arms in `src/api/router.rs` used to answer
//! inline. The cross-cutting arms grew by 1–2 lines per new variant,
//! and the cap (1000 lines for `router.rs`) was exceeded at the
//! `Route::AdminProjectGateStatus` / `Route::AdminProjectGateRun`
//! addition (prior subagent's `forge-web-gate-controls`).
//!
//! Moving the classification beside the dispatch file — instead of
//! onto the `Route` enum in `src/api/model.rs` — preserves the
//! "dispatch lives in `router.rs`" boundary. The three functions
//! are exhaustive, so adding a new `Route` variant is a compile
//! error in this file (the same compile error the cross-cutting
//! arms produced), but the error is centralized in one place
//! instead of being spread across three arms in `router.rs`.
//!
//! Every classification is byte-identical to the pre-refactor
//! cross-cutting `match` arms:
//! - [`required_permission`] is the pre-refactor
//!   `src/api/router.rs:566-706` `required_permission` function.
//! - [`is_admin_dispatch`] is the pre-refactor
//!   `src/api/router.rs:758-841` `matches!` arm (every
//!   `Route::Admin*` variant that short-circuits to
//!   `super::admin::handle`).
//! - [`is_admin_route`] is the pre-refactor
//!   `src/api/router.rs:924-996` final-dispatch fallthrough (every
//!   admin variant that returns `not_found()` if the pre-dispatch
//!   did not claim it).
//! - [`admin_fallthrough`] returns the existing `not_found()`
//!   envelope for the unreachable load-bearing exhaustiveness check.
//!
//! The next admin variant is registered in one place:
//! `src/api/admin/<thing>.rs` (path arm beside the handler), the
//! `Route` enum (one new variant), and three arms in this file
//! (one per `match` below) — zero new lines in `src/api/router.rs`.

use super::model::Route;
use super::ApiResponse;

/// Decide whether the request requires a session and, if so, what
/// permission is needed for the action. Mirrors the pre-refactor
/// `src/api/router.rs::required_permission` byte-for-byte.
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
        | Route::AdminProjectHealthRefresh { .. }
        | Route::AdminProjectGateStatus { .. }
        | Route::AdminProjectGateRun { .. }
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
        | Route::AdminProjectReleaseHistory { .. }
        | Route::AdminProjectReleaseInspect { .. }
        | Route::AdminProjectDeployHistory { .. }
        | Route::AdminProjectDeployInspect { .. }
        | Route::AdminProjectDeployStatus { .. }
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
        | Route::AdminPortfolioTagRemove { .. }
        | Route::AdminPortfolioRelationRemove { .. }
        | Route::AdminPortfolioEvidenceImport { .. }
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
        | Route::AdminCatalog
        | Route::AdminCatalogTags
        | Route::AdminCatalogLanguages
        | Route::AdminCatalogGaps
        | Route::AdminCatalogInspect { .. }
        | Route::AdminFleetInspect { .. }
        | Route::AdminProjectAgents { .. }
        | Route::AdminProjectIdentityConfig { .. }
        | Route::AdminProjectIdentitySessions { .. }
        | Route::AdminCreation { .. }
        | Route::AdminIdentityChangePassword
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

/// True for every variant that short-circuits to
/// `super::admin::handle(config, db_path, request, &route)` in
/// `src/api/router.rs::handle`. Mirrors the pre-refactor
/// `src/api/router.rs:758-841` `matches!` arm byte-for-byte:
/// every `Route::Admin*` variant **except** `Route::AdminOptions`
/// (which short-circuits to `super::admin::handle_preflight` at
/// `src/api/router.rs:754-756` and is handled separately).
pub(super) fn is_admin_dispatch(route: &Route) -> bool {
    match route {
        Route::AdminOptions => false,
        Route::AdminSessionGet
        | Route::AdminSessionPost
        | Route::AdminSessionDelete
        | Route::AdminProjects
        | Route::AdminCommands
        | Route::AdminCatalog
        | Route::AdminCatalogTags
        | Route::AdminCatalogLanguages
        | Route::AdminCatalogGaps
        | Route::AdminCatalogInspect { .. }
        | Route::AdminFleetInspect { .. }
        | Route::AdminCreation { .. }
        | Route::AdminFleetStatus
        | Route::AdminProjectDetail { .. }
        | Route::AdminProjectStatus { .. }
        | Route::AdminProjectMaintain { .. }
        | Route::AdminProjectClassifyApprove { .. }
        | Route::AdminProjectClassifyReject { .. }
        | Route::AdminProjectClassifyApply { .. }
        | Route::AdminProjectHealthRefresh { .. }
        | Route::AdminProjectGateStatus { .. }
        | Route::AdminProjectGateRun { .. }
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
        | Route::AdminProjectReleaseHistory { .. }
        | Route::AdminProjectReleaseInspect { .. }
        | Route::AdminProjectDeployHistory { .. }
        | Route::AdminProjectDeployInspect { .. }
        | Route::AdminProjectDeployStatus { .. }
        | Route::AdminProjectAgents { .. }
        | Route::AdminProjectIdentityConfig { .. }
        | Route::AdminProjectIdentitySessions { .. }
        | Route::AdminIdentityChangePassword
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
        | Route::AdminPortfolioTagRemove { .. }
        | Route::AdminPortfolioRelationRemove { .. }
        | Route::AdminPortfolioEvidenceImport { .. }
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
        | Route::AdminProjectStudioRefine { .. } => true,
        // Every non-admin variant falls through to the bearer-handler
        // dispatch in `src/api/router.rs::handle`.
        Route::Healthz
        | Route::ListProjects
        | Route::CreateProject
        | Route::InspectProject { .. }
        | Route::Doctor { .. }
        | Route::Governance { .. }
        | Route::AddFeature { .. }
        | Route::UpgradeProject { .. }
        | Route::GenerateSpec { .. }
        | Route::AgentTransition { .. }
        | Route::ApplyDeployment { .. }
        | Route::GitHubPush
        | Route::GetOperation { .. }
        | Route::StudioSpec { .. }
        | Route::StudioSpecSave { .. }
        | Route::StudioPreviewGet { .. }
        | Route::StudioPreviewPost { .. }
        | Route::StudioRefine { .. }
        | Route::PortfolioProject { .. }
        | Route::PortfolioTag { .. }
        | Route::PortfolioRelation { .. }
        | Route::PortfolioReview { .. }
        | Route::PortfolioEvidence { .. }
        | Route::GetShare { .. }
        | Route::SetShare { .. }
        | Route::RemoveShare { .. }
        | Route::SharePreview
        | Route::ShareApprove
        | Route::SharePublish
        | Route::ShareReconcile
        | Route::ShareAudit
        | Route::GetInterest { .. }
        | Route::ImportInterest { .. }
        | Route::InterestCompare
        | Route::InterestTrend
        | Route::InterestAudit
        | Route::InterestReadiness
        | Route::CatalogQuery { .. }
        | Route::DeliveryStatus { .. }
        | Route::DeliveryPreflight { .. }
        | Route::DeliveryStage { .. }
        | Route::DeliveryPromote { .. }
        | Route::DeliveryHermoraRetry { .. } => false,
    }
}

/// True for every admin-classified variant (the union of
/// [`is_admin_dispatch`] plus `Route::AdminOptions`). Mirrors the
/// pre-refactor `src/api/router.rs:924-996` final-dispatch
/// fallthrough arm byte-for-byte. In practice the function is the
/// exhaustiveness companion of [`is_admin_dispatch`] so the final
/// match in `handle()` collapses to a single fallthrough arm.
pub(super) fn is_admin_route(route: &Route) -> bool {
    match route {
        Route::AdminSessionGet
        | Route::AdminSessionPost
        | Route::AdminSessionDelete
        | Route::AdminProjects
        | Route::AdminCommands
        | Route::AdminCatalog
        | Route::AdminCatalogTags
        | Route::AdminCatalogLanguages
        | Route::AdminCatalogGaps
        | Route::AdminCatalogInspect { .. }
        | Route::AdminFleetInspect { .. }
        | Route::AdminCreation { .. }
        | Route::AdminFleetStatus
        | Route::AdminProjectDetail { .. }
        | Route::AdminProjectStatus { .. }
        | Route::AdminProjectMaintain { .. }
        | Route::AdminProjectClassifyApprove { .. }
        | Route::AdminProjectClassifyReject { .. }
        | Route::AdminProjectClassifyApply { .. }
        | Route::AdminProjectHealthRefresh { .. }
        | Route::AdminProjectGateStatus { .. }
        | Route::AdminProjectGateRun { .. }
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
        | Route::AdminProjectReleaseHistory { .. }
        | Route::AdminProjectReleaseInspect { .. }
        | Route::AdminProjectDeployHistory { .. }
        | Route::AdminProjectDeployInspect { .. }
        | Route::AdminProjectDeployStatus { .. }
        | Route::AdminProjectAgents { .. }
        | Route::AdminProjectIdentityConfig { .. }
        | Route::AdminProjectIdentitySessions { .. }
        | Route::AdminIdentityChangePassword
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
        | Route::AdminPortfolioTagRemove { .. }
        | Route::AdminPortfolioRelationRemove { .. }
        | Route::AdminPortfolioEvidenceImport { .. }
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
        | Route::AdminOptions => true,
        // Every non-admin variant.
        Route::Healthz
        | Route::ListProjects
        | Route::CreateProject
        | Route::InspectProject { .. }
        | Route::Doctor { .. }
        | Route::Governance { .. }
        | Route::AddFeature { .. }
        | Route::UpgradeProject { .. }
        | Route::GenerateSpec { .. }
        | Route::AgentTransition { .. }
        | Route::ApplyDeployment { .. }
        | Route::GitHubPush
        | Route::GetOperation { .. }
        | Route::StudioSpec { .. }
        | Route::StudioSpecSave { .. }
        | Route::StudioPreviewGet { .. }
        | Route::StudioPreviewPost { .. }
        | Route::StudioRefine { .. }
        | Route::PortfolioProject { .. }
        | Route::PortfolioTag { .. }
        | Route::PortfolioRelation { .. }
        | Route::PortfolioReview { .. }
        | Route::PortfolioEvidence { .. }
        | Route::GetShare { .. }
        | Route::SetShare { .. }
        | Route::RemoveShare { .. }
        | Route::SharePreview
        | Route::ShareApprove
        | Route::SharePublish
        | Route::ShareReconcile
        | Route::ShareAudit
        | Route::GetInterest { .. }
        | Route::ImportInterest { .. }
        | Route::InterestCompare
        | Route::InterestTrend
        | Route::InterestAudit
        | Route::InterestReadiness
        | Route::CatalogQuery { .. }
        | Route::DeliveryStatus { .. }
        | Route::DeliveryPreflight { .. }
        | Route::DeliveryStage { .. }
        | Route::DeliveryPromote { .. }
        | Route::DeliveryHermoraRetry { .. } => false,
    }
}

/// The final-dispatch fallthrough for an admin route that did not
/// short-circuit. Reachable only via an unreachable load-bearing
/// exhaustiveness check (every admin variant either short-circuits
/// via [`is_admin_dispatch`] or via the `Route::AdminOptions`
/// preflight above the dispatch). Returns the existing
/// `not_found()` envelope — the same refusal the pre-refactor
/// fallthrough produced.
pub(super) fn admin_fallthrough(_route: &Route) -> ApiResponse {
    super::not_found()
}
