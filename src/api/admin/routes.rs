//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

pub(super) const COOKIE: &str = "forge_admin_session";

/// The `forge feature add` authoring command exposed as a session-gated,
/// preview + confirm/digest-bound admin route. Exported so the command catalog
/// can name the exact path the router registers, keeping the two in lockstep.
pub const ROUTE_ADMIN_FEATURE: &str = "POST /v1/admin/projects/{id}/feature";

/// The `forge spec generate` authoring command exposed as a session-gated,
/// preview + confirm/digest-bound admin route.
pub const ROUTE_ADMIN_SPEC: &str = "POST /v1/admin/projects/{id}/spec";

/// The `forge feature remove` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `remove_feature` Core handler the CLI runs. Exported so the command
/// catalog names the exact path the router registers.
pub const ROUTE_ADMIN_FEATURE_REMOVE: &str = "POST /v1/admin/projects/{id}/feature/remove";

/// The `forge feature upgrade` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `upgrade_feature` Core handler the CLI runs.
pub const ROUTE_ADMIN_FEATURE_UPGRADE: &str = "POST /v1/admin/projects/{id}/feature/upgrade";

/// The `forge spec apply` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `apply_routing` Core handler the CLI runs.
pub const ROUTE_ADMIN_SPEC_APPLY: &str = "POST /v1/admin/projects/{id}/spec/apply";

/// The read-only deploy plan, exposed as a session-gated admin route that
/// renders `deploy::engine::prepare_deploy` for the server-default target. It
/// invokes no adapter, writes nothing and returns a path-free plan view.
/// Exported so the command catalog names the exact path the router registers.
pub const ROUTE_ADMIN_DEPLOY_PLAN: &str = "GET /v1/admin/projects/{id}/deploy/plan";

/// The `forge deploy` apply, exposed as a session-gated, preview +
/// confirm/digest-bound admin route delegating to the same
/// `deploy::engine::apply_deploy` the bearer `/v1` route and CLI run. The
/// descriptor binds the project id and the server-resolved target — never a
/// path, argv or shell. Exported so the command catalog names the exact path.
pub const ROUTE_ADMIN_DEPLOY: &str = "POST /v1/admin/projects/{id}/deploy";

/// The read-only release plan, exposed as a session-gated admin route that
/// renders `release::engine::prepare_release` for one typed semver version. It
/// invokes no adapter, writes nothing, mutates no git state and returns a
/// path-free plan view plus the confirm digest. Exported so the command catalog
/// names the exact path the router registers.
pub const ROUTE_ADMIN_RELEASE_PLAN: &str = "GET /v1/admin/projects/{id}/release/plan";

/// The `forge release` apply, exposed as a session-gated, preview +
/// confirm/digest-bound admin route delegating to the same
/// `release::engine::apply_release` the CLI runs, using the manifest's stages.
/// The descriptor binds the project id and the normalized semver version —
/// never a stage list, path, argv, remote or shell. Exported so the command
/// catalog names the exact path the router registers.
pub const ROUTE_ADMIN_RELEASE: &str = "POST /v1/admin/projects/{id}/release";

/// The read-only provider-publish plan, exposed as a session-gated admin route
/// that resolves the provider id, the provider configuration and the committed
/// git revision server-side. It invokes no provider, writes nothing and returns
/// a path-free plan view plus the confirm digest. Exported so the command
/// catalog names the exact path the router registers.
pub const ROUTE_ADMIN_PUBLISH_PLAN: &str = "GET /v1/admin/projects/{id}/publish/plan";

/// The `forge publish` apply, exposed as a session-gated, preview +
/// confirm/digest-bound admin route delegating to the same
/// `publish::providers::invoke_provider` the CLI and the GitHub-push handler
/// run. The descriptor binds the project id, the server-resolved provider id
/// and the committed revision — never a path, binary, argv, host, SSH target or
/// credential. Exported so the command catalog names the exact path.
pub const ROUTE_ADMIN_PUBLISH: &str = "POST /v1/admin/projects/{id}/publish";

