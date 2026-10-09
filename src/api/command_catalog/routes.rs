//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::Category;

/// Separately versioned catalog contract. Stable IDs are additive; removing
/// or renaming an ID requires a contract migration.
pub const CONTRACT_VERSION: &str = "forge-command-catalog/0.1.0";

/// The implemented typed JSON route web rows may resolve to. The fleet view
/// of the standalone frontend renders the registry rows, workspace-registry
/// source health and inventory classification from this one endpoint.
pub const WEB_ROUTE_PROJECTS: &str = "GET /v1/admin/projects";

/// Workbench typed routes (`forge-project-workbench/0.1.0`). These reference
/// the workbench module's own route constants so the catalog and the live
/// endpoints can never name different paths: `inspect`/`doctor` resolve to the
/// project detail projection (health is embedded there) and `upgrade` resolves
/// to the side-effect-free plan endpoint whose confirm-gated continuation is
/// the apply route.
pub(super) const WEB_ROUTE_PROJECT_DETAIL: &str = super::super::workbench::ROUTE_PROJECT_DETAIL;

pub(super) const WEB_ROUTE_PROJECT_PLAN: &str = super::super::workbench::ROUTE_PROJECT_PLAN;

pub(super) const WEB_ROUTE_PROJECT_APPLY: &str = super::super::workbench::ROUTE_PROJECT_APPLY;

/// Authoring-command typed routes (`forge-web-command-execution`). These name
/// the exact admin paths the router registers so the catalog and the live
/// endpoints can never diverge: `forge feature add` and `forge spec generate`
/// as session-gated, confirm/digest-bound browser-executable actions.
pub(super) const WEB_ROUTE_ADMIN_FEATURE: &str = super::super::admin::ROUTE_ADMIN_FEATURE;

pub(super) const WEB_ROUTE_ADMIN_SPEC: &str = super::super::admin::ROUTE_ADMIN_SPEC;

pub(super) const WEB_ROUTE_ADMIN_FEATURE_REMOVE: &str =
    super::super::admin::ROUTE_ADMIN_FEATURE_REMOVE;

pub(super) const WEB_ROUTE_ADMIN_FEATURE_UPGRADE: &str =
    super::super::admin::ROUTE_ADMIN_FEATURE_UPGRADE;

pub(super) const WEB_ROUTE_ADMIN_SPEC_APPLY: &str = super::super::admin::ROUTE_ADMIN_SPEC_APPLY;

/// Deploy-command typed routes (`forge-web-project-deployment`). These name the
/// exact admin paths the router registers so the catalog and the live endpoints
/// can never diverge: the read-only deploy plan and the session-gated,
/// confirm/digest-bound deploy apply delegating to `deploy::engine`.
pub(super) const WEB_ROUTE_ADMIN_DEPLOY_PLAN: &str = super::super::admin::ROUTE_ADMIN_DEPLOY_PLAN;

pub(super) const WEB_ROUTE_ADMIN_DEPLOY: &str = super::super::admin::ROUTE_ADMIN_DEPLOY;

/// Release-command typed routes (`forge-web-project-release`). These name the
/// exact admin paths the router registers so the catalog and the live endpoints
/// can never diverge: the read-only release plan and the session-gated,
/// confirm/digest-bound release apply delegating to `release::engine`.
pub(super) const WEB_ROUTE_ADMIN_RELEASE_PLAN: &str = super::super::admin::ROUTE_ADMIN_RELEASE_PLAN;

pub(super) const WEB_ROUTE_ADMIN_RELEASE: &str = super::super::admin::ROUTE_ADMIN_RELEASE;

/// Publish-command typed routes (`forge-web-project-publish`). These name the
/// exact admin paths the router registers so the catalog and the live endpoints
/// can never diverge: the read-only publish plan and the session-gated,
/// confirm/digest-bound publish apply delegating to `publish::providers`. The
/// provider, project and revision are resolved server-side, so the executable
/// row carries no browser-supplied parameters.
pub(super) const WEB_ROUTE_ADMIN_PUBLISH_PLAN: &str = super::super::admin::ROUTE_ADMIN_PUBLISH_PLAN;

