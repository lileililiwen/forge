//! Optional HTTP transport over Core (`core-http-api`).
//!
//! Forge Core owns every domain rule; this module is a thin
//! HTTP/1.1 transport that exposes the stable project and
//! lifecycle operations listed in [requirement.md §35]. The
//! API server defaults to a loopback-only bind so an
//! unauthenticated public listener is impossible by
//! construction; production exposure is the responsibility
//! of the operator and is out of scope for v0.1.
//!
//! ## Why
//!
//! The brief calls for an optional HTTP API that lets
//! callers reach the same Core contracts the CLI and MCP
//! surfaces consume. The same typed outcomes must flow
//! through every transport: the API validates input, then
//! dispatches through the existing `forge::core` modules
//! (registry, doctor, feature, upgrade, spec, agent,
//! deploy) and renders the response. No business rule is
//! duplicated in the transport.
//!
//! ## Authorization and idempotency
//!
//! Every request (other than `GET /healthz`) requires an
//! `Authorization: Bearer <session-id>` header. The token
//! is an OIDC admin session minted by the
//! `central-admin-identity` surface; it lives at
//! `.forge/identity/<project>/sessions/<id>.json` and is
//! strictly project-scoped. A token minted for project A
//! cannot authorize a request against project B; the
//! request is refused with
//! [`ForgeError::ApiProjectMismatch`] (R2 failure
//! scenario). Mutating routes additionally require the
//! session to carry the `admin:access` permission; a
//! session without that permission is refused with
//! [`ForgeError::ApiUnauthorized`].
//!
//! Mutating routes accept an `Idempotency-Key` header so
//! retries are safe. The `(kind, key)` pair is unique in
//! the registry's operations table; an identical retry
//! reuses the original `op_id` (R2 boundary scenario:
//! external side effects are not repeated). A retry that
//! reuses the key with a different request body is
//! refused with [`ForgeError::IdempotencyKeyConflict`] so
//! the operator never silently reinterprets a prior
//! operation.
//!
//! ## Async operation model
//!
//! Every mutating route returns `202 Accepted` with a
//! `Location: /v1/operations/<id>` header and a JSON
//! envelope containing the operation's recorded state.
//! The operation is journaled in the same `operations`
//! table every other transport writes to; the API layer
//! is therefore a peer of the CLI and MCP journals. A
//! `GET /v1/operations/<id>` request returns the latest
//! state so a caller can poll for progress or terminal
//! outcomes.
//!
//! ## Risks
//!
//! Binding HTTP broadens access beyond a local process.
//! This module therefore defaults to `127.0.0.1` and
//! refuses non-loopback binds unless the operator passes
//! `--bind 0.0.0.0` explicitly. Authentication is required
//! on every route other than the loopback health check;
//! the loopback health check is the only anonymous
//! surface and only returns `200 ok` plus the contract
//! version.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::agent::{
    apply_transition as apply_agent_transition, new_session, read_session, AgentProvider,
    SessionTransition,
};
use crate::catalog;
use crate::core::ForgeError;
use crate::deploy::DeployRequest;
use crate::doctor::{
    parse_target_level, run_doctor, FindingStatus, RegistryObservation, Remediation,
};
use crate::feature::{add_feature, remove_feature, upgrade_feature};
use crate::generate::{generate, normalize_explicit, GeneratedProject};
use crate::policy::{run_driftwatch, DriftWatchConfig, PolicyFinding, PolicySeverity};
use crate::publish::github::{verify_push, GitHubPushEvent};
use crate::publish::providers::{
    invoke_provider, load_config as load_publish_provider_config, select_provider,
    ProviderOperation, PublishProviderRequest, PUBLISH_PROVIDER_CONTRACT,
};
use crate::registry::{Registry, ReservationOutcome};
use crate::spec::{
    apply_routing, ensure_single_project, generate_spec, DoctorFindingInput, FindingSource,
    SpecRequest,
};
use crate::studio::PreviewSession;
use crate::upgrade::{apply_upgrade, plan_upgrade, SemanticConflict, UpgradeOutcome};

mod admin;
/// Typed CLI command-catalog metadata (`forge-command-catalog/0.1.0`)
/// backing `GET /v1/admin/commands`. Metadata only: the catalog never
/// executes anything and the API exposes no shell/eval route. Named
/// `command_catalog` because `catalog` already refers to the project
/// catalog (`forge-project-catalog/0.1.0`) in this module.
pub mod command_catalog;
/// Session-gated, confirm- and digest-bound delivery controls
/// (`forge-web-delivery-controls/0.1.0`) backing `/v1/admin/delivery*`:
/// the share allowlist → preview → approve → publish pipeline through the
/// crate's typed in-process Core functions. Every mutation requires an
/// explicit confirmation bound to the reviewed manifest digest; the
/// browser never supplies a path, and the subprocess publication adapter
/// stays CLI-only.
mod delivery;
mod fleet;
/// Session-gated portfolio controls and cross-project evidence views
/// (`forge-web-portfolio-controls/0.1.0`) backing `/v1/admin/portfolio*`.
/// Forge-owned metadata writes reuse registry Core only; imported,
/// source-owned evidence is read-only and never executed live on page load.
mod portfolio;
/// Session-gated project management (`forge-web-project-management`):
/// browser `new`/`import`/`register` plus the shared
/// `FORGE_ADMIN_PROJECTS_ROOT` confinement both this module and
/// [`workspace`](self::workspace) build on.
mod project_management;
/// Typed, session-gated read-only status projection (`forge-project-status/
/// 0.1.0`) backing `GET /v1/admin/projects/{id}/status` and
/// `GET /v1/admin/status`. Reuses the in-process doctor, checker and profile
/// readiness projections only — never a shell, a write or an external
/// adapter — and never serializes an absolute filesystem path.
mod status;
/// Sub-module that serves the in-process portal UI on the
/// same loopback listener (`GET /ui`, `GET /ui/projects/{id}`,
/// `POST /ui/projects/{id}/publish`). Rendered with
/// [`maud`](https://docs.rs/maud).
pub mod ui;
/// Typed, session-gated single-project workbench (`forge-project-workbench/
/// 0.1.0`) backing `GET /v1/admin/projects/{id}`, its `/plan` and `/apply`
/// subroutes. Reuses typed in-process Core functions only — never a shell —
/// and never serializes an absolute filesystem path.
mod workbench;
/// Live workspace onboarding (`forge-web-workspace-onboarding`): read-only
/// candidate discovery over the configured project root plus bulk
/// preview/confirm/digest-bound onboarding.
mod workspace;

/// Contract data version for the API surface. The version
/// is the source of truth for `/healthz` and the response
/// envelope; an older client can refuse the version
/// mismatch instead of silently reinterpreting the
/// response.
pub const API_CONTRACT_VERSION: &str = "0.1.0";

/// Default bind address. Loopback-only so an
/// unauthenticated public listener is impossible by
/// construction.
pub const DEFAULT_BIND_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// Default port the API server binds to when the
/// operator does not pass `--port`. The value is the
/// unprivileged 8765 range to avoid colliding with
/// system services.
pub const DEFAULT_PORT: u16 = 8765;

/// Maximum request body size. Anything larger is refused
/// with `413 Payload Too Large` so a malicious caller
/// cannot pin the server to an unbounded memory
/// allocation. 1 MiB is more than enough for the routes
/// in scope (deploys and feature installs carry typed
/// JSON, not artifacts).
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Maximum time a single connection may be held idle
/// while reading. Bounded so a slow-loris client cannot
/// pin a worker thread.
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum time a single request handler may run
/// end-to-end. Bounded so a misbehaving adapter (e.g. an
/// external deploy binary that hangs) cannot keep the
/// listener tied up indefinitely.
pub const HANDLER_TIMEOUT: Duration = Duration::from_secs(60);

/// Synthetic project id recorded in the operations table
/// when the API handles a request that is not bound to
/// one specific registered project (for example
/// `POST /v1/projects` for fleet creation). The id is
/// never visible to operators as a registered project.
pub const API_SYNTHETIC_PROJECT: &str = "__api__";

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

fn bind_label(bind: IpAddr) -> String {
    bind.to_string()
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
        let body = serde_json::json!({
            "error": {
                "code": err.code(),
                "message": err.to_string(),
            },
            "contract": API_CONTRACT_VERSION,
        });
        Self::json(status, body)
    }
}

fn err_status(err: &ForgeError) -> u16 {
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
        | "studio-invalid-spec" => 400,
        // Delivery provider / adapter availability is a transient
        // 503: the registry keeps the existing rows intact and the
        // operator can retry without re-running the stage.
        "delivery-unavailable" => 503,
        "studio-port-unavailable" | "studio-start-timeout" => 503,
        _ => 500,
    }
}