/// The read-only project-delivery status, exposed as a session-gated admin
/// route that reuses `delivery::handlers::run_status`. It invokes no provider
/// or adapter, writes nothing and returns the path-free projection plus the
/// next eligible staged confirmation. Exported so the command catalog names
/// the exact path the router registers.
pub const ROUTE_ADMIN_DELIVERY_STATUS: &str = "GET /v1/admin/projects/{id}/delivery/status";

/// The `forge delivery preflight` operation, exposed as a session-gated,
/// preview + confirm/digest-bound admin route. The descriptor binds the
/// project id and server-resolved registered revision; the browser supplies
/// no provider, path or credential. Exported so the command catalog names
/// the exact path.
pub const ROUTE_ADMIN_DELIVERY_PREFLIGHT: &str = "POST /v1/admin/projects/{id}/delivery/preflight";

/// The `forge delivery stage` operation, exposed as a session-gated, preview
/// + confirm/digest-bound admin route. The descriptor binds the project id,
/// server-resolved revision and canonical operation id; Core independently
/// verifies that the operation is a healthy same-revision preflight.
pub const ROUTE_ADMIN_DELIVERY_STAGE: &str = "POST /v1/admin/projects/{id}/delivery/stage";

/// The `forge delivery promote` operation, exposed as a session-gated,
/// preview + confirm/digest-bound admin route. The descriptor binds the
/// project id, server-resolved revision and supplied revision; Core
/// independently requires a healthy same-revision stage row.
pub const ROUTE_ADMIN_DELIVERY_PROMOTE: &str = "POST /v1/admin/projects/{id}/delivery/promote";

/// The `forge delivery hermora-retry` operation, exposed as a session-gated,
/// preview + confirm/digest-bound admin route. The browser supplies only an
/// HTTP(S) deployment URL and an environment-variable secret reference; Core
/// independently requires a healthy production row and validates the adapter
/// envelope without republishing.
pub const ROUTE_ADMIN_DELIVERY_HERMORA_RETRY: &str =
    "POST /v1/admin/projects/{id}/delivery/hermora-retry";

/// The server environment variable that selects the publish provider, matching
/// the CLI (`FORGE_PUBLISH_PROVIDER`).
pub(super) const PUBLISH_PROVIDER_ENV: &str = "FORGE_PUBLISH_PROVIDER";

/// The server environment variable that overrides the provider configuration
/// path. When unset the project's `.forge/providers.yaml` is used.
pub(super) const PUBLISH_PROVIDER_CONFIG_ENV: &str = "FORGE_PUBLISH_PROVIDER_CONFIG";

/// Read-only project-catalog browser (`web-project-catalog-browser`).
/// All six routes are session-gated GETs reusing the existing Core
/// catalog/gaps/fleet services — no write, no provider, no shell.
/// Exported so the command catalog names the exact paths the router
/// registers, keeping the two in lockstep.
pub const ROUTE_ADMIN_CATALOG: &str = "GET /v1/admin/projects/catalog";

pub const ROUTE_ADMIN_CATALOG_TAGS: &str = "GET /v1/admin/projects/catalog/tags";

pub const ROUTE_ADMIN_CATALOG_LANGUAGES: &str = "GET /v1/admin/projects/catalog/languages";

pub const ROUTE_ADMIN_CATALOG_GAPS: &str = "GET /v1/admin/projects/catalog/gaps";

pub const ROUTE_ADMIN_CATALOG_INSPECT: &str = "GET /v1/admin/projects/{id}/catalog";

pub const ROUTE_ADMIN_FLEET_INSPECT: &str = "GET /v1/admin/fleet/{entry}";

/// Read-only release history (`web-release-deploy-history`, audit gap 4).
/// All five routes are session-gated GETs reusing the existing Core
/// release/deploy stores and the registry journal — no write, no
/// provider, no adapter, no shell. Exported so the command catalog
/// names the exact paths the router registers, keeping the two in
/// lockstep.
pub const ROUTE_ADMIN_RELEASE_HISTORY: &str = "GET /v1/admin/projects/{id}/releases";

