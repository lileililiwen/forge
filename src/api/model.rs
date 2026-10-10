//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::catalog;
use crate::core::ForgeError;
use crate::studio::PreviewSession;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::contract::{API_CONTRACT_VERSION, DEFAULT_BIND_ADDR, DEFAULT_PORT, MAX_BODY_BYTES};
use super::router::{bad_request, bind_label, err_status};

/// Decode the `%XX` escapes a query string may carry.
///
/// Parsed query parameters for `GET /v1/projects/catalog` and
/// `GET /v1/projects/{id}/catalog`. Every field is the typed
/// counterpart of a catalog CLI flag; the handler builds the
/// `CatalogQuery` from these without re-parsing the wire format
/// itself.
#[derive(Debug, Clone)]
pub(super) struct CatalogQueryParams {
    pub(super) sources: Vec<String>,
    pub(super) tags: Vec<String>,
    pub(super) languages: Vec<String>,
    pub(super) profiles: Vec<String>,
    pub(super) lifecycles: Vec<String>,
    pub(super) repositories: Vec<String>,
    pub(super) ci: Vec<String>,
    pub(super) compose: Vec<String>,
    pub(super) evidence: Vec<String>,
    pub(super) filters: Vec<String>,
    pub(super) limit: usize,
    pub(super) cursor: Option<String>,
    pub(super) max_age: i64,
    pub(super) workspace_registry: Option<PathBuf>,
    pub(super) inventory: Option<PathBuf>,
    pub(super) git_repositories: Vec<PathBuf>,
    pub(super) github_repositories: Vec<String>,
}
/// Shared shutdown signal. The server loop checks the
/// flag at every accept so the CLI can stop the listener
/// without killing the process.
#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    pub(super) flag: Arc<AtomicBool>,
}
impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn trigger(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }
    pub fn is_set(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}