pub(super) const WEB_ROUTE_ADMIN_PUBLISH: &str = super::super::admin::ROUTE_ADMIN_PUBLISH;

/// Project-delivery typed routes (`forge-web-project-delivery`). These name
/// the exact admin paths the router registers so the catalog and the live
/// endpoints can never diverge: read-only delivery status plus the four
/// confirm/digest-bound staged mutations delegating to `delivery::handlers`.
pub(super) const WEB_ROUTE_ADMIN_DELIVERY_STATUS: &str =
    super::super::admin::ROUTE_ADMIN_DELIVERY_STATUS;

pub(super) const WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT: &str =
    super::super::admin::ROUTE_ADMIN_DELIVERY_PREFLIGHT;

pub(super) const WEB_ROUTE_ADMIN_DELIVERY_STAGE: &str =
    super::super::admin::ROUTE_ADMIN_DELIVERY_STAGE;

pub(super) const WEB_ROUTE_ADMIN_DELIVERY_PROMOTE: &str =
    super::super::admin::ROUTE_ADMIN_DELIVERY_PROMOTE;

pub(super) const WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY: &str =
    super::super::admin::ROUTE_ADMIN_DELIVERY_HERMORA_RETRY;

/// Read-only status typed routes (`forge-project-status/0.1.0`). These
/// reference the status module's own route constants so the catalog and the
/// live endpoints can never name different paths: the per-project status
/// projection and the fleet readiness summary.
pub(super) const WEB_ROUTE_PROJECT_STATUS: &str = super::super::status::ROUTE_PROJECT_STATUS;

pub(super) const WEB_ROUTE_FLEET_STATUS: &str = super::super::status::ROUTE_FLEET_STATUS;

/// Maintainer-surface typed routes (`forge-project-maintain/0.1.0`). These
/// reference the maintain module's own route constants so the catalog and
/// the live endpoints can never name different paths: the read-only
/// per-project maintainer projection plus the three preview +
/// confirm/digest-bound classification decision routes the browser's
/// Maintain card drives through `buildActionControl`.
pub(super) const WEB_ROUTE_PROJECT_MAINTAIN: &str = super::super::maintain::ROUTE_PROJECT_MAINTAIN;

pub(super) const WEB_ROUTE_CLASSIFY_APPROVE: &str = super::super::maintain::ROUTE_CLASSIFY_APPROVE;

pub(super) const WEB_ROUTE_CLASSIFY_REJECT: &str = super::super::maintain::ROUTE_CLASSIFY_REJECT;

pub(super) const WEB_ROUTE_CLASSIFY_APPLY: &str = super::super::maintain::ROUTE_CLASSIFY_APPLY;

/// Project-management typed routes (`forge-web-project-management`). These name
/// the exact admin paths the router registers so the catalog and the live
/// endpoints can never diverge: `forge new`, `forge import` and
/// `forge register` as session-gated, confirm/digest-bound browser-executable
/// creations that resolve the destination only from a server-side root.
pub(super) const WEB_ROUTE_ADMIN_PROJECT_NEW: &str =
    super::super::project_management::ROUTE_ADMIN_PROJECT_NEW;

pub(super) const WEB_ROUTE_ADMIN_PROJECT_IMPORT: &str =
    super::super::project_management::ROUTE_ADMIN_PROJECT_IMPORT;

pub(super) const WEB_ROUTE_ADMIN_PROJECT_REGISTER: &str =
    super::super::project_management::ROUTE_ADMIN_PROJECT_REGISTER;

/// Workspace-onboarding typed routes (`forge-web-workspace-onboarding`).
/// Web-only workflows with no CLI row: live candidate discovery and bulk
/// confirm/digest-bound onboarding under the configured project root.
pub(super) const WEB_ROUTE_ADMIN_WORKSPACE_CANDIDATES: &str =
    super::super::workspace::ROUTE_ADMIN_WORKSPACE_CANDIDATES;