pub const ROUTE_ADMIN_RELEASE_INSPECT: &str = "GET /v1/admin/projects/{id}/releases/{release_id}";

pub const ROUTE_ADMIN_DEPLOY_HISTORY: &str = "GET /v1/admin/projects/{id}/deploys";

pub const ROUTE_ADMIN_DEPLOY_INSPECT: &str = "GET /v1/admin/projects/{id}/deploys/{deploy_id}";

pub const ROUTE_ADMIN_DEPLOY_STATUS: &str = "GET /v1/admin/projects/{id}/deploy/status";

/// Read-only agent sessions (`web-agent-identity-readonly`, audit gap 5).
/// Both routes are session-gated GETs over the existing Core agent
/// store (`.forge/agents/`): the recorded session list and one
/// recorded session. No write, no adapter subprocess, no provider,
/// no shell. Exported so the command catalog names the exact paths
/// the router registers, keeping the two in lockstep.
pub const ROUTE_ADMIN_AGENT_LIST: &str = "GET /v1/admin/projects/{id}/agents";

pub const ROUTE_ADMIN_AGENT_STATUS: &str = "GET /v1/admin/projects/{id}/agents/{session_id}";

/// Read-only identity sessions + config validation
/// (`web-agent-identity-readonly`, audit gap 5). All three routes are
/// session-gated GETs over the existing Core identity store
/// (`.forge/identity/`) and the manifest's `identity:` block: the
/// persisted session list, one persisted session, and the validated
/// identity configuration. No write, no challenge, no mint, no
/// provider contact, no secret material. Exported so the command
/// catalog names the exact paths the router registers, keeping the
/// two in lockstep.
pub const ROUTE_ADMIN_IDENTITY_CONFIG: &str = "GET /v1/admin/projects/{id}/identity/config";

pub const ROUTE_ADMIN_IDENTITY_SESSIONS: &str = "GET /v1/admin/projects/{id}/identity/sessions";

pub const ROUTE_ADMIN_IDENTITY_SESSION_INSPECT: &str =
    "GET /v1/admin/projects/{id}/identity/sessions/{session_id}";

/// Read-only creation-catalog browser (`web-creation-catalog-browser`,
/// audit gap 6). All twenty routes are session-gated GETs reusing the
/// existing Core creation registries — no write, no provider, no
/// adapter, no toolchain probe, no shell, no journal row. Pure-catalog
/// routes carry no project id; the three project-bound reads resolve
/// the directory server-side from a validated id. Exported so the
/// command catalog names the exact paths the router registers,
/// keeping the two in lockstep.
pub const ROUTE_ADMIN_CREATION_PROFILES: &str = "GET /v1/admin/creation/profiles";
pub const ROUTE_ADMIN_CREATION_PROFILE: &str = "GET /v1/admin/creation/profiles/{id}";
pub const ROUTE_ADMIN_CREATION_PROFILE_RESOLVE: &str = "GET /v1/admin/creation/profiles/resolve";
pub const ROUTE_ADMIN_CREATION_FEATURES: &str = "GET /v1/admin/creation/features";
pub const ROUTE_ADMIN_CREATION_FEATURE: &str = "GET /v1/admin/creation/features/{id}";
pub const ROUTE_ADMIN_CREATION_FEATURE_RESOLVE: &str = "GET /v1/admin/creation/features/resolve";
pub const ROUTE_ADMIN_CREATION_COMPONENTS: &str = "GET /v1/admin/creation/components";
pub const ROUTE_ADMIN_CREATION_COMPONENT: &str = "GET /v1/admin/creation/components/{id}";
pub const ROUTE_ADMIN_CREATION_COMPONENT_RESOLVE: &str =
    "GET /v1/admin/creation/components/resolve";
pub const ROUTE_ADMIN_CREATION_UI_PATTERNS: &str = "GET /v1/admin/creation/ui-patterns";
pub const ROUTE_ADMIN_CREATION_UI_PATTERN: &str = "GET /v1/admin/creation/ui-patterns/{id}";
pub const ROUTE_ADMIN_CREATION_UI_PATTERN_RESOLVE: &str =
    "GET /v1/admin/creation/ui-patterns/resolve";
