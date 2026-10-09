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