/// Outcome of dispatching one request. A successful
/// `Ok` carries the response; an `Err` carries the typed
/// Core failure to render.
pub type DispatchResult = Result<ApiResponse, ForgeError>;

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
    /// `GET /ui` — in-process portal UI fleet list.
    UiFleet,
    /// `GET /ui/sign-in?project=<id>&return=<path>` —
    /// anonymous sign-in start; builds a challenge and
    /// redirects to the configured provider.
    UiSignIn,
    /// `GET /ui/auth/callback?code=…&state=…` — anonymous
    /// OIDC callback; consumes the challenge and mints a
    /// browser session.
    UiAuthCallback,
    /// `POST /ui/sign-out` — same-origin sign-out that
    /// revokes only the calling project's session and
    /// clears the browser cookie.
    UiSignOut,
    /// `GET /ui/projects/{id}` — project detail.
    UiProjectDetail {
        id: String,
    },
    /// `POST /ui/projects/{id}/publish` — confirm-gated republish.
    UiProjectPublish {
        id: String,
    },
    /// `POST /ui/projects/{id}/portfolio` — user-owned metadata write.
    UiProjectPortfolio {
        id: String,
    },
    /// `GET /ui/studio/{id}` — read-only Studio page.
    UiStudioProject {
        id: String,
    },
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
        // Project status is a five-segment read. Its literal `status`
        // segment never collides with the `plan`/`apply`/`feature`/`spec`/
        // `deploy` arms above, so no existing route is shadowed.
        ("GET", ["v1", "admin", "projects", id, "status"]) => Some(Route::AdminProjectStatus {
            id: (*id).to_string(),
        }),
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
        ("GET", ["ui"]) => Some(Route::UiFleet),
        ("GET", ["ui", "sign-in"]) => Some(Route::UiSignIn),
        ("GET", ["ui", "auth", "callback"]) => Some(Route::UiAuthCallback),
        ("POST", ["ui", "sign-out"]) => Some(Route::UiSignOut),
        ("GET", ["ui", "projects", id]) => Some(Route::UiProjectDetail {
            id: (*id).to_string(),
        }),
        ("POST", ["ui", "projects", id, "publish"]) => Some(Route::UiProjectPublish {
            id: (*id).to_string(),
        }),
        ("POST", ["ui", "projects", id, "portfolio"]) => Some(Route::UiProjectPortfolio {
            id: (*id).to_string(),
        }),
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
        ("GET", ["ui", "projects", id, "studio"]) | ("GET", ["ui", "studio", id]) => {
            Some(Route::UiStudioProject {
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

fn bad_request(reason: &str) -> ApiResponse {
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
fn required_permission(route: &Route) -> Option<&'static str> {
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
        | Route::AdminOptions => None,
        Route::ListProjects
        | Route::InspectProject { .. }
        | Route::Doctor { .. }
        | Route::Governance { .. }
        | Route::UiFleet
        | Route::UiSignIn
        | Route::UiAuthCallback
        | Route::UiSignOut
        | Route::UiProjectDetail { .. }
        | Route::UiStudioProject { .. } => None,
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
        | Route::UiProjectPublish { .. }
        // Every share route, preview included, demands admin:access:
        // previewing the candidate manifest reveals which projects an
        // operator considers publishable, which is itself private.
        | Route::UiProjectPortfolio { .. }
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
fn is_mutating(route: &Route) -> bool {
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
        return admin::handle_preflight(config, request);
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
    ) {
        return admin::handle(config, db_path, request, &route);
    }

    // 1. Authorization: every route (other than /healthz
    // and /v1/operations/{id}) demands a session. Read
    // routes demand any valid session; mutating routes
    // demand admin:access. A token for project A cannot
    // authorize project B.
    //
    // The in-process portal UI routes (`Route::UiFleet`,
    // `Route::UiProjectDetail`, `Route::UiProjectPublish`,
    // `Route::UiProjectPortfolio`, the new
    // `Route::UiSignIn`, `Route::UiAuthCallback`,
    // `Route::UiSignOut`) carry their own auth flow: the
    // existing `authorize()` helper looks for the bearer in
    // `request.bearer_token` (the JSON transport) but the
    // UI accepts it through `?token=<id>` as well, so we
    // short-circuit before `authorize()` and let the UI
    // handlers do the bearer/origin checks themselves.
    // `Route::UiSignIn` and `Route::UiAuthCallback` are
    // anonymous by design (the challenge is the proof-in-
    // progress); `Route::UiSignOut` runs its own session
    // lookup so it can revoke the owning project's session
    // regardless of the route's permission posture.
    let actor = if !matches!(
        route,
        Route::UiFleet
            | Route::UiSignIn
            | Route::UiAuthCallback
            | Route::UiSignOut
            | Route::UiProjectDetail { .. }
            | Route::UiProjectPublish { .. }
            | Route::UiProjectPortfolio { .. }
    ) {
        match authorize(db_path, &route, request, now) {
            Ok(actor) => actor,
            Err(response) => return response,
        }
    } else {
        String::new()
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
        | Route::AdminOptions => admin::handle(config, db_path, request, &route),
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
        Route::UiFleet => ui::routes::handle_fleet(db_path, config, request),
        Route::UiSignIn => ui::routes::handle_sign_in(db_path, config, request),
        Route::UiAuthCallback => ui::routes::handle_auth_callback(db_path, config, request, now),
        Route::UiSignOut => ui::routes::handle_sign_out(db_path, config, request, now),
        Route::UiProjectDetail { id } => {
            ui::routes::handle_project_detail(db_path, config, request, &id)
        }
        Route::UiProjectPublish { id } => {
            ui::routes::handle_project_publish(db_path, config, request, &id)
        }
        Route::UiProjectPortfolio { id } => {
            ui::routes::handle_project_portfolio(db_path, config, request, &id)
        }
        Route::UiStudioProject { id } => {
            ui::routes::handle_studio_project(db_path, config, request, &id)
        }
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
        | Route::AdminDeliveryReconcile => not_found(),
    }
}

fn alt_method(method: &str) -> &'static str {
    if method.eq_ignore_ascii_case("GET") {
        "POST"
    } else {
        "GET"
    }
}

/// Authorization step. Returns `Ok(actor)` when the caller
/// is permitted to issue the request, where `actor` is the
/// authenticated session subject recorded in the audit
/// trail; returns `Err(response)` with the rendered error
/// response otherwise. The session is loaded through the
/// identity surface so a token minted for project A cannot
/// authorize project B.
fn authorize(
    db_path: &Path,
    route: &Route,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> Result<String, ApiResponse> {
    if matches!(route, Route::Healthz | Route::GitHubPush) {
        return Ok(String::new());
    }
    let token = request.bearer_token.as_deref().ok_or_else(|| {
        ApiResponse::json(
            401,
            serde_json::json!({
                "error": {
                    "code": "api-unauthorized",
                    "message": "missing Authorization: Bearer <session-id> header"
                },
                "contract": API_CONTRACT_VERSION,
            }),
        )
    })?;
    if token.is_empty() || !is_hex(token) {
        return Err(ApiResponse::json(
            401,
            serde_json::json!({
                "error": {
                    "code": "api-unauthorized",
                    "message": "bearer token must be a non-empty hex session id"
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ));
    }
    // The match is exhaustive over every route that
    // requires authorization. Healthz is already handled
    // by the early return above.
    let result: Result<String, ApiResponse> = match route {
        Route::Healthz => Ok(String::new()),
        Route::GitHubPush => Ok(String::new()),
        // UI routes do their own auth flow; the dispatch
        // short-circuits before reaching this match, but
        // Rust requires the arms anyway. The new sign-in,
        // callback, and sign-out routes are explicit: the
        // sign-in/callback handlers are anonymous, and the
        // sign-out handler resolves the cookie to the
        // owning project itself.
        Route::UiFleet
        | Route::AdminSessionGet
        | Route::AdminSessionPost
        | Route::AdminSessionDelete
        | Route::AdminProjects
        | Route::AdminCommands
        | Route::AdminFleetStatus
        | Route::AdminProjectDetail { .. }
        | Route::AdminProjectStatus { .. }
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
        | Route::AdminOptions
        | Route::UiSignIn
        | Route::UiAuthCallback
        | Route::UiSignOut
        | Route::UiProjectDetail { .. }
        | Route::UiProjectPublish { .. }
        | Route::UiProjectPortfolio { .. }
        | Route::UiStudioProject { .. } => Ok(String::new()),
        Route::GetOperation { .. } => {
            // Operation lookups are read-only; the session
            // is looked up against the registry's known
            // projects to discover the owner.
            let registry = match Registry::open(db_path) {
                Ok(reg) => reg,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            let projects: Vec<(String, PathBuf)> = registry
                .list()
                .ok()
                .map(|records| {
                    records
                        .into_iter()
                        .map(|r| (r.id, PathBuf::from(r.path)))
                        .collect()
                })
                .unwrap_or_default();
            match crate::identity::lookup_session_across_projects(token, projects) {
                Ok(Some((session, _, _))) => Ok(session.subject),
                Ok(None) => Err(ApiResponse::json(
                    401,
                    serde_json::json!({
                        "error": {
                            "code": "api-unauthorized",
                            "message": "bearer session was not found in any registered project's identity store"
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                )),
                Err(err) => Err(ApiResponse::from_error(&err)),
            }
        }
        Route::ListProjects
        | Route::CreateProject
        | Route::CatalogQuery { .. }
        | Route::SharePreview
        | Route::ShareApprove
        | Route::SharePublish
        | Route::ShareReconcile
        | Route::ShareAudit
        | Route::InterestCompare
        | Route::InterestTrend
        | Route::InterestAudit
        | Route::InterestReadiness
        | Route::DeliveryStatus { .. }
        | Route::DeliveryPreflight { .. }
        | Route::DeliveryStage { .. }
        | Route::DeliveryPromote { .. }
        | Route::DeliveryHermoraRetry { .. } => {
            // Fleet routes: walk the registry to find
            // which project minted the session, then
            // validate the permission for the action.
            let registry = match Registry::open(db_path) {
                Ok(reg) => reg,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            let projects: Vec<(String, PathBuf)> = registry
                .list()
                .ok()
                .map(|records| {
                    records
                        .into_iter()
                        .map(|r| (r.id, PathBuf::from(r.path)))
                        .collect()
                })
                .unwrap_or_default();
            let (session, owner_id, owner_dir) =
                match crate::identity::lookup_session_across_projects(token, projects) {
                    Ok(Some(value)) => value,
                    Ok(None) => {
                        return Err(ApiResponse::json(
                            401,
                            serde_json::json!({
                                "error": {
                                    "code": "api-unauthorized",
                                    "message": "bearer session was not found in any registered project's identity store"
                                },
                                "contract": API_CONTRACT_VERSION,
                            }),
                        ));
                    }
                    Err(err) => return Err(ApiResponse::from_error(&err)),
                };
            if let Err(err) = check_session_state(&session, &owner_id, now) {
                return Err(ApiResponse::from_error(&err));
            }
            if let Some(perm) = required_permission(route) {
                if let Err(err) = check_session_permission(&session, &owner_id, perm) {
                    return Err(ApiResponse::from_error(&err));
                }
                if let Err(err) = crate::identity::validate_session(&session, &owner_id, perm, now)
                {
                    return Err(ApiResponse::from_error(&err));
                }
            }
            // The session is valid for the owning project.
            // The actual call target (e.g. `POST /v1/projects`
            // for fleet-level creation) does not need the
            // per-project identity config: the registry
            // enforces id/path uniqueness at write time.
            let _ = owner_dir;
            Ok(session.subject)
        }
        Route::InspectProject { id }
        | Route::Doctor { id }
        | Route::Governance { id }
        | Route::AddFeature { id }
        | Route::UpgradeProject { id }
        | Route::GenerateSpec { id }
        | Route::AgentTransition { id }
        | Route::PortfolioProject { id }
        | Route::PortfolioTag { id }
        | Route::PortfolioRelation { id }
        | Route::PortfolioReview { id }
        | Route::PortfolioEvidence { id }
        | Route::GetShare { id }
        | Route::SetShare { id }
        | Route::RemoveShare { id }
        | Route::GetInterest { id }
        | Route::ImportInterest { id }
        | Route::ApplyDeployment { id }
        | Route::StudioSpec { id }
        | Route::StudioSpecSave { id }
        | Route::StudioPreviewGet { id }
        | Route::StudioPreviewPost { id }
        | Route::StudioRefine { id } => {
            // Project-scoped route: load the project,
            // locate the session in the project directory
            // (the common case) or in any other
            // registered project's identity store (the
            // cross-project boundary case). When the
            // session's actual owner differs from the
            // target project, refuse with
            // `api-project-mismatch` so a token minted
            // for project A cannot authorize project B.
            let registry = match Registry::open(db_path) {
                Ok(reg) => reg,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            // The project must exist in the registry
            // before any session lookup, otherwise the
            // caller could probe arbitrary project
            // identifiers.
            let record = match registry.inspect(id) {
                Ok(value) => value,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            let project_dir = PathBuf::from(record.path);
            // Direct path: the session lives in the
            // target project's identity store. Cross-
            // project fallback: the session may live
            // anywhere in the registered fleet.
            let resolved = match crate::identity::load_session(&project_dir, id, token) {
                Ok(Some(value)) => Some((value, id.to_string())),
                Ok(None) => {
                    let projects: Vec<(String, PathBuf)> = registry
                        .list()
                        .ok()
                        .map(|records| {
                            records
                                .into_iter()
                                .map(|r| (r.id, PathBuf::from(r.path)))
                                .filter(|(pid, _)| pid != id)
                                .collect()
                        })
                        .unwrap_or_default();
                    match crate::identity::lookup_session_across_projects(token, projects) {
                        Ok(Some((session, owner_id, _dir))) => Some((session, owner_id)),
                        Ok(None) => None,
                        Err(err) => return Err(ApiResponse::from_error(&err)),
                    }
                }
                Err(err) => return Err(ApiResponse::from_error(&err)),
            };
            let (session, owner_id) = match resolved {
                Some(value) => value,
                None => {
                    return Err(ApiResponse::json(
                        401,
                        serde_json::json!({
                            "error": {
                                "code": "api-unauthorized",
                                "message": "bearer session was not found in this project's identity store"
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    ));
                }
            };
            if owner_id != *id {
                return Err(ApiResponse::from_error(&ForgeError::ApiProjectMismatch {
                    reason: format!(
                        "session was minted for project `{owner_id}`; presenting it to project `{id}` is refused"
                    ),
                }));
            }
            if let Some(perm) = required_permission(route) {
                if let Err(err) = crate::identity::validate_session(&session, id, perm, now) {
                    return Err(ApiResponse::from_error(&err));
                }
            } else {
                // Read-only route: still require a
                // non-revoked, non-expired session for
                // this project.
                if let Err(err) = check_session_state(&session, id, now) {
                    return Err(ApiResponse::from_error(&err));
                }
            }
            Ok(session.subject)
        }
    };
    result
}

fn check_session_state(
    session: &crate::identity::AdminSession,
    project_id: &str,
    now: DateTime<Utc>,
) -> Result<(), ForgeError> {
    use crate::identity::SessionState;
    if session.project_id != project_id {
        return Err(ForgeError::ApiProjectMismatch {
            reason: format!(
                "session was minted for project `{}`; presenting it to project `{project_id}` is refused",
                session.project_id
            ),
        });
    }
    if session.state == SessionState::Revoked {
        return Err(ForgeError::ApiUnauthorized {
            reason: format!(
                "session `{}` is revoked; a new challenge must be built",
                session.session_id
            ),
        });
    }
    if session.is_expired(now) {
        return Err(ForgeError::ApiUnauthorized {
            reason: format!(
                "session `{}` expired at {}",
                session.session_id,
                session.expires_at.to_rfc3339()
            ),
        });
    }
    Ok(())
}

fn check_session_permission(
    session: &crate::identity::AdminSession,
    project_id: &str,
    perm: &str,
) -> Result<(), ForgeError> {
    if !session.permissions.iter().any(|p| p == perm) {
        return Err(ForgeError::ApiUnauthorized {
            reason: format!(
                "session `{}` for project `{project_id}` does not carry the `{perm}` permission; minted permissions: {:?}",
                session.session_id, session.permissions
            ),
        });
    }
    Ok(())
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}

// ---- handlers ------------------------------------------------------

fn handle_github_push(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let signature = match request.header("x-hub-signature-256") {
        Some(value) => value.to_string(),
        None => return bad_request("GitHub push requires X-Hub-Signature-256"),
    };
    let delivery_id = match request.header("x-github-delivery") {
        Some(value) if !value.trim().is_empty() => value.to_string(),
        _ => return bad_request("GitHub push requires X-GitHub-Delivery"),
    };
    let body: Value = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(_) => return bad_request("GitHub push body must be valid JSON"),
    };
    let repository = body
        .get("repository")
        .and_then(|value| value.get("full_name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let git_ref = body.get("ref").and_then(Value::as_str).unwrap_or_default();
    let after = body
        .get("after")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let secret = match std::env::var("FORGE_GITHUB_WEBHOOK_SECRET") {
        Ok(value) if !value.is_empty() => value,
        _ => return bad_request("FORGE_GITHUB_WEBHOOK_SECRET is not configured"),
    };
    let allowed_repository = std::env::var("FORGE_GITHUB_REPOSITORY").unwrap_or_default();
    let allowed_ref =
        std::env::var("FORGE_GITHUB_REF").unwrap_or_else(|_| "refs/heads/main".to_string());
    let event = GitHubPushEvent {
        delivery_id: delivery_id.clone(),
        repository: repository.to_string(),
        git_ref: git_ref.to_string(),
        after: after.to_string(),
        signature,
        body: request.body.clone(),
    };
    if let Err(error) = verify_push(&event, secret.as_bytes(), &allowed_repository, &allowed_ref) {
        return bad_request(&error.to_string());
    }
    let project_id = match std::env::var("FORGE_GITHUB_PROJECT_ID") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return bad_request("FORGE_GITHUB_PROJECT_ID is not configured"),
    };
    let provider_id = match std::env::var("FORGE_PUBLISH_PROVIDER") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return bad_request("FORGE_PUBLISH_PROVIDER is not configured"),
    };
    let registry = match Registry::open(db_path) {
        Ok(value) => value,
        Err(error) => return ApiResponse::from_error(&error),
    };
    let mut hash = Sha256::new();
    hash.update(&request.body);
    let request_hash = format!("{:x}", hash.finalize());
    let reservation = match registry.reserve_idempotent_operation(
        "publish.github",
        &project_id,
        &delivery_id,
        &request_hash,
    ) {
        Ok(value) => value,
        Err(error) => return ApiResponse::from_error(&error),
    };
    let op_id = match reservation {
        ReservationOutcome::Reused { op_id } => {
            return ApiResponse::json(
                200,
                serde_json::json!({
                    "contract": API_CONTRACT_VERSION,
                    "delivery_id": delivery_id,
                    "operation_id": op_id,
                    "status": "duplicate",
                }),
            )
        }
        ReservationOutcome::Reserved { op_id } => op_id,
    };
    let record = match registry.inspect(&project_id) {
        Ok(value) => value,
        Err(error) => {
            let _ = registry.finalize_operation(op_id, "failed", &error.to_string());
            return ApiResponse::from_error(&error);
        }
    };
    let project_dir = PathBuf::from(record.path);
    let config_path = std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"));
    let config = match load_publish_provider_config(&config_path) {
        Ok(value) => value,
        Err(error) => {
            let _ = registry.finalize_operation(op_id, "failed", &error.to_string());
            return ApiResponse::from_error(&error);
        }
    };
    let provider = match select_provider(&config, &provider_id) {
        Ok(value) => value,
        Err(error) => {
            let _ = registry.finalize_operation(op_id, "failed", &error.to_string());
            return ApiResponse::from_error(&error);
        }
    };
    let provider_request = PublishProviderRequest {
        contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: ProviderOperation::Publish,
        provider: provider_id.clone(),
        project_id: project_id.clone(),
        revision: after.to_string(),
        operation_id: format!("github-{delivery_id}"),
        folder: Some(project_dir.display().to_string()),
        dry_run: false,
        queue_id: None,
    };
    let response = match invoke_provider(&provider, &provider_request, &project_dir) {
        Ok(value) => value,
        Err(error) => {
            let _ = registry.finalize_operation(op_id, "failed", &error.to_string());
            return ApiResponse::from_error(&error);
        }
    };
    let phase_revision = response
        .revision
        .clone()
        .unwrap_or_else(|| provider_request.revision.clone());
    let container_identity = response.container_identity.clone().unwrap_or_else(|| {
        crate::publish::providers::compose_project_name(&project_id, &phase_revision)
    });
    let _ = registry.update_operation_phase(
        op_id,
        Some(&phase_revision),
        response.build_status.as_deref(),
        response.run_status.as_deref(),
        Some(&container_identity),
    );
    let detail = format!(
        "provider={} revision={} health={} build={:?} run={:?}",
        response.provider,
        phase_revision,
        response.health,
        response.build_status,
        response.run_status
    );
    let _ = registry.finalize_operation(op_id, &response.status, &detail);
    ApiResponse::json(
        202,
        serde_json::json!({
            "contract": API_CONTRACT_VERSION,
            "delivery_id": delivery_id,
            "operation_id": op_id,
            "provider": response.provider,
            "revision": phase_revision,
            "build_status": response.build_status,
            "run_status": response.run_status,
            "container_identity": container_identity,
            "status": response.status,
            "health": response.health,
            "evidence": response.evidence,
            "recovery": response.recovery,
        }),
    )
}

fn handle_list_projects(db_path: &Path) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.list() {
        Ok(projects) => ApiResponse::json(
            200,
            serde_json::json!({
                "projects": projects,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_inspect_project(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.inspect(id) {
        Ok(record) => ApiResponse::json(
            200,
            serde_json::json!({
                "project": record,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/projects/catalog` and `GET /v1/projects/{id}/catalog`.
///
/// Read-only projection over the shared Core catalog service
/// (`src/catalog/`). The handler parses query-string parameters,
/// builds a `CatalogQuery`, and delegates to `catalog::collect` +
/// `catalog::apply` (list) or `catalog::inspect_records` (inspect).
/// No filtering, ordering or pagination rule lives in the
/// transport: the same inputs produce the same `CatalogPage` bytes
/// the CLI and MCP surfaces serialize.
fn handle_catalog_query(
    db_path: &Path,
    request: &ApiRequest,
    id: Option<&str>,
    now: DateTime<Utc>,
) -> ApiResponse {
    let params = match parse_catalog_query_params(request.query.as_deref()) {
        Ok(params) => params,
        Err(response) => return response,
    };
    let selection = match build_catalog_selection(&params) {
        Ok(selection) => selection,
        Err(response) => return response,
    };
    if let Err(err) = catalog::validate_max_age(params.max_age) {
        return ApiResponse::from_error(&err);
    }
    let pairs = catalog_filter_pairs_from(&params);
    let query = match catalog::CatalogQuery::from_pairs(&pairs, params.limit, params.cursor.clone())
    {
        Ok(query) => query.normalize(),
        Err(err) => return ApiResponse::from_error(&err),
    };
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: params.max_age,
        now,
    });
    match id {
        Some(project_id) => match catalog::inspect_records(&bundle, project_id) {
            Ok(records) => ApiResponse::json(
                200,
                serde_json::json!({
                    "catalog": {
                        "contract": catalog::CATALOG_CONTRACT_VERSION,
                        "project_id": project_id,
                        "records": records,
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            ),
            Err(err) => ApiResponse::from_error(&err),
        },
        None => {
            let mut page = match catalog::apply(&bundle.records, &query, &bundle.observed_at) {
                Ok(page) => page,
                Err(err) => return ApiResponse::from_error(&err),
            };
            page.sources = bundle.statuses.clone();
            ApiResponse::json(
                200,
                serde_json::json!({
                    "catalog": page,
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
    }
}

// --- delivery ----------------------------------------------------------
//
// The delivery routes share the same handlers the CLI surface
// uses; every refusal is typed (`delivery-invalid`,
// `delivery-conflict`, `delivery-unavailable`) so the error code
// is identical across transports.

fn handle_delivery_status(db_path: &Path, id: &str, now: DateTime<Utc>) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_status(&registry, id, now) {
        Ok(report) => ApiResponse::json(200, serde_json::to_value(&report).unwrap_or(Value::Null)),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_delivery_preflight(db_path: &Path, id: &str, now: DateTime<Utc>) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_preflight(&registry, id, now) {
        Ok(outcome) => ApiResponse::json(
            200,
            serde_json::to_value(&outcome.report).unwrap_or(Value::Null),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_delivery_stage(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let confirm_operation_id = match body.get("confirm_operation_id").and_then(|v| v.as_i64()) {
        Some(value) => value,
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "delivery-invalid",
                        "message": "delivery stage requires a `confirm_operation_id` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_stage(&registry, id, confirm_operation_id, now) {
        Ok(outcome) => ApiResponse::json(
            200,
            serde_json::to_value(&outcome.report).unwrap_or(Value::Null),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_delivery_promote(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let confirm_revision = match body.get("confirm_revision").and_then(|v| v.as_str()) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "delivery-invalid",
                        "message": "delivery promote requires a `confirm_revision` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_promote(&registry, id, &confirm_revision, now) {
        Ok(outcome) => ApiResponse::json(
            200,
            serde_json::to_value(&outcome.report).unwrap_or(Value::Null),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_delivery_hermora_retry(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let deployment_url = match body.get("deployment_url").and_then(|v| v.as_str()) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "delivery-invalid",
                        "message": "delivery hermora-retry requires a `deployment_url` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let secret_ref = match body.get("secret_ref").and_then(|v| v.as_str()) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "delivery-invalid",
                        "message": "delivery hermora-retry requires a `secret_ref` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::delivery::handlers::run_hermora_retry(
        &registry,
        id,
        &deployment_url,
        &secret_ref,
        now,
    ) {
        Ok(outcome) => ApiResponse::json(
            200,
            serde_json::to_value(&outcome.report).unwrap_or(Value::Null),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

// --- studio handlers ---------------------------------------------------

fn handle_studio_spec_get(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match crate::studio::load_session(&project_root) {
        Ok(Some(session)) => {
            let envelope = crate::studio::state::session_envelope(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "studio": envelope,
                    "contract": crate::studio::STUDIO_SESSION_CONTRACT,
                }),
            )
        }
        Ok(None) => ApiResponse::json(
            404,
            serde_json::json!({
                "error": {
                    "code": "studio-invalid-spec",
                    "message": format!("no Studio session for project '{id}'; save a spec first"),
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_studio_spec_save(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let spec_text = match body.get("spec").and_then(|v| v.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "studio-invalid-spec",
                        "message": "studio spec save requires a `spec` body field (YAML text)",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let expected_revision = match body.get("expected_revision").and_then(|v| v.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "studio-revision-conflict",
                        "message": "studio spec save requires `expected_revision` (use `r0` for the first save)",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let confirm = body.get("confirm").and_then(|v| v.as_str());
    if confirm != Some("yes") {
        return ApiResponse::json(
            400,
            serde_json::json!({
                "error": {
                    "code": "studio-invalid-spec",
                    "message": "studio spec save requires `confirm: yes`",
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let spec = match crate::studio::parse_spec_text(&spec_text) {
        Ok(spec) => spec,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match crate::studio::save_spec(&registry, id, &project_root, spec, &expected_revision) {
        Ok(session) => {
            let envelope = crate::studio::state::session_envelope(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "studio": envelope,
                    "contract": crate::studio::STUDIO_SESSION_CONTRACT,
                }),
            )
        }
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_studio_preview_get(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match crate::studio::load_session(&project_root) {
        Ok(Some(session)) => {
            let envelope = crate::studio::envelope_from_session(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                    "contract": crate::studio::PREVIEW_CONTRACT,
                }),
            )
        }
        Ok(None) => {
            let envelope = crate::studio::PreviewEnvelope::from_session(
                id,
                "r0",
                &crate::studio::state::SessionPreviewState::default(),
            );
            ApiResponse::json(
                200,
                serde_json::json!({
                    "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                    "contract": crate::studio::PREVIEW_CONTRACT,
                }),
            )
        }
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_studio_preview_post(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let action = body.get("action").and_then(|v| v.as_str()).unwrap_or("");
    let confirm = body.get("confirm").and_then(|v| v.as_str());
    if confirm != Some("yes") {
        return ApiResponse::from_error(&ForgeError::StudioInvalidSpec {
            reason: "studio preview requires `confirm: yes`".to_string(),
        });
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match action {
        "start" => {
            // The API server is long-lived, so it owns the live
            // session. Start the replacement first; only after it
            // reports ready do we drop any previous session (whose
            // drop kills only its own child).
            let runner = Box::new(crate::studio::ProcessRunner::react_web());
            let (session, live) =
                match crate::studio::start_preview(&registry, &project_root, runner) {
                    Ok(value) => value,
                    Err(err) => return ApiResponse::from_error(&err),
                };
            let previous = config.previews.lock().unwrap().insert(id.to_string(), live);
            drop(previous);
            let envelope = crate::studio::envelope_from_session(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                    "contract": crate::studio::PREVIEW_CONTRACT,
                }),
            )
        }
        "stop" => {
            // Remove the live session first so a concurrent stop cannot
            // double-kill it, then let the Core record the idempotent
            // journal row and persist `stopped` (with or without a
            // held child).
            let live = config.previews.lock().unwrap().remove(id);
            match crate::studio::stop_preview(&registry, &project_root, live) {
                Ok(session) => {
                    let envelope = crate::studio::envelope_from_session(&session);
                    ApiResponse::json(
                        200,
                        serde_json::json!({
                            "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                            "contract": crate::studio::PREVIEW_CONTRACT,
                        }),
                    )
                }
                Err(err) => ApiResponse::from_error(&err),
            }
        }
        _ => ApiResponse::from_error(&ForgeError::StudioInvalidSpec {
            reason: "studio preview action must be `start` or `stop`".to_string(),
        }),
    }
}

fn handle_studio_refine(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let expected_revision = match body.get("expected_revision").and_then(|v| v.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "studio-revision-conflict",
                        "message": "studio refine requires `expected_revision` (use the current spec_revision or app_revision)",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let request_text = match body.get("request").and_then(|v| v.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "studio-invalid-spec",
                        "message": "studio refine requires a `request` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let selected_files: Vec<String> = body
        .get("selected_files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match crate::studio::record_refinement(
        &registry,
        &project_root,
        &expected_revision,
        &request_text,
        &selected_files,
    ) {
        Ok(session) => {
            let envelope = crate::studio::envelope_from_session(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                    "contract": crate::studio::PREVIEW_CONTRACT,
                }),
            )
        }
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_doctor(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_dir = PathBuf::from(&record.path);
    let level = match request.json_body().get("target").and_then(|v| v.as_str()) {
        Some(raw) => match parse_target_level(raw) {
            Ok(value) => Some(value),
            Err(err) => return ApiResponse::from_error(&err),
        },
        None => None,
    };
    let observation = Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at.clone()),
    });
    let policy_outcome = run_driftwatch(&project_dir, &DriftWatchConfig::from_env());
    match run_doctor(
        &project_dir,
        level,
        observation.as_ref(),
        Some(&policy_outcome),
    ) {
        Ok(report) => ApiResponse::json(
            200,
            serde_json::json!({
                "doctor": report,
                "policy": serde_json::to_value(&policy_outcome).unwrap_or(Value::Null),
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_governance(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::governance::evaluate_project(Path::new(&record.path)) {
        Ok(observation) => ApiResponse::json(
            200,
            serde_json::json!({
                "governance": observation,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn handle_create_project(db_path: &Path, request: &ApiRequest, _now: DateTime<Utc>) -> ApiResponse {
    let body = request.json_body();
    let path = match body.get("path").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("create_project requires a `path` field"),
    };
    let profile = match body.get("profile").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("create_project requires a `profile` field"),
    };
    let id = match body.get("id").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("create_project requires an `id` field"),
    };
    let name = body.get("name").and_then(|v| v.as_str());
    let features: Vec<String> = body
        .get("features")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if let Err(reason) = crate::core::validate_project_id(id) {
        return bad_request(&format!("invalid project id: {reason}"));
    }
    let destination = PathBuf::from(path);
    let normalized =
        match normalize_explicit(Some(profile), Some(id), name, &features, &destination, None) {
            Ok(value) => value,
            Err(err) => return ApiResponse::from_error(&err),
        };
    let (created, _op_id, project_id) = match run_with_operation(
        db_path,
        "api.create_project",
        API_SYNTHETIC_PROJECT,
        request,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let generated: GeneratedProject = generate(&mut registry, &normalized)?;
            let value = serde_json::to_value(&generated).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            let project_id = generated.record.id.clone();
            let detail = format!(
                "api create_project `{}` ({}) from {}@{}",
                generated.record.id,
                generated.record.path,
                profile,
                crate::generate::GENERATOR_VERSION
            );
            let _ = registry.record_operation("api", &project_id, "done", &detail);
            Ok((value, op_id, project_id))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "created": created,
            "contract": API_CONTRACT_VERSION,
            "project_id": project_id,
        }),
    )
}

fn handle_add_feature(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let feature = match body.get("feature").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("add_feature requires a `feature` field"),
    };
    let version = body.get("version").and_then(|v| v.as_str());
    let (outcome, op_id, _pid) = match run_with_operation(
        db_path,
        "api.add_feature",
        id,
        request,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let outcome = add_feature(&mut registry, id, feature, version)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "feature": outcome,
            "operation_id": op_id,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

fn handle_upgrade(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let feature = body.get("feature").and_then(|v| v.as_str());
    let confirm = body
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !confirm {
        return ApiResponse::json(
            409,
            serde_json::json!({
                "error": {
                    "code": "api-confirm-required",
                    "message": "upgrade requires `confirm: true`; refusing implicit project mutation"
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let dry_run = body
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let (value, _op_id, _pid) =
        match run_with_operation(db_path, "api.upgrade", id, request, |op_id, _registry| {
            if dry_run {
                let registry = Registry::open(db_path)?;
                let plan = plan_upgrade(&registry, id, feature)?;
                let value = serde_json::to_value(&plan).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?;
                return Ok((value, op_id, id.to_string()));
            }
            let mut registry = Registry::open(db_path)?;
            let outcome: UpgradeOutcome = apply_upgrade(&mut registry, id, feature)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        }) {
            Ok(value) => value,
            Err(response) => return response,
        };
    ApiResponse::json(
        202,
        serde_json::json!({
            "upgrade": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

fn handle_generate_spec(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let findings: Vec<String> = body
        .get("findings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if findings.is_empty() {
        return bad_request("generate_spec requires at least one finding id");
    }
    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .map(String::from);
    let project_dir = match Registry::open(db_path)
        .ok()
        .and_then(|registry| registry.inspect(id).ok())
        .map(|record| PathBuf::from(record.path))
    {
        Some(value) => value,
        None => {
            return ApiResponse::from_error(&ForgeError::UnknownProject {
                query: id.to_string(),
            });
        }
    };
    let spec_request = SpecRequest {
        project_path: project_dir,
        finding_ids: findings,
        reason,
    };
    if let Err(err) = ensure_single_project(&spec_request) {
        return ApiResponse::from_error(&err);
    }
    let sources: Vec<FindingSource> = Vec::new();
    let now = Utc::now();
    let (value, _op_id, _pid) = match run_with_operation(
        db_path,
        "api.generate_spec",
        id,
        request,
        |op_id, _registry| {
            let outcome = generate_spec(&spec_request, &sources, now)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "spec": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

/// Synthesize the in-process [`FindingSource`] for `forge spec apply` exactly
/// as the CLI's `finding_source_for` does: it is derived solely from the
/// finding-name prefix (a policy, semantic-conflict or doctor finding) and the
/// server-resolved project directory — never from a caller-supplied path,
/// argv or shell text. This keeps the portal route inside the same typed,
/// id-scoped boundary the CLI runs.
fn synthesize_finding_source(
    target: &str,
    project_path: &Path,
    finding: &str,
) -> Result<FindingSource, ForgeError> {
    if let Some(stripped) = finding.strip_prefix("driftwatch-") {
        return Ok(FindingSource::Policy(PolicyFinding {
            id: stripped.to_string(),
            category: "spec".to_string(),
            severity: PolicySeverity::Fail,
            applicable: true,
            message: format!("policy finding `{stripped}`"),
            evidence: Vec::new(),
            reason: None,
        }));
    }
    if let Some(stripped) = finding.strip_prefix("semantic-") {
        let (manifest, _) = crate::core::manifest::Manifest::load_from_dir(project_path, None)?;
        return Ok(FindingSource::Conflict(SemanticConflict {
            project_id: manifest.project.id,
            feature: stripped.to_string(),
            owned_file: format!(".forge/features/{stripped}.receipt"),
            reason: "drifted receipt reported by the portal".to_string(),
            suggested_spec: format!("forge spec generate --project {target} --finding {finding}"),
        }));
    }
    Ok(FindingSource::Doctor(DoctorFindingInput {
        id: finding.to_string(),
        status: FindingStatus::Fail,
        remediation: Remediation::Manual,
        category: "spec".to_string(),
        detail: format!("finding `{finding}` routed by `forge spec apply`"),
    }))
}

fn handle_remove_feature(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let feature = match body.get("feature").and_then(|v| v.as_str()) {
        Some(value) if !value.trim().is_empty() => value,
        _ => return bad_request("remove_feature requires a `feature` field"),
    };
    let (outcome, op_id, _pid) = match run_with_operation(
        db_path,
        "api.remove_feature",
        id,
        request,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let outcome = remove_feature(&mut registry, id, feature)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "feature": outcome,
            "operation_id": op_id,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

fn handle_upgrade_feature(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let feature = match body.get("feature").and_then(|v| v.as_str()) {
        Some(value) if !value.trim().is_empty() => value,
        _ => return bad_request("upgrade_feature requires a `feature` field"),
    };
    let version = body.get("version").and_then(|v| v.as_str());
    let (outcome, op_id, _pid) = match run_with_operation(
        db_path,
        "api.upgrade_feature",
        id,
        request,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let outcome = upgrade_feature(&mut registry, id, feature, version)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "feature": outcome,
            "operation_id": op_id,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

fn handle_apply_spec(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let findings: Vec<String> = body
        .get("findings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if findings.is_empty() {
        return bad_request("apply_spec requires at least one finding id");
    }
    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .map(String::from);
    // Resolve the stored project directory from the validated id only; the
    // browser never sends a path. A Core error carrying a path is scrubbed by
    // `ApiResponse::from_error`.
    let project_dir = match Registry::open(db_path)
        .ok()
        .and_then(|registry| registry.inspect(id).ok())
        .map(|record| PathBuf::from(record.path))
    {
        Some(value) => value,
        None => {
            return ApiResponse::from_error(&ForgeError::UnknownProject {
                query: id.to_string(),
            });
        }
    };
    let now = Utc::now();
    let (value, _op_id, _pid) = match run_with_operation(
        db_path,
        "api.apply_spec",
        id,
        request,
        |op_id, _registry| {
            // Mirror the CLI `spec apply`: route each finding individually
            // through `apply_routing`, synthesizing the same in-process
            // finding source the CLI derives from the finding-name prefix.
            let mut outcomes = Vec::new();
            for finding in &findings {
                let spec_request = SpecRequest {
                    project_path: project_dir.clone(),
                    finding_ids: vec![finding.clone()],
                    reason: reason.clone(),
                };
                ensure_single_project(&spec_request)?;
                let source = synthesize_finding_source(id, &project_dir, finding)?;
                let outcome = apply_routing(&spec_request, &source, now)?;
                outcomes.push(serde_json::to_value(&outcome).map_err(|err| {
                    ForgeError::Registry {
                        reason: err.to_string(),
                    }
                })?);
            }
            let value = Value::Array(outcomes);
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "spec": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

fn handle_agent_transition(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let session_id = match body.get("session").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("agents requires a `session` field"),
    };
    let transition = match body.get("transition").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("agents requires a `transition` field"),
    };
    let provider = body
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("opencode");
    let spec = body.get("spec").and_then(|v| v.as_str()).map(String::from);
    let provider_id = match provider {
        "opencode" => AgentProvider::Opencode,
        "codex" => AgentProvider::Codex,
        "ariadex" => AgentProvider::Ariadex,
        other => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "api-invalid",
                        "message": format!("unknown agent provider `{other}`; expected one of: opencode, codex, ariadex")
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let transition_kind = match transition {
        "start" => SessionTransition::Start,
        "pause" => SessionTransition::Pause,
        "takeover" => SessionTransition::Takeover,
        "resume" => SessionTransition::Resume,
        "restart" => SessionTransition::Restart,
        "new_session" => SessionTransition::NewSession,
        other => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "api-invalid",
                        "message": format!("unknown agent transition `{other}`; expected one of: start, pause, takeover, resume, restart, new_session")
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_dir = match registry.inspect(id).map(|r| PathBuf::from(r.path)) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let (value, _op_id, _pid) =
        match run_with_operation(db_path, "api.agent", id, request, |op_id, _registry| {
            let session = match transition_kind {
                SessionTransition::Start => new_session(
                    &project_dir,
                    session_id,
                    provider_id,
                    spec.as_deref().unwrap_or(""),
                    now,
                )?,
                _ => match read_session(&project_dir, session_id)? {
                    Some(value) => value,
                    None => {
                        return Err(ForgeError::AgentUnavailable {
                            reason: format!(
                                "session `{session_id}` was not found under `.forge/agents/`"
                            ),
                        });
                    }
                },
            };
            let outcome = apply_agent_transition(session, transition_kind, now)?;
            let files = crate::agent::write_session(&project_dir, &outcome.session)?;
            let detail = format!(
                "api agent `{id}` session `{session_id}` {transition} -> `{}`",
                outcome.state.label()
            );
            let _ = registry.record_operation("api", id, "done", &detail);
            let value = serde_json::json!({
                "transition": {
                    "session_id": outcome.session.session_id,
                    "state": outcome.state.label(),
                    "requested": outcome.requested.label(),
                    "evidence": outcome.evidence,
                    "next_step": outcome.next_step,
                    "note": outcome.note,
                    "contract": outcome.contract,
                },
                "files_written": files,
            });
            Ok((value, op_id, id.to_string()))
        }) {
            Ok(value) => value,
            Err(response) => return response,
        };
    ApiResponse::json(
        202,
        serde_json::json!({
            "agent": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

fn handle_apply_deployment(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let confirm = body
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !confirm {
        return ApiResponse::json(
            409,
            serde_json::json!({
                "error": {
                    "code": "deploy-confirm-required",
                    "message": "deploy requires `confirm: true`; refusing implicit remote write"
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let dry_run = body
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_dir = match registry.inspect(id).map(|r| PathBuf::from(r.path)) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let (manifest, config) = match crate::deploy::engine::load_config(&project_dir) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let target = body
        .get("target_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| config.default_target.clone());
    let deploy_request = DeployRequest {
        project_id: id.to_string(),
        target,
        confirm: true,
        dry_run,
    };
    let (value, _op_id, _pid) =
        match run_with_operation(db_path, "api.deploy", id, request, |op_id, _registry| {
            let adapter = crate::deploy::DeployAdapterConfig::from_env();
            let report = if dry_run {
                let plan = crate::deploy::engine::prepare_deploy(
                    &project_dir,
                    &manifest,
                    &config,
                    &deploy_request,
                )?;
                serde_json::to_value(&plan).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?
            } else {
                let report = crate::deploy::engine::apply_deploy(
                    &project_dir,
                    &manifest,
                    &config,
                    &deploy_request,
                    &adapter,
                )?;
                serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?
            };
            Ok((report, op_id, id.to_string()))
        }) {
            Ok(value) => value,
            Err(response) => return response,
        };
    ApiResponse::json(
        202,
        serde_json::json!({
            "deploy": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

fn handle_get_operation(db_path: &Path, op_id: i64) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.operation(op_id) {
        Ok(Some(entry)) => {
            ApiResponse::json(200, serde_json::to_value(&entry).unwrap_or(Value::Null))
        }
        Ok(None) => ApiResponse::json(
            404,
            serde_json::json!({
                "error": {
                    "code": "operation-not-found",
                    "message": format!("operation `{op_id}` is not recorded in the registry's journal")
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

// --- portfolio ----------------------------------------------------------
//
// Every portfolio handler dispatches into the same Core contracts
// the CLI uses, so the JSON transport adds no business rule of its
// own. The read projection is read-only; the four mutations share
// the existing authorization boundary (an `admin:access`
// session for the target project) and validate their payload
// before touching the registry. An unauthorized request never
// reaches a write, so it persists no change.

/// `GET /v1/projects/{id}/portfolio`
fn handle_portfolio_project(db_path: &Path, id: &str, now: DateTime<Utc>) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.portfolio_project_view(id, now) {
        Ok(view) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": view,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn required_field<'a>(body: &'a Value, field: &str) -> Result<&'a str, ApiResponse> {
    body.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| bad_request(&format!("portfolio request requires a `{field}` field")))
}

fn optional_field<'a>(body: &'a Value, field: &str) -> Option<&'a str> {
    body.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
}

/// `POST /v1/projects/{id}/portfolio/tags`
fn handle_portfolio_tag(db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    let body = request.json_body();
    let name = match required_field(&body, "name") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let color = optional_field(&body, "color").map(|value| value.to_string());
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.portfolio_add_tag(id, &name, color.as_deref()) {
        Ok(tag) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": { "project_id": id, "tag": tag },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/portfolio/relations`
fn handle_portfolio_relation(db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    let body = request.json_body();
    let to = match required_field(&body, "to") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let raw_type = match required_field(&body, "type") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let relation_type = match crate::portfolio::RelationType::parse(&raw_type) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let note = optional_field(&body, "note").map(|value| value.to_string());
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.portfolio_add_relation(id, &to, relation_type, note.as_deref()) {
        Ok(relation) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": { "project_id": id, "relation": relation },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/portfolio/reviews`
fn handle_portfolio_review(db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    let body = request.json_body();
    let raw_confidence = match required_field(&body, "confidence") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let confidence = match crate::portfolio::Confidence::parse(&raw_confidence) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let lifecycle = match optional_field(&body, "lifecycle") {
        Some(raw) => match crate::portfolio::Lifecycle::parse(raw) {
            Ok(value) => Some(value),
            Err(reason) => return bad_request(&reason),
        },
        None => None,
    };
    let note = optional_field(&body, "note").map(|value| value.to_string());
    let next_action = optional_field(&body, "next_action").map(|value| value.to_string());
    let blocker = optional_field(&body, "blocker").map(|value| value.to_string());
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let write = crate::registry::PortfolioWrite {
        lifecycle,
        confidence: Some(confidence),
        next_action,
        blocker,
    };
    let outcome = registry.portfolio_write(id, &write).and_then(|profile| {
        registry
            .portfolio_record_review(id, confidence, note.as_deref())
            .map(|review| (profile, review))
    });
    match outcome {
        Ok((profile, review)) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": { "project_id": id, "profile": profile, "review": review },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/portfolio/evidence`
fn handle_portfolio_evidence(db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    let body = request.json_body();
    let source = match required_field(&body, "source") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let revision = match required_field(&body, "revision") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let raw_status = match required_field(&body, "status") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let status = match crate::portfolio::EvidenceStatus::parse(&raw_status) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let write = crate::registry::SnapshotWrite {
        source_system: source,
        source_revision: revision,
        observed_at: optional_field(&body, "observed_at")
            .map(|value| value.to_string())
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
        status,
        stale_after: optional_field(&body, "stale_after").map(|value| value.to_string()),
        evidence_json: body
            .get("evidence")
            .cloned()
            .filter(|value| !value.is_null())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "{}".to_string()),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.portfolio_import_snapshot(id, &write) {
        Ok(snapshot) => ApiResponse::json(
            200,
            serde_json::json!({
                "portfolio": { "project_id": id, "snapshot": snapshot },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

// --- portfolio share ---------------------------------------------------
//
// Every handler below dispatches into
// [`crate::portfolio::publication`], the same orchestration the CLI
// uses, so the JSON transport adds no business rule of its own. The
// authorization boundary has already demanded `admin:access` for
// every one of these routes, so an unauthorized request never reaches
// a write and persists no share state. Preview is included on
// purpose: the candidate manifest reveals which projects an operator
// considers publishable, which is itself private.

/// `GET /v1/projects/{id}/share`
fn handle_get_share(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    if let Err(err) = registry.inspect(id) {
        return ApiResponse::from_error(&err);
    }
    match registry.share_record(id) {
        // A project without a share record is private, not empty: the
        // projection says so rather than inventing a blank entry.
        Ok(None) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "project_id": id, "shared": false },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Ok(Some(record)) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "project_id": id, "shared": true, "record": record },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/share`
fn handle_set_share(db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    use crate::portfolio::share::{ShareSurface, ShareWrite, ShowcaseStatus, Visibility};
    let body = request.json_body();
    let title = match required_field(&body, "title") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let summary = match required_field(&body, "summary") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let category = match required_field(&body, "category") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let source_url = match required_field(&body, "source_url") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let visibility = match optional_field(&body, "visibility") {
        Some(raw) => match Visibility::parse(raw) {
            Ok(value) => value,
            Err(reason) => return bad_request(&reason),
        },
        None => Visibility::Public,
    };
    let showcase_status = match optional_field(&body, "showcase_status") {
        Some(raw) => match ShowcaseStatus::parse(raw) {
            Ok(value) => value,
            Err(reason) => return bad_request(&reason),
        },
        None => ShowcaseStatus::Unknown,
    };
    let mut surfaces = Vec::new();
    if let Some(entries) = body.get("surfaces").and_then(|v| v.as_array()) {
        for entry in entries {
            let label = entry
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let url = entry
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            // Unvalidated on purpose: the registry's `validate_share`
            // is the single gate, so the refusal and its persisted
            // finding look the same whether they came from the CLI or
            // from this route.
            surfaces.push(ShareSurface::new(label, url));
        }
    }
    let write = ShareWrite {
        title,
        summary,
        category,
        source_url,
        demo_url: optional_field(&body, "demo_url").map(|value| value.to_string()),
        visibility,
        featured: body
            .get("featured")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        showcase_status,
        status_evidence: optional_field(&body, "status_evidence").map(|value| value.to_string()),
        surfaces,
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.share_upsert_record(id, &write) {
        Ok(record) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "project_id": id, "record": record },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/share/remove`
fn handle_remove_share(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.share_remove_record(id) {
        Ok(removed) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": {
                    "project_id": id,
                    "shared": false,
                    "removed": removed,
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/share/manifest`
fn handle_share_preview(db_path: &Path, now: DateTime<Utc>) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::portfolio::publication::preview_manifest(&registry) {
        Ok(draft) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": {
                    "manifest_revision": draft.body.manifest_revision,
                    "manifest_sha256": draft.manifest_sha256(),
                    "project_count": draft.project_count(),
                    "approvable": draft.approvable(),
                    "canonical_body": draft.body.canonical_json(),
                    "findings": draft.findings,
                    "generated_at": now.to_rfc3339(),
                },
                "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/share/approve`
fn handle_share_approve(db_path: &Path, request: &ApiRequest, actor: &str) -> ApiResponse {
    let body = request.json_body();
    let manifest_sha256 = match required_field(&body, "manifest_sha256") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.share_approve(&manifest_sha256, actor) {
        Ok(approval) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "approval": approval },
                "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/share/publish`
fn handle_share_publish(
    db_path: &Path,
    request: &ApiRequest,
    actor: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let target = match required_field(&body, "target") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let operation_key = match required_field(&body, "operation_key") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let adapter = optional_field(&body, "adapter").map(PathBuf::from);
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let plan = crate::portfolio::publication::PublishPlan {
        operation_key,
        target,
        actor: actor.to_string(),
        adapter,
    };
    match crate::portfolio::publication::publish_approved_manifest(&registry, &plan, now) {
        Ok(report) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "publication": report },
                "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/share/reconcile`
fn handle_share_reconcile(db_path: &Path, request: &ApiRequest, actor: &str) -> ApiResponse {
    use crate::portfolio::share::PublicationStatus;
    let body = request.json_body();
    let raw_id = match required_field(&body, "publication_id") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let publication_id = match raw_id.parse::<i64>() {
        Ok(value) => value,
        Err(_) => return bad_request("publication_id must be an integer"),
    };
    let status = match required_field(&body, "status") {
        Ok(value) => value.to_string(),
        Err(response) => return response,
    };
    let status = match PublicationStatus::parse(&status) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.share_reconcile_publication(publication_id, status, actor) {
        Ok(attempt) => ApiResponse::json(
            200,
            serde_json::json!({
                "share": { "publication": attempt },
                "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/share/audit`
fn handle_share_audit(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let limit = match parse_share_limit(request.query.as_deref().unwrap_or_default()) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let approvals = match registry.share_approvals(limit) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let publications = match registry.share_publications(limit) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let unreconciled = match registry.share_unreconciled_publication() {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    ApiResponse::json(
        200,
        serde_json::json!({
            "share": {
                "approvals": approvals,
                "publications": publications,
                "unreconciled": unreconciled,
            },
            "contract": crate::portfolio::share::SHARE_CONTRACT_VERSION,
        }),
    )
}

/// Parse the `limit` query parameter of the share audit route. An
/// absent parameter is the house default; a present but unusable one
/// is a typed refusal rather than a silently widened list.
fn parse_share_limit(query: &str) -> Result<usize, String> {
    let Some(pair) = query.split('&').find(|part| !part.is_empty()) else {
        return Ok(50);
    };
    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
    if key.trim() != "limit" {
        return Err(format!("unknown share audit query parameter `{key}`"));
    }
    let parsed = value
        .trim()
        .parse::<usize>()
        .map_err(|_| "limit must be an integer".to_string())?;
    if !(1..=500).contains(&parsed) {
        return Err("limit must be between 1 and 500".to_string());
    }
    Ok(parsed)
}

// --- portfolio interest --------------------------------------------------
//
// Every handler below dispatches into
// [`crate::portfolio::interest_report`], the same orchestration the
// CLI uses, so the JSON transport adds no business rule of its own.
// The authorization boundary has already demanded `admin:access` for
// every one of these routes — the reads included, because which
// projects draw interest is itself private — so an unauthorized
// request never reaches a read and never persists an import.

/// `GET /v1/projects/{id}/interest`
fn handle_get_interest(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    let query = request.query.as_deref().unwrap_or_default();
    let stale_after_days = match parse_interest_stale_after_days(query) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::portfolio::interest_report::project_interest(&registry, id, stale_after_days, now)
    {
        Ok(projection) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": {
                    "project_id": id,
                    "measured": !projection.snapshots.is_empty(),
                    "projection": projection,
                },
                "contract": crate::portfolio::interest::INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `POST /v1/projects/{id}/interest`
fn handle_import_interest(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    actor: &str,
    now: DateTime<Utc>,
) -> ApiResponse {
    use crate::portfolio::interest::{RawSnapshot, INTEREST_CONTRACT_VERSION};
    let body = request.json_body();
    // The body carries the same array an importer hands the CLI, with
    // the path supplying the project: a body that names a different
    // project is refused rather than silently re-scoped.
    let Some(entries) = body.get("snapshots").and_then(|v| v.as_array()) else {
        return bad_request("interest import requires a `snapshots` array");
    };
    if request.body.len() > crate::portfolio::interest::MAX_IMPORT_BYTES {
        return bad_request(&format!(
            "import document is larger than {} bytes",
            crate::portfolio::interest::MAX_IMPORT_BYTES
        ));
    }
    let mut records: Vec<RawSnapshot> = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let Value::Object(record) = entry else {
            return bad_request(&format!(
                "snapshot {index} must be a JSON object; an aggregate record carries no free-form value"
            ));
        };
        match record.get("project_id").and_then(|v| v.as_str()) {
            Some(declared) if declared.trim() != id => {
                return bad_request(&format!(
                    "snapshot {index} declares project_id `{declared}` but the route targets `{id}`"
                ))
            }
            _ => {}
        }
        // Unvalidated on purpose: the registry's `validate_snapshot`
        // is the single gate, so a refusal looks the same whether the
        // record arrived over HTTP or from a file. The route supplies
        // the project so a caller cannot import one project's evidence
        // under another's name by accident.
        let mut record = record.clone();
        record.insert("project_id".to_string(), Value::String(id.to_string()));
        records.push(RawSnapshot { index, record });
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    // The importer is the authenticated session subject, never a
    // client-claimed string: provenance is the point of the store.
    match crate::portfolio::interest_report::import_snapshots(
        &registry,
        &crate::portfolio::interest::InterestImport { records },
        actor,
        now,
    ) {
        Ok(imported) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": {
                    "project_id": id,
                    "import": imported,
                },
                "contract": INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/interest/compare`
fn handle_interest_compare(
    db_path: &Path,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    use crate::portfolio::interest::{InterestMetric, INTEREST_CONTRACT_VERSION};
    let query = request.query.as_deref().unwrap_or_default().to_string();
    let mut projects: Vec<String> = Vec::new();
    let mut metrics: Vec<String> = Vec::new();
    let mut source: Option<String> = None;
    let mut stale_after_days = crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS;
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key.trim() {
            "projects" => projects.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string),
            ),
            "metric" => metrics.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string),
            ),
            "source" => source = Some(value),
            "stale_after_days" => match value.trim().parse::<i64>() {
                Ok(parsed) => stale_after_days = parsed,
                Err(_) => return bad_request("stale_after_days must be an integer"),
            },
            other => {
                return bad_request(&format!(
                    "unknown interest compare query parameter `{other}`"
                ))
            }
        }
    }
    if projects.is_empty() {
        return bad_request("interest compare requires a `projects` parameter");
    }
    let selected: Vec<InterestMetric> = if metrics.is_empty() {
        InterestMetric::ALL.to_vec()
    } else {
        let mut chosen = Vec::with_capacity(metrics.len());
        for raw in &metrics {
            match InterestMetric::parse(raw) {
                Ok(metric) if !chosen.contains(&metric) => chosen.push(metric),
                Ok(_) => {}
                Err(reason) => return bad_request(&reason),
            }
        }
        chosen
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::portfolio::interest_report::compare_projects(
        &registry,
        &projects,
        &selected,
        source.as_deref(),
        stale_after_days,
        now,
    ) {
        Ok(comparison) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": { "comparison": comparison },
                "contract": INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/interest/trend`
fn handle_interest_trend(db_path: &Path, request: &ApiRequest, now: DateTime<Utc>) -> ApiResponse {
    use crate::portfolio::interest::{InterestMetric, INTEREST_CONTRACT_VERSION};
    let query = request.query.as_deref().unwrap_or_default().to_string();
    let mut project: Option<String> = None;
    let mut metric: Option<String> = None;
    let mut limit = 12usize;
    let mut stale_after_days = crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS;
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key.trim() {
            "project" => project = Some(value),
            "metric" => metric = Some(value),
            "limit" => match value.trim().parse::<usize>() {
                Ok(parsed) => limit = parsed,
                Err(_) => return bad_request("limit must be an integer"),
            },
            "stale_after_days" => match value.trim().parse::<i64>() {
                Ok(parsed) => stale_after_days = parsed,
                Err(_) => return bad_request("stale_after_days must be an integer"),
            },
            other => {
                return bad_request(&format!("unknown interest trend query parameter `{other}`"))
            }
        }
    }
    let (Some(project), Some(raw_metric)) = (project, metric) else {
        return bad_request("interest trend requires `project` and `metric` parameters");
    };
    let metric = match InterestMetric::parse(raw_metric.trim()) {
        Ok(metric) => metric,
        Err(reason) => return bad_request(&reason),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::portfolio::interest_report::interest_trend(
        &registry,
        project.trim(),
        metric,
        limit,
        stale_after_days,
        now,
    ) {
        Ok(trend) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": { "trend": trend },
                "contract": INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/interest/audit`
fn handle_interest_audit(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let limit = match parse_interest_limit(request.query.as_deref().unwrap_or_default()) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.interest_findings(limit) {
        Ok(findings) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": { "refusals": findings },
                "contract": crate::portfolio::interest::INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/interest/readiness`
/// Read-only verdict on whether aggregate evidence justifies the
/// product-owned activation follow-up. Always answers `200` for both
/// verdicts: the gate exit code is a CLI concept and an HTTP client
/// reads the verdict from the body.
fn handle_interest_readiness(
    db_path: &Path,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    use crate::portfolio::interest::{InterestMetric, ACTIVATION_CONTRACT_VERSION};
    let query = request.query.as_deref().unwrap_or_default().to_string();
    let mut project: Option<String> = None;
    let mut projects: Option<Vec<String>> = None;
    let mut metric: Option<String> = None;
    let mut min_value: Option<u64> = None;
    let mut source: Option<String> = None;
    let mut window: Option<String> = None;
    let mut stale_after_days = crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS;
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key.trim() {
            "project" => project = Some(value),
            "projects" => {
                projects = Some(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|part| !part.is_empty())
                        .map(str::to_string)
                        .collect(),
                );
            }
            "metric" => metric = Some(value),
            "min_value" => match value.trim().parse::<u64>() {
                Ok(parsed) => min_value = Some(parsed),
                Err(_) => return bad_request("min_value must be an integer"),
            },
            "source" => source = Some(value),
            "window" => window = Some(value),
            "stale_after_days" => match value.trim().parse::<i64>() {
                Ok(parsed) => stale_after_days = parsed,
                Err(_) => return bad_request("stale_after_days must be an integer"),
            },
            other => {
                return bad_request(&format!(
                    "unknown interest readiness query parameter `{other}`"
                ))
            }
        }
    }
    if project.is_some() && projects.is_some() {
        return bad_request("interest readiness takes `project` or `projects`, not both");
    }
    let Some(raw_metric) = metric else {
        return bad_request("interest readiness requires a `metric` parameter");
    };
    let metric = match InterestMetric::parse(raw_metric.trim()) {
        Ok(metric) => metric,
        Err(reason) => return bad_request(&reason),
    };
    let threshold = match min_value {
        Some(value) => match crate::portfolio::interest::validate_threshold(value) {
            Ok(bound) => Some(bound),
            Err(reason) => return bad_request(&reason),
        },
        None => None,
    };
    let source = match source {
        Some(raw) if raw.trim().is_empty() => {
            return bad_request("source must not be blank");
        }
        Some(raw) => Some(raw.trim().to_string()),
        None => None,
    };
    let parsed_window = match window {
        Some(raw) => match crate::portfolio::interest::parse_window(raw.trim()) {
            Ok((start, end)) => Some((start, end)),
            Err(reason) => return bad_request(&reason),
        },
        None => None,
    };
    let window_pair = parsed_window
        .as_ref()
        .map(|(start, end)| (start.as_str(), end.as_str()));
    if let Err(reason) = crate::portfolio::interest::bound_stale_after_days(stale_after_days) {
        return bad_request(&reason);
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_ids: Vec<String> = match (project, projects) {
        (Some(one), None) => vec![one.trim().to_string()],
        (None, Some(many)) => many,
        (None, None) => match registry.list() {
            Ok(records) => records.into_iter().map(|record| record.id).collect(),
            Err(err) => return ApiResponse::from_error(&err),
        },
        (Some(_), Some(_)) => {
            return bad_request("interest readiness takes `project` or `projects`, not both");
        }
    };
    match crate::portfolio::interest_report::activation_readiness(
        &registry,
        &project_ids,
        metric,
        threshold,
        source.as_deref(),
        window_pair,
        stale_after_days,
        now,
    ) {
        Ok(report) => ApiResponse::json(
            200,
            serde_json::json!({
                "contract": ACTIVATION_CONTRACT_VERSION,
                "interest": {
                    "activation": {
                        "metric": report.metric,
                        "threshold": report.threshold,
                        "stale_after_days": report.stale_after_days,
                        "requested_window": report.requested_window,
                        "requested_source": report.requested_source,
                        "ready": report.is_ready(),
                        "ready_count": report.ready_count,
                        "not_ready_count": report.not_ready_count,
                        "verdicts": report.verdicts,
                    },
                },
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// Parse the `stale_after_days` query parameter. An absent parameter
/// is the house default; a present but unusable one is a typed refusal
/// rather than a silently clamped bound.
fn parse_interest_stale_after_days(query: &str) -> Result<i64, ApiResponse> {
    let Some(pair) = query.split('&').find(|part| !part.is_empty()) else {
        return Ok(crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS);
    };
    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
    if key.trim() != "stale_after_days" {
        return Err(bad_request(&format!(
            "unknown interest query parameter `{key}`"
        )));
    }
    let parsed = value
        .trim()
        .parse::<i64>()
        .map_err(|_| bad_request("stale_after_days must be an integer"))?;
    crate::portfolio::interest::bound_stale_after_days(parsed)
        .map_err(|reason| bad_request(&reason))
}

/// Parse the `limit` query parameter of the interest audit route.
fn parse_interest_limit(query: &str) -> Result<usize, String> {
    let Some(pair) = query.split('&').find(|part| !part.is_empty()) else {
        return Ok(50);
    };
    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
    if key.trim() != "limit" {
        return Err(format!("unknown interest audit query parameter `{key}`"));
    }
    let parsed = value
        .trim()
        .parse::<usize>()
        .map_err(|_| "limit must be an integer".to_string())?;
    if !(1..=500).contains(&parsed) {
        return Err("limit must be between 1 and 500".to_string());
    }
    Ok(parsed)
}

/// Decode the `%XX` escapes a query string may carry.
///
/// Parsed query parameters for `GET /v1/projects/catalog` and
/// `GET /v1/projects/{id}/catalog`. Every field is the typed
/// counterpart of a catalog CLI flag; the handler builds the
/// `CatalogQuery` from these without re-parsing the wire format
/// itself.
#[derive(Debug, Clone)]
struct CatalogQueryParams {
    sources: Vec<String>,
    tags: Vec<String>,
    languages: Vec<String>,
    profiles: Vec<String>,
    lifecycles: Vec<String>,
    repositories: Vec<String>,
    ci: Vec<String>,
    compose: Vec<String>,
    evidence: Vec<String>,
    filters: Vec<String>,
    limit: usize,
    cursor: Option<String>,
    max_age: i64,
    workspace_registry: Option<PathBuf>,
    inventory: Option<PathBuf>,
    git_repositories: Vec<PathBuf>,
    github_repositories: Vec<String>,
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

/// Parse the query string of a `/v1/projects/catalog` request into a
/// [`CatalogQueryParams`]. Unknown keys are a typed `api-invalid`
/// refusal — the same shape the interest routes use — so a typo
/// never silently disables a filter.
fn parse_catalog_query_params(raw: Option<&str>) -> Result<CatalogQueryParams, ApiResponse> {
    let mut params = CatalogQueryParams::default();
    let query = raw.unwrap_or_default();
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key.trim() {
            "source" => params.sources.push(value),
            "tag" => params.tags.push(value),
            "language" => params.languages.push(value),
            "profile" => params.profiles.push(value),
            "lifecycle" => params.lifecycles.push(value),
            "repository" | "repo" => params.repositories.push(value),
            "ci" => params.ci.push(value),
            "compose" => params.compose.push(value),
            "evidence" => params.evidence.push(value),
            "filter" => params.filters.push(value),
            "limit" => match value.trim().parse::<usize>() {
                Ok(parsed) => params.limit = parsed,
                Err(_) => {
                    return Err(bad_request("limit must be a positive integer"));
                }
            },
            "cursor" => {
                if !value.trim().is_empty() {
                    params.cursor = Some(value);
                }
            }
            "max-age" | "max_age" => match value.trim().parse::<i64>() {
                Ok(parsed) => params.max_age = parsed,
                Err(_) => return Err(bad_request("max-age must be an integer")),
            },
            "workspace-registry" | "workspace_registry" => {
                if !value.trim().is_empty() {
                    params.workspace_registry = Some(PathBuf::from(value));
                }
            }
            "inventory" => {
                if !value.trim().is_empty() {
                    params.inventory = Some(PathBuf::from(value));
                }
            }
            "git-repository" | "git_repository" => {
                if !value.trim().is_empty() {
                    params.git_repositories.push(PathBuf::from(value));
                }
            }
            "github-repository" | "github_repository" => {
                if !value.trim().is_empty() {
                    params.github_repositories.push(value);
                }
            }
            other => {
                return Err(bad_request(&format!(
                    "unknown catalog query parameter `{other}`"
                )));
            }
        }
    }
    Ok(params)
}

/// Build a `CatalogSourceSelection` from the request params.
/// Unknown source kinds are a typed `api-invalid` refusal so the
/// transport surfaces a 400 rather than a silent no-op.
fn build_catalog_selection(
    params: &CatalogQueryParams,
) -> Result<catalog::CatalogSourceSelection, ApiResponse> {
    let mut kinds: Vec<catalog::SourceKind> = Vec::new();
    if params.sources.is_empty() {
        kinds.push(catalog::SourceKind::Local);
    } else {
        for raw in &params.sources {
            let kind = catalog::SourceKind::parse(raw).ok_or_else(|| {
                bad_request(&format!(
                    "unknown --source `{raw}`; expected one of \
                     local|git|workspace-registry|inventory|github"
                ))
            })?;
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    let workspace_registry = params
        .workspace_registry
        .clone()
        .or_else(|| crate::fleet::resolve_registry_path(None));
    let inventory = params
        .inventory
        .clone()
        .or_else(|| crate::publish::inventory::resolve_source(None));
    Ok(catalog::CatalogSourceSelection {
        kinds,
        git_repositories: params.git_repositories.clone(),
        workspace_registry,
        inventory,
        github_repositories: params.github_repositories.clone(),
    })
}

/// Flatten the typed filters plus the generic `key=value` filters
/// into one ordered pair list. `CatalogQuery::from_pairs` refuses
/// an unknown key, so the surface contract — the same wire form
/// the CLI uses — stays the only place filter keys are named.
fn catalog_filter_pairs_from(params: &CatalogQueryParams) -> Vec<String> {
    let mut pairs: Vec<String> = Vec::new();
    for value in &params.tags {
        pairs.push(format!("tag={value}"));
    }
    for value in &params.languages {
        pairs.push(format!("language={value}"));
    }
    for value in &params.profiles {
        pairs.push(format!("profile={value}"));
    }
    for value in &params.lifecycles {
        pairs.push(format!("lifecycle={value}"));
    }
    for value in &params.repositories {
        pairs.push(format!("repository={value}"));
    }
    for value in &params.ci {
        pairs.push(format!("ci={value}"));
    }
    for value in &params.compose {
        pairs.push(format!("compose={value}"));
    }
    for value in &params.evidence {
        pairs.push(format!("evidence={value}"));
    }
    pairs.extend(params.filters.iter().cloned());
    pairs
}

/// The interest routes take a comma-separated project list, so a
/// caller whose ids ever need escaping could not otherwise express
/// them. Only the three characters that actually change a query's
/// meaning are decoded; anything else is left verbatim rather than
/// guessed at.
fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok();
            if let Some(byte) = hex.and_then(|value| u8::from_str_radix(value, 16).ok()) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        if bytes[index] == b'+' {
            out.push(b' ');
            index += 1;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Reservation helper. Reserves a pending operation,
/// invokes the closure, then finalizes the operation with/// `done` or `failed`. Returns the closure's value plus
/// the operation id.
///
/// The closure receives the reserved `op_id` and a
/// read-only [`Registry`] handle. Handlers that need to
/// mutate the registry (for example `apply_upgrade`)
/// should re-open the registry inside the closure
/// because [`Registry::open`] consumes the path and the
/// original handle is borrowed for the duration of the
/// reservation.
fn run_with_operation<T, F>(
    db_path: &Path,
    kind: &str,
    project_id: &str,
    request: &ApiRequest,
    f: F,
) -> Result<(T, i64, String), ApiResponse>
where
    F: FnOnce(i64, &Registry) -> Result<(T, i64, String), ForgeError>,
{
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return Err(ApiResponse::from_error(&err)),
    };
    let request_hash = request_hash(request);
    let key_owned;
    let key = match request.idempotency_key.as_deref() {
        Some(value) => {
            key_owned = value.to_string();
            Some(key_owned.as_str())
        }
        None => None,
    };
    let outcome = match key {
        Some(value) => {
            match registry.reserve_idempotent_operation(kind, project_id, value, &request_hash) {
                Ok(value) => value,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            }
        }
        None => {
            // No key: insert a regular pending row and
            // always go through finalize so the journal is
            // consistent.
            let detail = format!("{kind} initiated by api");
            if let Err(err) = registry.record_operation(kind, project_id, "pending", &detail) {
                return Err(ApiResponse::from_error(&err));
            }
            let entries = registry.journal_entries().ok();
            let op_id = entries
                .as_ref()
                .and_then(|list| {
                    list.iter()
                        .rev()
                        .find(|e| e.kind == kind && e.project_id == project_id)
                        .map(|e| e.op_id)
                })
                .unwrap_or(0);
            ReservationOutcome::Reserved { op_id }
        }
    };
    let op_id = match outcome {
        ReservationOutcome::Reserved { op_id } => op_id,
        ReservationOutcome::Reused { op_id } => {
            // Replay path: return the existing operation
            // without re-running the closure.
            return Err(pending_or_done_response(&registry, op_id, kind, project_id));
        }
    };
    match f(op_id, &registry) {
        Ok((value, op_id, project_id)) => {
            let detail = format!("{kind} completed");
            let _ = registry.finalize_operation(op_id, "done", &detail);
            Ok((value, op_id, project_id))
        }
        Err(err) => {
            let detail = err.to_string();
            let _ = registry.finalize_operation(op_id, "failed", &detail);
            Err(ApiResponse::from_error(&err))
        }
    }
}

fn pending_or_done_response(
    registry: &Registry,
    op_id: i64,
    kind: &str,
    project_id: &str,
) -> ApiResponse {
    // Try to fetch the existing entry; if we can, return
    // its current state with `200 OK` so the retry sees
    // the same operation identity the boundary scenario
    // requires.
    if let Ok(Some(entry)) = registry.operation(op_id) {
        let status = if entry.state == "pending" { 202 } else { 200 };
        return ApiResponse::json(
            status,
            serde_json::json!({
                "operation": entry,
                "contract": API_CONTRACT_VERSION,
                "replay": true,
                "kind": kind,
                "project_id": project_id,
            }),
        );
    }
    ApiResponse::json(
        500,
        serde_json::json!({
            "error": {
                "code": "api-internal",
                "message": "operation reservation succeeded but the record could not be re-read"
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

fn request_hash(request: &ApiRequest) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(request.method.as_bytes());
    hasher.update(b"\n");
    hasher.update(request.path.as_bytes());
    hasher.update(b"\n");
    hasher.update(&request.body);
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

// ---- wire parser ---------------------------------------------------

/// Parse one HTTP/1.1 request from the wire. The
/// transport is intentionally minimal: it accepts the
/// request line plus headers, reads `Content-Length`
/// bytes, and refuses anything larger than
/// `max_body_bytes`. The body is read into a `Vec<u8>` so
/// the handler can interpret it.
pub fn parse_request(
    raw: &[u8],
    remote_addr: Option<SocketAddr>,
    max_body_bytes: usize,
) -> Result<ApiRequest, ApiError> {
    let header_end = find_header_end(raw).ok_or_else(|| {
        ApiError::Parse("no \\r\\n\\r\\n separator between headers and body".to_string())
    })?;
    let body_offset = header_end + 4;
    let body = if body_offset < raw.len() {
        raw[body_offset..].to_vec()
    } else {
        Vec::new()
    };
    if body.len() > max_body_bytes {
        return Err(ApiError::BodyTooLarge {
            limit: max_body_bytes,
            got: body.len(),
        });
    }
    let header_text = std::str::from_utf8(&raw[..header_end])
        .map_err(|err| ApiError::Parse(format!("non-UTF8 header: {err}")))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| ApiError::Parse("request line missing".to_string()))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| ApiError::Parse("method missing".to_string()))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| ApiError::Parse("request target missing".to_string()))?;
    let version = parts
        .next()
        .ok_or_else(|| ApiError::Parse("HTTP version missing".to_string()))?;
    if !version.starts_with("HTTP/") {
        return Err(ApiError::Parse(format!(
            "unsupported protocol `{version}`; HTTP/1.1 only"
        )));
    }
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), Some(q.to_string())),
        None => (target.to_string(), None),
    };
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| ApiError::Parse(format!("malformed header `{line}`")))?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    let idempotency_key = headers
        .get("idempotency-key")
        .filter(|value| !value.is_empty())
        .cloned();
    let bearer_token = headers.get("authorization").and_then(|value| {
        let (scheme, token) = value.split_once(' ')?;
        if scheme.eq_ignore_ascii_case("Bearer") {
            Some(token.trim().to_string())
        } else {
            None
        }
    });
    let cookies = headers
        .get("cookie")
        .map(|raw| parse_cookie_header(raw))
        .unwrap_or_default();
    Ok(ApiRequest {
        method,
        path,
        query,
        headers,
        body,
        idempotency_key,
        bearer_token,
        cookies,
        remote_addr,
        started_at: Utc::now(),
    })
}

/// Parse a `Cookie:` header into a name→value map. The
/// header is a `;`-separated list of `name=value` pairs;
/// whitespace is trimmed and the value is returned
/// undecoded because session cookies are hex tokens that
/// never need URL escaping. Duplicate names keep the first
/// occurrence so a forged `Cookie:` header cannot smuggle
/// a second `forge_session` value past the dispatch.
pub fn parse_cookie_header(raw: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for pair in raw.split(';') {
        let trimmed = pair.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (name, value) = match trimmed.split_once('=') {
            Some((n, v)) => (n.trim(), v.trim()),
            None => continue,
        };
        if name.is_empty() {
            continue;
        }
        out.entry(name.to_string())
            .or_insert_with(|| value.to_string());
    }
    out
}

fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n")
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
                serde_json::json!({
                    "error": {
                        "code": "api-body-too-large",
                        "message": format!("body is {got} bytes; limit is {limit}")
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            ),
            ApiError::Io(reason) => ApiResponse::json(
                500,
                serde_json::json!({
                    "error": {
                        "code": "api-internal",
                        "message": format!("transport io error: {reason}")
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            ),
            ApiError::Timeout => ApiResponse::json(
                408,
                serde_json::json!({
                    "error": {
                        "code": "api-timeout",
                        "message": "request exceeded the handler timeout"
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            ),
        }
    }
}

impl From<io::Error> for ApiError {
    fn from(err: io::Error) -> Self {
        ApiError::Io(err.to_string())
    }
}

// ---- wire writer ---------------------------------------------------

/// Render a response back to the socket. Single-shot
/// write: status line, headers, body. Connection: close
/// is the safe default for a low-throughput control
/// plane; clients open a fresh connection per request.
pub fn write_response(stream: &mut TcpStream, response: &ApiResponse) -> io::Result<()> {
    let reason = reason_phrase(response.status);
    let mut buffer = Vec::with_capacity(128 + response.body.len());
    buffer.extend_from_slice(
        format!("HTTP/1.1 {status} {reason}\r\n", status = response.status).as_bytes(),
    );
    buffer.extend_from_slice(b"connection: close\r\n");
    for (name, value) in &response.headers {
        buffer.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    buffer.extend_from_slice(format!("content-length: {}\r\n", response.body.len()).as_bytes());
    buffer.extend_from_slice(b"\r\n");
    buffer.extend_from_slice(&response.body);
    stream.write_all(&buffer)?;
    stream.flush()?;
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

// ---- TCP server loop -----------------------------------------------

/// Shared shutdown signal. The server loop checks the
/// flag at every accept so the CLI can stop the listener
/// without killing the process.
#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    flag: Arc<AtomicBool>,
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

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Bind a TCP listener and serve the API until the
/// shutdown signal fires. The function returns the
/// number of accepted connections on graceful shutdown.
/// The listener is bound to `config.socket_addr()` so a
/// non-loopback bind is the operator's choice, never the
/// default.
pub fn serve(
    config: &ApiConfig,
    db_path: &Path,
    shutdown: ShutdownSignal,
) -> Result<usize, ForgeError> {
    let listener =
        TcpListener::bind(config.socket_addr()).map_err(|err| ForgeError::ApiInvalid {
            reason: format!(
                "cannot bind api listener on {}: {err}",
                config.socket_addr()
            ),
        })?;
    listener
        .set_nonblocking(false)
        .map_err(|err| ForgeError::ApiInvalid {
            reason: format!("cannot configure api listener: {err}"),
        })?;
    let mut accepted = 0usize;
    for stream in listener.incoming() {
        if shutdown.is_set() {
            break;
        }
        let mut stream = match stream {
            Ok(value) => value,
            Err(err) => {
                // Transient socket error: log and continue.
                eprintln!("forge api: accept error: {err}");
                continue;
            }
        };
        accepted += 1;
        let db_path = db_path.to_path_buf();
        let shutdown = shutdown.clone();
        let max_body = config.max_body_bytes;
        let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
        let _ = stream.set_write_timeout(Some(HANDLER_TIMEOUT));
        // Inline handler so the test can drive the
        // server with a single read. The accept loop is
        // single-threaded by design (the contract is
        // "low-throughput control plane"), but a future
        // revision can move the per-connection logic
        // into a thread pool without changing the wire
        // contract.
        let response = handle_one(config, &mut stream, &db_path, max_body, shutdown);
        let _ = write_response(&mut stream, &response);
    }
    Ok(accepted)
}

fn handle_one(
    config: &ApiConfig,
    stream: &mut TcpStream,
    db_path: &Path,
    max_body: usize,
    _shutdown: ShutdownSignal,
) -> ApiResponse {
    let mut buffer = Vec::with_capacity(2048);
    let mut chunk = [0u8; 4096];
    let start = Instant::now();
    loop {
        if start.elapsed() > READ_TIMEOUT {
            return ApiError::Timeout.to_response();
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&chunk[..n]);
                if find_header_end(&buffer).is_some() {
                    // Once we have the header terminator,
                    // read Content-Length more bytes for
                    // the body. Cap at max_body_bytes
                    // before allocating.
                    let header_end = find_header_end(&buffer).unwrap();
                    let body_offset = header_end + 4;
                    let content_length = header_content_length(&buffer[..header_end]).unwrap_or(0);
                    let desired = body_offset + content_length;
                    if content_length > max_body {
                        let limit = max_body;
                        let got = content_length;
                        return ApiError::BodyTooLarge { limit, got }.to_response();
                    }
                    while buffer.len() < desired {
                        if start.elapsed() > READ_TIMEOUT {
                            return ApiError::Timeout.to_response();
                        }
                        match stream.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                            Err(err) if err.kind() == io::ErrorKind::WouldBlock => continue,
                            Err(err) => return ApiError::Io(err.to_string()).to_response(),
                        }
                    }
                    break;
                }
                if buffer.len() > max_body + 8192 {
                    return ApiError::BodyTooLarge {
                        limit: max_body,
                        got: buffer.len(),
                    }
                    .to_response();
                }
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => continue,
            Err(err) => return ApiError::Io(err.to_string()).to_response(),
        }
    }
    let remote_addr = stream.peer_addr().ok();
    let request = match parse_request(&buffer, remote_addr, max_body) {
        Ok(value) => value,
        Err(err) => return err.to_response(),
    };
    handle(config, db_path, &request, Utc::now())
}

fn header_content_length(header_text: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(header_text).ok()?;
    for line in text.split("\r\n").skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                return value.trim().parse().ok();
            }
        }
    }
    None
}

// ---- test helpers --------------------------------------------------

/// Read a full request from a `BufRead` source and
/// return the response. Exposed for tests so the
/// in-process server can be driven without a socket.
pub fn handle_buffered<R: BufRead, W: Write>(
    config: &ApiConfig,
    db_path: &Path,
    reader: &mut R,
    writer: &mut W,
    max_body_bytes: usize,
) -> io::Result<ApiResponse> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let request = match parse_request(&buffer, None, max_body_bytes) {
        Ok(value) => value,
        Err(err) => {
            let response = err.to_response();
            let body = serde_json::to_string(
                &serde_json::from_slice::<Value>(&response.body).unwrap_or(Value::Null),
            )
            .unwrap_or_else(|_| "{}".to_string());
            writeln!(writer, "{}", body)?;
            return Ok(response);
        }
    };
    let response = handle(config, db_path, &request, Utc::now());
    writeln!(
        writer,
        "{}",
        serde_json::to_string(
            &serde_json::from_slice::<Value>(&response.body).unwrap_or(Value::Null)
        )
        .unwrap_or_else(|_| "{}".to_string())
    )?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_matches_healthz_and_project_routes() {
        assert!(matches!(
            route_request("GET", "/healthz"),
            Some(Route::Healthz)
        ));
        assert!(matches!(
            route_request("GET", "/v1/projects"),
            Some(Route::ListProjects)
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects"),
            Some(Route::CreateProject)
        ));
        assert!(matches!(
            route_request("GET", "/v1/projects/rust-web"),
            Some(Route::InspectProject { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/doctor"),
            Some(Route::Doctor { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("GET", "/v1/projects/rust-web/governance"),
            Some(Route::Governance { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/features"),
            Some(Route::AddFeature { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/upgrade"),
            Some(Route::UpgradeProject { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/specs"),
            Some(Route::GenerateSpec { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/agents"),
            Some(Route::AgentTransition { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/deployments"),
            Some(Route::ApplyDeployment { ref id }) if id == "rust-web"
        ));
        assert_eq!(
            route_request("POST", "/v1/publish/github"),
            Some(Route::GitHubPush)
        );
        assert!(matches!(
            route_request("GET", "/v1/operations/42"),
            Some(Route::GetOperation { op_id: 42 })
        ));
    }

    #[test]
    fn routing_rejects_unknown_paths() {
        assert!(route_request("GET", "/v2/projects").is_none());
        assert!(route_request("GET", "/nope").is_none());
        assert!(route_request("GET", "/v1/operations/abc").is_none());
    }

    #[test]
    fn parse_request_extracts_method_path_and_headers() {
        let raw = b"GET /v1/projects HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer deadbeef\r\nIdempotency-Key: abc-123\r\nContent-Length: 0\r\n\r\n";
        let req = parse_request(raw, None, MAX_BODY_BYTES).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/v1/projects");
        assert_eq!(req.bearer_token.as_deref(), Some("deadbeef"));
        assert_eq!(req.idempotency_key.as_deref(), Some("abc-123"));
    }

    #[test]
    fn parse_request_caps_body_at_max_bytes() {
        let raw = b"POST /v1/projects HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello";
        let err = parse_request(raw, None, 4).unwrap_err();
        assert!(matches!(err, ApiError::BodyTooLarge { .. }));
    }

    #[test]
    fn err_status_maps_unauthorized_and_conflict_codes() {
        assert_eq!(
            err_status(&ForgeError::ApiUnauthorized { reason: "x".into() }),
            401
        );
        assert_eq!(
            err_status(&ForgeError::ApiProjectMismatch { reason: "x".into() }),
            403
        );
        assert_eq!(
            err_status(&ForgeError::IdempotencyKeyConflict { reason: "x".into() }),
            409
        );
        assert_eq!(err_status(&ForgeError::PushConfirmRequired), 409);
        assert_eq!(
            err_status(&ForgeError::UnknownProject { query: "x".into() }),
            400
        );
    }

    #[test]
    fn healthz_route_does_not_require_authorization() {
        // `GET /healthz` should render the contract version
        // even when no bearer token is supplied.
        let raw = b"GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let request = parse_request(raw, None, MAX_BODY_BYTES).unwrap();
        assert!(request.bearer_token.is_none());
        assert_eq!(
            route_request(&request.method, &request.path),
            Some(Route::Healthz)
        );
    }

    #[test]
    fn required_permission_is_admin_for_mutating_routes() {
        assert_eq!(required_permission(&Route::Healthz), None);
        assert_eq!(required_permission(&Route::ListProjects), None);
        assert_eq!(
            required_permission(&Route::InspectProject { id: "a".into() }),
            None
        );
        assert_eq!(
            required_permission(&Route::CreateProject),
            Some("admin:access")
        );
        assert_eq!(
            required_permission(&Route::AddFeature { id: "a".into() }),
            Some("admin:access")
        );
        assert_eq!(
            required_permission(&Route::ApplyDeployment { id: "a".into() }),
            Some("admin:access")
        );
    }

    #[test]
    fn is_mutating_classifies_routes() {
        assert!(!is_mutating(&Route::Healthz));
        assert!(!is_mutating(&Route::ListProjects));
        assert!(!is_mutating(&Route::GetOperation { op_id: 1 }));
        assert!(is_mutating(&Route::CreateProject));
        assert!(is_mutating(&Route::AddFeature { id: "a".into() }));
        assert!(is_mutating(&Route::ApplyDeployment { id: "a".into() }));
    }

    #[test]
    fn api_config_from_env_overrides_bind_and_port() {
        // Use a unique environment override and ensure
        // the parser picks it up.
        std::env::set_var("FORGE_API_BIND", "127.0.0.1");
        std::env::set_var("FORGE_API_PORT", "9999");
        let cfg = ApiConfig::from_env();
        assert_eq!(cfg.bind, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(cfg.port, 9999);
        std::env::remove_var("FORGE_API_BIND");
        std::env::remove_var("FORGE_API_PORT");
    }

    #[test]
    fn synthetic_project_constant_is_stable() {
        assert_eq!(API_SYNTHETIC_PROJECT, "__api__");
    }

    #[test]
    fn request_hash_is_deterministic_per_request() {
        let raw = b"POST /v1/projects/rust-web/features HTTP/1.1\r\nContent-Length: 17\r\n\r\n{\"feature\":\"x\"}";
        let request = parse_request(raw, None, MAX_BODY_BYTES).unwrap();
        let first = request_hash(&request);
        let second = request_hash(&request);
        assert_eq!(first, second);
        let mut other = request.clone();
        other.method = "GET".to_string();
        assert_ne!(first, request_hash(&other));
    }
}