pub(super) const WEB_ROUTE_ADMIN_WORKSPACE_ONBOARD: &str =
    super::super::workspace::ROUTE_ADMIN_WORKSPACE_ONBOARD;

/// Delivery-control typed routes (`forge-web-delivery-controls/0.1.0`).
/// These reference the delivery module's own route constants so the
/// catalog and the live endpoints can never name different paths: the
/// share allowlist, manifest preview, digest-bound approval, publication,
/// reconciliation and operation-status surfaces.
pub(super) const WEB_ROUTE_DELIVERY_OVERVIEW: &str =
    super::super::delivery::ROUTE_DELIVERY_OVERVIEW;

pub(super) const WEB_ROUTE_DELIVERY_PREVIEW: &str = super::super::delivery::ROUTE_DELIVERY_PREVIEW;

pub(super) const WEB_ROUTE_DELIVERY_ALLOWLIST: &str =
    super::super::delivery::ROUTE_DELIVERY_ALLOWLIST;

pub(super) const WEB_ROUTE_DELIVERY_ALLOWLIST_REMOVE: &str =
    super::super::delivery::ROUTE_DELIVERY_ALLOWLIST_REMOVE;

pub(super) const WEB_ROUTE_DELIVERY_APPROVE: &str = super::super::delivery::ROUTE_DELIVERY_APPROVE;

pub(super) const WEB_ROUTE_DELIVERY_PUBLISH: &str = super::super::delivery::ROUTE_DELIVERY_PUBLISH;

pub(super) const WEB_ROUTE_DELIVERY_RECONCILE: &str =
    super::super::delivery::ROUTE_DELIVERY_RECONCILE;

/// Routes a `web` row may honestly point at today. A row naming any other
/// route is a catalog bug and is reported by [`problems`].
pub(super) const IMPLEMENTED_WEB_ROUTES: &[&str] = &[
    WEB_ROUTE_PROJECTS,
    WEB_ROUTE_PROJECT_DETAIL,
    WEB_ROUTE_PROJECT_PLAN,
    WEB_ROUTE_PROJECT_APPLY,
    WEB_ROUTE_ADMIN_FEATURE,
    WEB_ROUTE_ADMIN_SPEC,
    WEB_ROUTE_ADMIN_FEATURE_REMOVE,
    WEB_ROUTE_ADMIN_FEATURE_UPGRADE,
    WEB_ROUTE_ADMIN_SPEC_APPLY,
    WEB_ROUTE_ADMIN_DEPLOY_PLAN,
    WEB_ROUTE_ADMIN_DEPLOY,
    WEB_ROUTE_ADMIN_RELEASE_PLAN,
    WEB_ROUTE_ADMIN_RELEASE,
    WEB_ROUTE_ADMIN_PUBLISH_PLAN,
    WEB_ROUTE_ADMIN_PUBLISH,
    WEB_ROUTE_ADMIN_DELIVERY_STATUS,
    WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT,
    WEB_ROUTE_ADMIN_DELIVERY_STAGE,
    WEB_ROUTE_ADMIN_DELIVERY_PROMOTE,
    WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY,
    WEB_ROUTE_PROJECT_STATUS,
    WEB_ROUTE_FLEET_STATUS,
    WEB_ROUTE_PROJECT_MAINTAIN,
    WEB_ROUTE_CLASSIFY_APPROVE,
    WEB_ROUTE_CLASSIFY_REJECT,
    WEB_ROUTE_CLASSIFY_APPLY,
    WEB_ROUTE_ADMIN_PROJECT_NEW,
    WEB_ROUTE_ADMIN_PROJECT_IMPORT,
    WEB_ROUTE_ADMIN_PROJECT_REGISTER,
    WEB_ROUTE_ADMIN_WORKSPACE_CANDIDATES,
    WEB_ROUTE_ADMIN_WORKSPACE_ONBOARD,
    WEB_ROUTE_DELIVERY_OVERVIEW,
    WEB_ROUTE_DELIVERY_PREVIEW,
    WEB_ROUTE_DELIVERY_ALLOWLIST,
    WEB_ROUTE_DELIVERY_ALLOWLIST_REMOVE,
    WEB_ROUTE_DELIVERY_APPROVE,
    WEB_ROUTE_DELIVERY_PUBLISH,
    WEB_ROUTE_DELIVERY_RECONCILE,
];