pub const ROUTE_ADMIN_CREATION_STANDARDS: &str = "GET /v1/admin/creation/standards";
pub const ROUTE_ADMIN_CREATION_STANDARD: &str = "GET /v1/admin/creation/standards/{id}";
pub const ROUTE_ADMIN_CREATION_PROCEDURES: &str = "GET /v1/admin/creation/procedures";
pub const ROUTE_ADMIN_CREATION_PROCEDURE: &str = "GET /v1/admin/creation/procedures/{id}";
pub const ROUTE_ADMIN_CREATION_INTENT_VALIDATE: &str = "GET /v1/admin/creation/intents/validate";
pub const ROUTE_ADMIN_CREATION_STANDARD_CHECK: &str = "GET /v1/admin/projects/{id}/standard/check";
pub const ROUTE_ADMIN_CREATION_STANDARD_DIFF: &str = "GET /v1/admin/projects/{id}/standard/diff";
pub const ROUTE_ADMIN_CREATION_INTENT_PLANS: &str = "GET /v1/admin/projects/{id}/intent/plans";

/// Read-only assurance browser (`web-assurance-browser`, audit gap 7).
/// All seventeen routes are session-gated GETs reusing the existing Core
/// stores — no write, no provider probe, no adapter run, no shell, no
/// journal row, no browser-supplied path. Pure contract routes carry no
/// project id; the fifteen project-bound reads resolve the directory
/// server-side from a validated id. Exported so the command catalog
/// names the exact paths the router registers, keeping the two in
/// lockstep.
pub const ROUTE_ADMIN_CONTRACTS: &str = "GET /v1/admin/contracts";
pub const ROUTE_ADMIN_CONTRACT_INSPECT: &str = "GET /v1/admin/contracts/{family}";
pub const ROUTE_ADMIN_SPECS: &str = "GET /v1/admin/projects/{id}/specs";
pub const ROUTE_ADMIN_SPEC_INSPECT: &str = "GET /v1/admin/projects/{id}/specs/{spec}";
pub const ROUTE_ADMIN_SPEC_ROUTE: &str = "GET /v1/admin/projects/{id}/spec/route";
pub const ROUTE_ADMIN_REMEDIATE_SCAN: &str = "GET /v1/admin/projects/{id}/remediate/scan";
pub const ROUTE_ADMIN_REMEDIATE_DIFF: &str = "GET /v1/admin/projects/{id}/remediate/diff";
pub const ROUTE_ADMIN_DESCRIBE_LIST: &str = "GET /v1/admin/projects/{id}/describe/proposals";
pub const ROUTE_ADMIN_DESCRIBE_SHOW: &str =
    "GET /v1/admin/projects/{id}/describe/proposals/{proposal}";
pub const ROUTE_ADMIN_CLASSIFY_LIST: &str = "GET /v1/admin/projects/{id}/classify/proposals";
pub const ROUTE_ADMIN_CLASSIFY_SHOW: &str =
    "GET /v1/admin/projects/{id}/classify/proposals/{proposal}";
pub const ROUTE_ADMIN_CONTRACT_EMIT: &str = "GET /v1/admin/projects/{id}/contracts/emit";
pub const ROUTE_ADMIN_GOVERNANCE_LIST: &str = "GET /v1/admin/projects/{id}/governance";
pub const ROUTE_ADMIN_GOVERNANCE_STATUS: &str = "GET /v1/admin/projects/{id}/governance/status";
pub const ROUTE_ADMIN_GOVERNANCE_INSPECT: &str = "GET /v1/admin/projects/{id}/governance/inspect";
pub const ROUTE_ADMIN_ANALYTICS_METRICS: &str = "GET /v1/admin/projects/{id}/analytics/metrics";
pub const ROUTE_ADMIN_STUDIO_PREVIEW: &str = "GET /v1/admin/projects/{id}/studio/preview";