/// Transport-level parse error. Never escapes the
/// transport: the wire loop renders it as a `400` JSON
/// response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    Parse(String),
    BodyTooLarge { limit: usize, got: usize },
    Io(String),
    Timeout,
}
impl ApiError {
    pub fn to_response(&self) -> ApiResponse {
        match self {
            ApiError::Parse(reason) => bad_request(reason),
            ApiError::BodyTooLarge { limit, got } => ApiResponse::json(
                413,
                serde_json::json!(
                    { "error" : { "code" : "api-body-too-large", "message" :
                    format!("body is {got} bytes; limit is {limit}") }, "contract" :
                    API_CONTRACT_VERSION, }
                ),
            ),
            ApiError::Io(reason) => ApiResponse::json(
                500,
                serde_json::json!(
                    { "error" : { "code" : "api-internal", "message" :
                    format!("transport io error: {reason}") }, "contract" :
                    API_CONTRACT_VERSION, }
                ),
            ),
            ApiError::Timeout => ApiResponse::json(
                408,
                serde_json::json!(
                    { "error" : { "code" : "api-timeout", "message" :
                    "request exceeded the handler timeout" }, "contract" :
                    API_CONTRACT_VERSION, }
                ),
            ),
        }
    }
}
/// Operator-facing API configuration. Bounded to the
/// fields the API surface depends on; the rest of the
/// Forge environment is resolved through existing
/// helpers (registry path, identity directory, …).
#[derive(Clone)]
pub struct ApiConfig {
    pub bind: IpAddr,
    pub port: u16,
    pub max_body_bytes: usize,
    /// Exact browser origin allowed to call the Forge-wide admin API.
    pub frontend_origin: String,
    /// Live Studio preview sessions keyed by project id. Only the
    /// long-lived `serve` loop can host a running preview, so the map
    /// lives on the configuration and drops — killing each child —
    /// when the server shuts down. The CLI is a bounded probe and
    /// never stores here.
    pub previews: Arc<Mutex<HashMap<String, PreviewSession>>>,
    /// Browser OIDC verifier. The default is
    /// [`LibraryBrowserAuthVerifier`], a stub that fails
    /// closed; tests inject a [`FakeBrowserAuthVerifier`]
    /// so the in-process transport can complete a
    /// deterministic round trip without contacting a
    /// provider. The field lives on the configuration so
    /// `ApiConfig::default()` stays a single-line
    /// constructor and so the test surface can swap the
    /// verifier without threading a new parameter
    /// through every handler.
    pub browser_auth_verifier: Arc<dyn crate::identity::BrowserAuthVerifier + Send + Sync>,
}
impl ApiConfig {
    /// Build the configuration from the environment. The
    /// `FORGE_API_BIND` and `FORGE_API_PORT` variables
    /// override the defaults when set; an empty or
    /// unparseable value is ignored so the operator gets
    /// the safe default rather than a panic.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(value) = std::env::var("FORGE_API_BIND") {
            if let Ok(parsed) = value.trim().parse::<IpAddr>() {
                cfg.bind = parsed;
            }
        }
        if let Ok(value) = std::env::var("FORGE_API_PORT") {
            if let Ok(parsed) = value.trim().parse::<u16>() {
                cfg.port = parsed;
            }
        }
        if let Ok(value) = std::env::var("FORGE_FRONTEND_ORIGIN") {
            let value = value.trim().trim_end_matches('/');
            if value.starts_with("http://") || value.starts_with("https://") {
                cfg.frontend_origin = value.to_string();
            }
        }
        cfg
    }
    /// Resolved socket address.
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.bind, self.port)
    }
}
/// One parsed HTTP/1.1 request. Body is a `Vec<u8>` so
/// the handler can decide whether to interpret it as
/// JSON, render it as text, or skip it entirely (for
/// `GET`).
#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub idempotency_key: Option<String>,
    pub bearer_token: Option<String>,
    /// Cookies parsed from the `Cookie` header, keyed by
    /// attribute name. The map is the source of truth for
    /// the browser session id on `/ui` routes; `/v1`
    /// requests intentionally ignore it.
    pub cookies: BTreeMap<String, String>,
    pub remote_addr: Option<SocketAddr>,
    pub started_at: DateTime<Utc>,
}
impl ApiRequest {
    /// Case-insensitive header lookup. Returns the first
    /// matching value, or `None` for unknown names. The
    /// method is `pub(crate)` so the in-process portal UI
    /// handlers can read the same headers the JSON API
    /// already exposes; the wire transport still does its
    /// own case-insensitive parsing in [`parse_request`].
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    /// Decoded JSON body, or an empty `Value::Null` for
    /// empty bodies. The transport never fails on
    /// missing bodies: an empty `GET` body is normal.
    pub fn json_body(&self) -> Value {
        if self.body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&self.body).unwrap_or(Value::Null)
        }
    }
}
/// One rendered HTTP/1.1 response. Status, headers and
/// body are the only fields the transport cares about;
/// every other invariant (valid JSON, project-scoped
/// session, idempotency replay) is enforced by the
/// handler that built the response.
#[derive(Debug, Clone)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
impl ApiResponse {
    pub fn json(status: u16, value: Value) -> Self {
        let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec());
        let mut headers = BTreeMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert(
            "x-forge-contract".to_string(),
            API_CONTRACT_VERSION.to_string(),
        );
        Self {
            status,
            headers,
            body,
        }
    }
    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.insert(name.to_string(), value.into());
        self
    }
    /// Build a typed error response. The JSON body carries
    /// the stable `code` plus the human message so a
    /// caller can render a structured diagnostic without
    /// parsing free-form text.
    pub fn from_error(err: &ForgeError) -> Self {
        let status = err_status(err);
        let body = serde_json::json!(
            { "error" : { "code" : err.code(), "message" : err.to_string(), }, "contract"
            : API_CONTRACT_VERSION, }
        );
        Self::json(status, body)
    }
}
/// Match the request against the route table. Returns
/// `None` for unmatched paths so the handler can render
/// `404`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Healthz,
    ListProjects,
    CreateProject,
    InspectProject {
        id: String,
    },
    Doctor {
        id: String,
    },
    Governance {
        id: String,
    },
    AddFeature {
        id: String,
    },
    UpgradeProject {
        id: String,
    },
    GenerateSpec {
        id: String,
    },
    AgentTransition {
        id: String,
    },
    ApplyDeployment {
        id: String,
    },
    GitHubPush,
    GetOperation {
        op_id: i64,
    },
    AdminSessionGet,
    AdminSessionPost,
    AdminSessionDelete,
    AdminProjects,
    AdminCommands,
    /// `GET /v1/admin/status` — read-only fleet readiness summary
    /// (`forge-project-status/0.1.0`): every registered project counted by
    /// overall state, plus a bounded per-project sample. Session-gated.
    AdminFleetStatus,
    /// `GET /v1/admin/projects/{id}` — typed single-project workbench
    /// detail (`forge-project-workbench/0.1.0`): manifest + doctor health
    /// + journal evidence + honest workflow dispositions. Session-gated.
    AdminProjectDetail {
        id: String,
    },
    /// `GET /v1/admin/projects/{id}/status` — read-only status of one
    /// managed project (`forge-project-status/0.1.0`): its doctor, checker
    /// and profile-readiness sub-checks reduced to one overall state.
    /// Session-gated; the root is resolved server-side from a validated id.
    AdminProjectStatus {
        id: String,
    },
    /// `GET /v1/admin/projects/{id}/maintain` — read-only maintainer
    /// projection (`forge-project-maintain/0.1.0`): the GitHub observation
    /// as Forge last saw it (with freshness, or an honest `unavailable`
    /// with the reason), the derived classification proposals (bounded)
    /// and the configured plugin registry. Session-gated; invokes no
    /// plugin and writes nothing.
    AdminProjectMaintain {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/classify/approve` — preview, then
    /// confirm- and digest-bound approve of one classification proposal.
    AdminProjectClassifyApprove {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/classify/reject` — preview, then
    /// confirm- and digest-bound reject of one classification proposal.
    AdminProjectClassifyReject {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/classify/apply` — preview, then
    /// confirm- and digest-bound apply of the approved set through the
    /// configured metadata plugin (PR mode only).
    AdminProjectClassifyApply {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/health/refresh` — explicit on-demand
    /// full health check (`workbench-health-latency`): the live external
    /// policy pass plus doctor, returning the complete health document.
    /// Read-only effect: no confirm/digest binding, no journal row.
    /// Session-gated; the root is resolved server-side from a validated id.
    AdminProjectHealthRefresh {
        id: String,
    },
    /// `GET /v1/admin/projects/{id}/plan` — side-effect-free upgrade plan
    /// plus the digest a later confirmation must echo.
    AdminProjectPlan {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/apply` — confirm- and digest-bound
    /// upgrade apply, journaled through the shared operation boundary.
    AdminProjectApply {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/feature` — the `forge feature add`
    /// authoring command, exposed as a session-gated, preview + confirm- and
    /// digest-bound typed call to the same `handle_add_feature` Core handler
    /// the bearer `/v1` route and CLI use. The `feature`/`version` fields are
    /// structured values, never a path or argv.
    AdminProjectFeature {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/spec` — the `forge spec generate`
    /// authoring command, exposed as a session-gated, preview + confirm- and
    /// digest-bound typed call to the same `handle_generate_spec` Core handler
    /// the bearer `/v1` route and CLI use. The `findings`/`reason` fields are
    /// structured values, never a path or argv.
    AdminProjectSpec {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/feature/remove` — the `forge feature
    /// remove` lifecycle write, exposed as a session-gated, preview +
    /// confirm- and digest-bound typed call to the same `remove_feature` Core
    /// handler the CLI runs. The `feature` field is a structured value, never
    /// a path or argv.
    AdminProjectFeatureRemove {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/feature/upgrade` — the `forge feature
    /// upgrade` lifecycle write, exposed as a session-gated, preview +
    /// confirm- and digest-bound typed call to the same `upgrade_feature`
    /// Core handler the CLI runs. The `feature`/`version` fields are
    /// structured values, never a path or argv.
    AdminProjectFeatureUpgrade {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/spec/apply` — the `forge spec apply`
    /// lifecycle write, exposed as a session-gated, preview + confirm- and
    /// digest-bound typed call to the same `apply_routing` Core handler the
    /// CLI runs. The `findings`/`reason` fields are structured values; the
    /// finding source is synthesized server-side from the finding-name prefix,
    /// never supplied as a path or argv by the caller.
    AdminProjectSpecApply {
        id: String,
    },
    /// `GET /v1/admin/projects/{id}/deploy/plan` — the read-only deploy plan,
    /// exposed as a session-gated admin route that renders
    /// `deploy::engine::prepare_deploy` for the server-resolved target. It
    /// invokes no adapter, writes nothing and returns a path-free plan view.
    AdminProjectDeployPlan {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/deploy` — the `forge deploy` apply, exposed
    /// as a session-gated, preview + confirm- and digest-bound admin route
    /// delegating to the same `deploy::engine::apply_deploy` the bearer `/v1`
    /// route and CLI run. The descriptor binds the project id and the
    /// server-resolved target — never a path, argv or shell.
    AdminProjectDeploy {
        id: String,
    },
    /// `GET /v1/admin/projects/{id}/release/plan` — the read-only release plan,
    /// exposed as a session-gated admin route that renders
    /// `release::engine::prepare_release` for one typed semver version. It
    /// invokes no adapter, writes nothing, mutates no git state and returns a
    /// path-free plan view plus the confirm digest.
    AdminProjectReleasePlan {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/release` — the `forge release` apply,
    /// exposed as a session-gated, preview + confirm- and digest-bound admin
    /// route delegating to the same `release::engine::apply_release` the CLI
    /// runs, using the manifest's stages. The descriptor binds the project id
    /// and the normalized semver version — never a stage list, path, argv,
    /// remote or shell.
    AdminProjectRelease {
        id: String,
    },
    /// `GET /v1/admin/projects/{id}/publish/plan` — the read-only provider
    /// publish plan, exposed as a session-gated admin route that resolves the
    /// provider id, the provider configuration and the committed git revision
    /// server-side. It invokes no provider, writes nothing and returns a
    /// path-free plan view plus the confirm digest.
    AdminProjectPublishPlan {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/publish` — the `forge publish` apply,
    /// exposed as a session-gated, preview + confirm- and digest-bound admin
    /// route delegating to the same `publish::providers::invoke_provider` the
    /// CLI and the GitHub-push handler run. The descriptor binds the project
    /// id, the server-resolved provider id and the committed revision — never a
    /// path, binary, argv, host, SSH target or credential.
    AdminProjectPublish {
        id: String,
    },
    /// `GET /v1/admin/projects/{id}/delivery/status` — read-only project
    /// delivery status through `delivery::handlers::run_status`. No provider,
    /// adapter or write runs.
    AdminProjectDeliveryStatus {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/delivery/preflight` — confirm- and
    /// digest-bound `forge delivery preflight` through the unchanged Core
    /// handler.
    AdminProjectDeliveryPreflight {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/delivery/stage` — confirm- and
    /// digest-bound `forge delivery stage` through the unchanged Core handler.
    AdminProjectDeliveryStage {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/delivery/promote` — confirm- and
    /// digest-bound `forge delivery promote` through the unchanged Core
    /// handler.
    AdminProjectDeliveryPromote {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/delivery/hermora-retry` — confirm- and
    /// digest-bound `forge delivery hermora-retry` through the unchanged Core
    /// handler. The adapter enrolls a healthy deployment; it never
    /// republishes.
    AdminProjectDeliveryHermoraRetry {
        id: String,
    },
    /// `POST /v1/admin/graduation/preview` — validate artifact text, no write.
    AdminGraduationPreview,
    /// `POST /v1/admin/graduation/import` — digest-bound adopt, journaled.
    AdminGraduationImport,
    /// `POST /v1/admin/projects/{id}/intent/resolve` — plan preview, no receipt.
    AdminProjectIntentResolve {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/intent/apply` — digest-bound apply, journaled.
    AdminProjectIntentApply {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/remediate/plan` — plan preview, no write.
    AdminProjectRemediatePlan {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/remediate/apply` — digest-bound apply, journaled.
    AdminProjectRemediateApply {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/delivery/next-idea` — journal the loop transition.
    AdminProjectDeliveryNextIdea {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/studio/spec-save` — admin-gated Studio spec save.
    AdminProjectStudioSpecSave {
        id: String,
    },
    /// `POST /v1/admin/projects/{id}/studio/refine-admin` — admin-gated Studio refine.
    AdminProjectStudioRefine {
        id: String,
    },
    /// `POST /v1/admin/projects/new` — `forge new` exposed as a session-gated,
    /// preview + confirm/digest-bound admin route. The browser supplies only a
    /// validated project name plus typed fields; the destination is resolved
    /// server-side from `FORGE_ADMIN_PROJECTS_ROOT`.
    AdminProjectNew,
    /// `POST /v1/admin/projects/import` — `forge import`, destination resolved
    /// server-side; the browser never supplies a path.
    AdminProjectImport,
    /// `POST /v1/admin/projects/register` — `forge register`, destination
    /// resolved server-side; the browser never supplies a path.
    AdminProjectRegister,
    /// `GET /v1/admin/workspace/candidates` — live, read-only discovery of
    /// sibling directories under the configured project root.
    AdminWorkspaceCandidates,
    /// `POST /v1/admin/workspace/onboard` — confirm- and digest-bound bulk
    /// import/register of selected workspace directories.
    AdminWorkspaceOnboard,
    /// `GET /v1/admin/portfolio` — cross-project portfolio fleet: each
    /// registered project's user-owned record, tags and read-only evidence
    /// states (`forge-web-portfolio-controls/0.1.0`). Session-gated.
    AdminPortfolioList,
    /// `GET /v1/admin/portfolio/evidence` — truthful cross-project evidence
    /// bundle (catalog, gaps, fleet/inventory, governance, analytics,
    /// provider, readiness, interest). Non-live; no probe on page load.
    AdminPortfolioEvidence,
    /// `GET /v1/admin/portfolio/{id}` — full single-project portfolio view.
    AdminPortfolioProject {
        id: String,
    },
    /// `GET /v1/admin/portfolio/{id}/{kind}` — a read-only portfolio
    /// sub-resource (today `evidence`).
    AdminPortfolioRead {
        id: String,
        kind: String,
    },
    /// `POST /v1/admin/portfolio/{id}/{action}` — a Forge-owned metadata
    /// mutation (`tags`, `relations`, `reviews`, `goals`); `evidence` is the
    /// honest source-owned refusal. `action` is a validated key, never a path.
    AdminPortfolioWrite {
        id: String,
        action: String,
    },
    /// `POST /v1/admin/portfolio/{id}/tags/remove` — confirm-gated,
    /// idempotent detachment of one Forge-owned tag.
    AdminPortfolioTagRemove {
        id: String,
    },
    /// `POST /v1/admin/portfolio/{id}/relations/remove` — confirm-gated,
    /// idempotent withdrawal of one declared relation.
    AdminPortfolioRelationRemove {
        id: String,
    },
    /// `POST /v1/admin/portfolio/{id}/evidence/import` — confirm-gated,
    /// append-only import of one source-owned observation. Never edits.
    AdminPortfolioEvidenceImport {
        id: String,
    },
    /// `GET /v1/admin/delivery` — the delivery overview: share allowlist,
    /// manifest preview digest, approval/publication trail, unreconciled
    /// attempts and non-live provider state
    /// (`forge-web-delivery-controls/0.1.0`). Session-gated, no probe,
    /// no write.
    AdminDelivery,
    /// `GET /v1/admin/delivery/preview` — the side-effect-free manifest
    /// plan every delivery mutation binds its digest to.
    AdminDeliveryPreview,
    /// `GET /v1/admin/delivery/operation/{key}` — journal-backed status of
    /// one publication by its validated operation key. The key is a
    /// validated token, never a path.
    AdminDeliveryOperation {
        key: String,
    },
    /// `POST /v1/admin/delivery/allowlist/{id}` — confirm- and
    /// digest-bound allowlist set/replace of one project's share record.
    AdminDeliveryAllowlist {
        id: String,
    },
    /// `POST /v1/admin/delivery/allowlist/{id}/remove` — confirm- and
    /// digest-bound withdrawal of one share record.
    AdminDeliveryAllowlistRemove {
        id: String,
    },
    /// `POST /v1/admin/delivery/approve` — approval of the exact reviewed
    /// manifest digest; a stale or mismatched digest creates no approval.
    AdminDeliveryApprove,
    /// `POST /v1/admin/delivery/publish` — publication of the approved
    /// manifest through the default-safe local export, bound to the
    /// approved digest plus an idempotent operation key. The artifact
    /// target is server-configured; the browser never names a path.
    AdminDeliveryPublish,
    /// `POST /v1/admin/delivery/reconcile` — operator-recorded resolution
    /// of an `unknown` publication attempt, bound to its exact digest.
    AdminDeliveryReconcile,
    AdminOptions,
    /// `GET /v1/studio/{id}/spec` — read the Studio session.
    StudioSpec {
        id: String,
    },
    /// `POST /v1/studio/{id}/spec` — save a validated AppSpec.
    StudioSpecSave {
        id: String,
    },
    /// `GET /v1/studio/{id}/preview` — read the preview state.
    StudioPreviewGet {
        id: String,
    },
    /// `POST /v1/studio/{id}/preview` — start/stop the bounded preview.
    StudioPreviewPost {
        id: String,
    },
    /// `POST /v1/studio/{id}/refine` — record a refinement.
    StudioRefine {
        id: String,
    },
    /// `GET /v1/projects/{id}/portfolio` — read-only portfolio
    /// projection: user-owned metadata plus the newest
    /// source-owned snapshot per source system.
    PortfolioProject {
        id: String,
    },
    /// `POST /v1/projects/{id}/portfolio/tags` — attach a tag.
    PortfolioTag {
        id: String,
    },
    /// `POST /v1/projects/{id}/portfolio/relations` — link two projects.
    PortfolioRelation {
        id: String,
    },
    /// `POST /v1/projects/{id}/portfolio/reviews` — record a review.
    PortfolioReview {
        id: String,
    },
    /// `POST /v1/projects/{id}/portfolio/evidence` — append a snapshot.
    PortfolioEvidence {
        id: String,
    },
    /// `GET /v1/projects/{id}/share` — read the public share record.
    GetShare {
        id: String,
    },
    /// `POST /v1/projects/{id}/share` — create or replace the public
    /// share record and its allowlisted surfaces.
    SetShare {
        id: String,
    },
    /// `POST /v1/projects/{id}/share/remove` — withdraw the record.
    RemoveShare {
        id: String,
    },
    /// `GET /v1/share/manifest` — preview the candidate manifest.
    SharePreview,
    /// `POST /v1/share/approve` — approve one exact manifest hash.
    ShareApprove,
    /// `POST /v1/share/publish` — publish the approved manifest.
    SharePublish,
    /// `POST /v1/share/reconcile` — resolve a partial publication.
    ShareReconcile,
    /// `GET /v1/share/audit` — approval and publication trail.
    ShareAudit,
    /// `GET /v1/projects/{id}/interest` — the project's aggregate
    /// interest evidence with its freshness labels.
    GetInterest {
        id: String,
    },
    /// `POST /v1/projects/{id}/interest` — import aggregate snapshots
    /// for one project.
    ImportInterest {
        id: String,
    },
    /// `GET /v1/interest/compare` — compare projects on allowlisted
    /// metrics without totalling across windows.
    InterestCompare,
    /// `GET /v1/interest/trend` — one metric's windowed history.
    InterestTrend,
    /// `GET /v1/interest/audit` — the refusals this store recorded.
    InterestAudit,
    /// `GET /v1/interest/readiness` — read-only verdict on whether
    /// aggregate evidence justifies the product-owned activation
    /// follow-up. Admin-gated; always answers `200`.
    InterestReadiness,
    /// `GET /v1/projects/catalog` — read-only catalog query over the
    /// shared Core service. `GET /v1/projects/{id}/catalog` is the
    /// same route with a project id, returning every catalog record
    /// for that id across the selected sources. The transports
    /// stay thin: filtering, ordering and pagination live in Core.
    CatalogQuery {
        id: Option<String>,
    },
    /// `GET /v1/projects/{id}/delivery` — read-only delivery status
    /// projection (`forge-delivery-status/0.1.0`).
    DeliveryStatus {
        id: String,
    },
    /// `POST /v1/projects/{id}/delivery/preflight` — invoke the publish
    /// provider's `preflight` operation and record the terminal
    /// evidence.
    DeliveryPreflight {
        id: String,
    },
    /// `POST /v1/projects/{id}/delivery/stage` — invoke the publish
    /// provider's `publish` operation for the stage environment.
    /// Requires the `confirm_operation_id` body field.
    DeliveryStage {
        id: String,
    },
    /// `POST /v1/projects/{id}/delivery/promote` — invoke the publish
    /// provider's `publish` operation for the production environment.
    /// Requires the `confirm_revision` body field.
    DeliveryPromote {
        id: String,
    },
    /// `POST /v1/projects/{id}/delivery/hermora/retry` — invoke the
    /// Hermora adapter for a healthy deployment. Requires
    /// `deployment_url` and `secret_ref` body fields. Never republishes.
    DeliveryHermoraRetry {
        id: String,
    },
}

/// Outcome of dispatching one request. A successful
/// `Ok` carries the response; an `Err` carries the typed
/// Core failure to render.
pub type DispatchResult = Result<ApiResponse, ForgeError>;

impl std::fmt::Debug for ApiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiConfig")
            .field("bind", &bind_label(self.bind))
            .field("port", &self.port)
            .field("max_body_bytes", &self.max_body_bytes)
            .field("frontend_origin", &self.frontend_origin)
            .finish_non_exhaustive()
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind: DEFAULT_BIND_ADDR,
            port: DEFAULT_PORT,
            max_body_bytes: MAX_BODY_BYTES,
            frontend_origin: "http://127.0.0.1:4173".to_string(),
            previews: Arc::new(Mutex::new(HashMap::new())),
            browser_auth_verifier: Arc::new(crate::identity::LibraryBrowserAuthVerifier),
        }
    }
}

impl From<io::Error> for ApiError {
    fn from(err: io::Error) -> Self {
        ApiError::Io(err.to_string())
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CatalogQueryParams {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            tags: Vec::new(),
            languages: Vec::new(),
            profiles: Vec::new(),
            lifecycles: Vec::new(),
            repositories: Vec::new(),
            ci: Vec::new(),
            compose: Vec::new(),
            evidence: Vec::new(),
            filters: Vec::new(),
            limit: catalog::DEFAULT_LIMIT,
            cursor: None,
            max_age: catalog::DEFAULT_MAX_AGE_SECONDS,
            workspace_registry: None,
            inventory: None,
            git_repositories: Vec::new(),
            github_repositories: Vec::new(),
        }
    }
}