/// Plain-language reasons shared by rows in the same family. Each one names
/// why browser execution is not available and the operator's next step.
pub(super) const REASON_GROUP: &str = "Command groups only organize their subcommands; browse the subcommands listed under this group here, or run the group with --help in a terminal.";

pub(super) const REASON_HELP: &str = "Terminal context help is replaced in the browser by this catalog view: every command is listed here with its state. Run `forge help` in a terminal for the same tree as text.";

pub(super) const REASON_TRANSPORT: &str = "This command starts a long-running server process bound to a loopback listener; it is a transport, not a browser workflow. Next step: run it in a terminal and open the page or API it serves.";

pub(super) const REASON_LOCAL_FS: &str = "This command reads or writes the local project tree directly; a browser cannot safely touch the filesystem and no typed JSON route for it exists yet. Next step: run it in a terminal inside the project.";

pub(super) const REASON_FILE_STDIN: &str = "This command consumes a local file path or stdin document; the browser form for it would be a file upload the API does not accept. Next step: run it in a terminal.";

pub(super) const REASON_NATIVE: &str = "This command invokes native build/test toolchains (compilers, package managers, generated fixture matrices) and streams their output. Next step: run it in a terminal with the toolchain installed.";

pub(super) const REASON_GIT: &str = "This command performs a local Git write (stage, commit, push, mirror distribution) with explicit interactive confirmation flags; the browser has no Git surface and the confirmation must stay human-initiated. Next step: run it in a terminal.";

pub(super) const REASON_PROVIDER: &str = "This command requires a configured external provider (Jenkins/Mac, analytics, translation, GitHub) whose credentials live in the terminal environment, never in the browser. Next step: configure the provider via `forge publish provider` or the environment and run it in a terminal.";

pub(super) const REASON_PROJECT_CAPABILITY: &str = "This command requires the project manifest to declare a deployment target and adapter; nothing can run until that capability exists for the project. Next step: declare deployment in the project, then run it in a terminal.";

pub(super) const REASON_TTY_HIDDEN: &str = "This command reads hidden terminal input (password without echo) or pastes manual provider-callback values; neither may ever cross a browser form or the JSON API. Next step: run it in a terminal.";

pub(super) const REASON_LOCAL_SECRET: &str = "This command prints a freshly generated secret to the terminal for the operator to copy; routing that value through a browser form or the JSON API would expose it. Next step: run it in a terminal.";

pub(super) const REASON_LEGACY_HTML: &str = "The legacy portal renders server-side HTML sections; the standalone frontend (this page) replaced that surface for browsers. Next step: use this dashboard, or run it in a terminal for the HTML view.";

pub(super) const REASON_AGENT: &str = "This command drives local agent adapter processes against a project directory (start, pause, takeover, resume, restart, new sessions); these are terminal-session operations with no typed JSON route. Next step: run it in a terminal.";

pub(super) const REASON_LOCAL_TOOLCHAIN: &str = "This command shells out to a locally installed developer CLI (gh) and its credential store; the browser cannot reach that installation. Next step: install and authenticate the CLI, then run it in a terminal.";

pub(super) const REASON_NOT_YET_WEB: &str = "A web workflow for this command is planned in the staged web packages (workbench, portfolio controls, delivery controls) but not implemented yet; it stays visible as a tracked gap, never labeled supported. Next step: use the CLI for now.";

pub(super) const ALL_CATEGORIES: [Category; 9] = [
    Category::Registry,
    Category::Creation,
    Category::Quality,
    Category::Release,
    Category::Delivery,
    Category::Portfolio,
    Category::Identity,
    Category::Transports,
    Category::Reference,
];
