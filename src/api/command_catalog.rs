//! forge — typed CLI command-catalog metadata (`forge-command-catalog/0.1.0`).
//!
//! The catalog is the browser-facing inventory of every Rust CLI command in
//! [`crate::Cli`]: one row per Clap path (top-level and nested), each mapped
//! to a truthful availability state. It is metadata only — the API serializes
//! descriptions, never executes commands, and exposes no shell/eval route.
//! The parity test inside `src/main.rs` walks the real Clap tree and fails
//! when this catalog and the CLI drift apart.
//!
//! Availability vocabulary (design.md): `web`, `cli_only`,
//! `provider_required`, `project_capability_required`, `disabled`,
//! `not_yet_web`. Every non-web row carries a plain-language reason and next
//! step; `web` rows must resolve to an implemented typed JSON route. Risk
//! labels are guidance for reading, not a permission grant: authorization is
//! enforced by each invoked operation.

use serde::Serialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::OnceLock;

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
const WEB_ROUTE_PROJECT_DETAIL: &str = super::workbench::ROUTE_PROJECT_DETAIL;
const WEB_ROUTE_PROJECT_PLAN: &str = super::workbench::ROUTE_PROJECT_PLAN;
const WEB_ROUTE_PROJECT_APPLY: &str = super::workbench::ROUTE_PROJECT_APPLY;

/// Authoring-command typed routes (`forge-web-command-execution`). These name
/// the exact admin paths the router registers so the catalog and the live
/// endpoints can never diverge: `forge feature add` and `forge spec generate`
/// as session-gated, confirm/digest-bound browser-executable actions.
const WEB_ROUTE_ADMIN_FEATURE: &str = super::admin::ROUTE_ADMIN_FEATURE;
const WEB_ROUTE_ADMIN_SPEC: &str = super::admin::ROUTE_ADMIN_SPEC;
const WEB_ROUTE_ADMIN_FEATURE_REMOVE: &str = super::admin::ROUTE_ADMIN_FEATURE_REMOVE;
const WEB_ROUTE_ADMIN_FEATURE_UPGRADE: &str = super::admin::ROUTE_ADMIN_FEATURE_UPGRADE;
const WEB_ROUTE_ADMIN_SPEC_APPLY: &str = super::admin::ROUTE_ADMIN_SPEC_APPLY;

/// Deploy-command typed routes (`forge-web-project-deployment`). These name the
/// exact admin paths the router registers so the catalog and the live endpoints
/// can never diverge: the read-only deploy plan and the session-gated,
/// confirm/digest-bound deploy apply delegating to `deploy::engine`.
const WEB_ROUTE_ADMIN_DEPLOY_PLAN: &str = super::admin::ROUTE_ADMIN_DEPLOY_PLAN;
const WEB_ROUTE_ADMIN_DEPLOY: &str = super::admin::ROUTE_ADMIN_DEPLOY;

/// Release-command typed routes (`forge-web-project-release`). These name the
/// exact admin paths the router registers so the catalog and the live endpoints
/// can never diverge: the read-only release plan and the session-gated,
/// confirm/digest-bound release apply delegating to `release::engine`.
const WEB_ROUTE_ADMIN_RELEASE_PLAN: &str = super::admin::ROUTE_ADMIN_RELEASE_PLAN;
const WEB_ROUTE_ADMIN_RELEASE: &str = super::admin::ROUTE_ADMIN_RELEASE;

/// Publish-command typed routes (`forge-web-project-publish`). These name the
/// exact admin paths the router registers so the catalog and the live endpoints
/// can never diverge: the read-only publish plan and the session-gated,
/// confirm/digest-bound publish apply delegating to `publish::providers`. The
/// provider, project and revision are resolved server-side, so the executable
/// row carries no browser-supplied parameters.
const WEB_ROUTE_ADMIN_PUBLISH_PLAN: &str = super::admin::ROUTE_ADMIN_PUBLISH_PLAN;
const WEB_ROUTE_ADMIN_PUBLISH: &str = super::admin::ROUTE_ADMIN_PUBLISH;

/// Project-delivery typed routes (`forge-web-project-delivery`). These name
/// the exact admin paths the router registers so the catalog and the live
/// endpoints can never diverge: read-only delivery status plus the four
/// confirm/digest-bound staged mutations delegating to `delivery::handlers`.
const WEB_ROUTE_ADMIN_DELIVERY_STATUS: &str = super::admin::ROUTE_ADMIN_DELIVERY_STATUS;
const WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT: &str = super::admin::ROUTE_ADMIN_DELIVERY_PREFLIGHT;
const WEB_ROUTE_ADMIN_DELIVERY_STAGE: &str = super::admin::ROUTE_ADMIN_DELIVERY_STAGE;
const WEB_ROUTE_ADMIN_DELIVERY_PROMOTE: &str = super::admin::ROUTE_ADMIN_DELIVERY_PROMOTE;
const WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY: &str =
    super::admin::ROUTE_ADMIN_DELIVERY_HERMORA_RETRY;

/// Read-only status typed routes (`forge-project-status/0.1.0`). These
/// reference the status module's own route constants so the catalog and the
/// live endpoints can never name different paths: the per-project status
/// projection and the fleet readiness summary.
const WEB_ROUTE_PROJECT_STATUS: &str = super::status::ROUTE_PROJECT_STATUS;
const WEB_ROUTE_FLEET_STATUS: &str = super::status::ROUTE_FLEET_STATUS;

/// Project-management typed routes (`forge-web-project-management`). These name
/// the exact admin paths the router registers so the catalog and the live
/// endpoints can never diverge: `forge new`, `forge import` and
/// `forge register` as session-gated, confirm/digest-bound browser-executable
/// creations that resolve the destination only from a server-side root.
const WEB_ROUTE_ADMIN_PROJECT_NEW: &str = super::project_management::ROUTE_ADMIN_PROJECT_NEW;
const WEB_ROUTE_ADMIN_PROJECT_IMPORT: &str = super::project_management::ROUTE_ADMIN_PROJECT_IMPORT;
const WEB_ROUTE_ADMIN_PROJECT_REGISTER: &str =
    super::project_management::ROUTE_ADMIN_PROJECT_REGISTER;

/// Workspace-onboarding typed routes (`forge-web-workspace-onboarding`).
/// Web-only workflows with no CLI row: live candidate discovery and bulk
/// confirm/digest-bound onboarding under the configured project root.
const WEB_ROUTE_ADMIN_WORKSPACE_CANDIDATES: &str =
    super::workspace::ROUTE_ADMIN_WORKSPACE_CANDIDATES;
const WEB_ROUTE_ADMIN_WORKSPACE_ONBOARD: &str = super::workspace::ROUTE_ADMIN_WORKSPACE_ONBOARD;

/// Delivery-control typed routes (`forge-web-delivery-controls/0.1.0`).
/// These reference the delivery module's own route constants so the
/// catalog and the live endpoints can never name different paths: the
/// share allowlist, manifest preview, digest-bound approval, publication,
/// reconciliation and operation-status surfaces.
const WEB_ROUTE_DELIVERY_OVERVIEW: &str = super::delivery::ROUTE_DELIVERY_OVERVIEW;
const WEB_ROUTE_DELIVERY_PREVIEW: &str = super::delivery::ROUTE_DELIVERY_PREVIEW;
const WEB_ROUTE_DELIVERY_ALLOWLIST: &str = super::delivery::ROUTE_DELIVERY_ALLOWLIST;
const WEB_ROUTE_DELIVERY_ALLOWLIST_REMOVE: &str = super::delivery::ROUTE_DELIVERY_ALLOWLIST_REMOVE;
const WEB_ROUTE_DELIVERY_APPROVE: &str = super::delivery::ROUTE_DELIVERY_APPROVE;
const WEB_ROUTE_DELIVERY_PUBLISH: &str = super::delivery::ROUTE_DELIVERY_PUBLISH;
const WEB_ROUTE_DELIVERY_RECONCILE: &str = super::delivery::ROUTE_DELIVERY_RECONCILE;

/// Routes a `web` row may honestly point at today. A row naming any other
/// route is a catalog bug and is reported by [`problems`].
const IMPLEMENTED_WEB_ROUTES: &[&str] = &[
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
const REASON_GROUP: &str = "Command groups only organize their subcommands; browse the subcommands listed under this group here, or run the group with --help in a terminal.";
const REASON_HELP: &str = "Terminal context help is replaced in the browser by this catalog view: every command is listed here with its state. Run `forge help` in a terminal for the same tree as text.";
const REASON_TRANSPORT: &str = "This command starts a long-running server process bound to a loopback listener; it is a transport, not a browser workflow. Next step: run it in a terminal and open the page or API it serves.";
const REASON_LOCAL_FS: &str = "This command reads or writes the local project tree directly; a browser cannot safely touch the filesystem and no typed JSON route for it exists yet. Next step: run it in a terminal inside the project.";
const REASON_FILE_STDIN: &str = "This command consumes a local file path or stdin document; the browser form for it would be a file upload the API does not accept. Next step: run it in a terminal.";
const REASON_NATIVE: &str = "This command invokes native build/test toolchains (compilers, package managers, generated fixture matrices) and streams their output. Next step: run it in a terminal with the toolchain installed.";
const REASON_GIT: &str = "This command performs a local Git write (stage, commit, push, mirror distribution) with explicit interactive confirmation flags; the browser has no Git surface and the confirmation must stay human-initiated. Next step: run it in a terminal.";
const REASON_PROVIDER: &str = "This command requires a configured external provider (Jenkins/Mac, analytics, translation, GitHub) whose credentials live in the terminal environment, never in the browser. Next step: configure the provider via `forge publish provider` or the environment and run it in a terminal.";
const REASON_PROJECT_CAPABILITY: &str = "This command requires the project manifest to declare a deployment target and adapter; nothing can run until that capability exists for the project. Next step: declare deployment in the project, then run it in a terminal.";
const REASON_TTY_HIDDEN: &str = "This command reads hidden terminal input (password without echo) or pastes manual provider-callback values; neither may ever cross a browser form or the JSON API. Next step: run it in a terminal.";
const REASON_LOCAL_SECRET: &str = "This command prints a freshly generated secret to the terminal for the operator to copy; routing that value through a browser form or the JSON API would expose it. Next step: run it in a terminal.";
const REASON_LEGACY_HTML: &str = "The legacy portal renders server-side HTML sections; the standalone frontend (this page) replaced that surface for browsers. Next step: use this dashboard, or run it in a terminal for the HTML view.";
const REASON_AGENT: &str = "This command drives local agent adapter processes against a project directory (start, pause, takeover, resume, restart, new sessions); these are terminal-session operations with no typed JSON route. Next step: run it in a terminal.";
const REASON_LOCAL_TOOLCHAIN: &str = "This command shells out to a locally installed developer CLI (gh) and its credential store; the browser cannot reach that installation. Next step: install and authenticate the CLI, then run it in a terminal.";
const REASON_NOT_YET_WEB: &str = "A web workflow for this command is planned in the staged web packages (workbench, portfolio controls, delivery controls) but not implemented yet; it stays visible as a tracked gap, never labeled supported. Next step: use the CLI for now.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Registry,
    Creation,
    Quality,
    Release,
    Delivery,
    Portfolio,
    Identity,
    Transports,
    Reference,
}

impl Category {
    fn id(self) -> &'static str {
        match self {
            Category::Registry => "registry",
            Category::Creation => "creation",
            Category::Quality => "quality",
            Category::Release => "release",
            Category::Delivery => "delivery",
            Category::Portfolio => "portfolio",
            Category::Identity => "identity",
            Category::Transports => "transports",
            Category::Reference => "reference",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Category::Registry => "Registry",
            Category::Creation => "Creation",
            Category::Quality => "Quality",
            Category::Release => "Release",
            Category::Delivery => "Delivery",
            Category::Portfolio => "Portfolio",
            Category::Identity => "Identity",
            Category::Transports => "Transports",
            Category::Reference => "Reference",
        }
    }
}

const ALL_CATEGORIES: [Category; 9] = [
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Workspace,
    Project,
    Forge,
    Profile,
    Provider,
}

impl Scope {
    fn id(self) -> &'static str {
        match self {
            Scope::Workspace => "workspace",
            Scope::Project => "project",
            Scope::Forge => "forge",
            Scope::Profile => "profile",
            Scope::Provider => "provider",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    Read,
    LocalWrite,
    RemoteWrite,
    SessionAdmin,
}

impl Risk {
    fn id(self) -> &'static str {
        match self {
            Risk::Read => "read",
            Risk::LocalWrite => "local_write",
            Risk::RemoteWrite => "remote_write",
            Risk::SessionAdmin => "session_admin",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Web,
    CliOnly,
    ProviderRequired,
    ProjectCapabilityRequired,
    Disabled,
    NotYetWeb,
}

impl Availability {
    fn id(self) -> &'static str {
        match self {
            Availability::Web => "web",
            Availability::CliOnly => "cli_only",
            Availability::ProviderRequired => "provider_required",
            Availability::ProjectCapabilityRequired => "project_capability_required",
            Availability::Disabled => "disabled",
            Availability::NotYetWeb => "not_yet_web",
        }
    }
}

/// One typed parameter an executable (`web`) row accepts: a `name`, a closed
/// scalar `kind` (`string`, `string_array` or `boolean`) and whether it is
/// `required`. The browser renders one control per parameter and never a
/// free-text shell/path/argv field, so this is the entire request surface of
/// the row's route — no field here is ever interpreted as a command or path.
#[derive(Clone, Debug, Serialize)]
pub struct ExecParameter {
    pub name: String,
    pub kind: &'static str,
    pub required: bool,
}

/// The machine-readable execution contract carried only by `web` rows so the
/// frontend can render a runnable, confirm-gated control directly from the
/// catalog with no bespoke per-command wiring. `route` is the exact admin
/// route the router registers (always one of [`IMPLEMENTED_WEB_ROUTES`]);
/// `method` is its HTTP verb; `risk` mirrors the row's risk label;
/// `confirm_required` and `digest_bound` are always true for these session-
/// gated mutating lifecycle actions (preview writes nothing; the confirmed
/// run must echo the matching `plan_digest`). Non-executable rows carry
/// `None`.
#[derive(Clone, Debug, Serialize)]
pub struct CommandExecution {
    pub route: &'static str,
    pub method: &'static str,
    pub risk: &'static str,
    pub confirm_required: bool,
    pub digest_bound: bool,
    pub parameters: Vec<ExecParameter>,
}

/// One catalog row: `{id,parent_id,label,summary,category,scope,risk,
/// availability,route,cli_invocation,reason,capabilities,execution}`.
#[derive(Clone, Debug, Serialize)]
pub struct CommandRow {
    pub id: String,
    pub parent_id: Option<String>,
    pub label: String,
    pub summary: &'static str,
    pub category: &'static str,
    pub scope: &'static str,
    pub risk: &'static str,
    pub availability: &'static str,
    pub route: Option<&'static str>,
    pub cli_invocation: String,
    pub reason: Option<&'static str>,
    pub capabilities: &'static [&'static str],
    /// Present (Some) only for `web` rows that are executable from the
    /// portal; every other row serializes this as `null`.
    pub execution: Option<CommandExecution>,
}

struct CatalogBuilder {
    rows: Vec<CommandRow>,
}

impl CatalogBuilder {
    fn new() -> Self {
        Self { rows: Vec::new() }
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        availability: Availability,
        route: Option<&'static str>,
        reason: Option<&'static str>,
        capabilities: &'static [&'static str],
    ) {
        let id = match parent {
            Some(prefix) => format!("{prefix}.{name}"),
            None => name.to_string(),
        };
        let cli_invocation = format!("forge {}", id.replace('.', " "));
        self.rows.push(CommandRow {
            label: name.to_string(),
            cli_invocation,
            id,
            parent_id: parent.map(str::to_string),
            summary,
            category: category.id(),
            scope: scope.id(),
            risk: risk.id(),
            availability: availability.id(),
            route,
            reason,
            capabilities,
            execution: None,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn leaf(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        availability: Availability,
        capabilities: &'static [&'static str],
    ) {
        let reason = match availability {
            Availability::Web => None,
            Availability::NotYetWeb => Some(REASON_NOT_YET_WEB),
            _ => Some(REASON_LOCAL_FS),
        };
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            risk,
            availability,
            None,
            reason,
            capabilities,
        );
    }

    fn web(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        capabilities: &'static [&'static str],
    ) {
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            Risk::Read,
            Availability::Web,
            Some(WEB_ROUTE_PROJECTS),
            None,
            capabilities,
        );
    }

    /// A `web` row that resolves to an explicit typed route with its own
    /// risk. Used by the workbench commands, whose endpoints carry a project
    /// id and (for `upgrade`) a genuine local-write risk that the read-only
    /// [`web`](Self::web) helper must not understate.
    #[allow(clippy::too_many_arguments)]
    fn web_at(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        route: &'static str,
        capabilities: &'static [&'static str],
    ) {
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            risk,
            Availability::Web,
            Some(route),
            None,
            capabilities,
        );
    }

    /// A `web` row that is executable from the portal: it resolves to an
    /// explicit typed `POST` admin route and carries a self-describing
    /// `execution` block (method, the row's own risk, the mandatory confirm
    /// and digest-binding, and the ordered typed parameters) so the frontend
    /// renders a runnable preview→confirm→apply control from the catalog with
    /// no bespoke wiring. `parameters` is `[(name, kind, required)]` with
    /// `kind` one of `string`, `string_array` or `boolean` — never a path or
    /// argv. Used by the project feature/spec lifecycle write commands.
    #[allow(clippy::too_many_arguments)]
    fn web_exec(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        route: &'static str,
        method: &'static str,
        parameters: &[(&'static str, &'static str, bool)],
        capabilities: &'static [&'static str],
    ) {
        let id = match parent {
            Some(prefix) => format!("{prefix}.{name}"),
            None => name.to_string(),
        };
        let cli_invocation = format!("forge {}", id.replace('.', " "));
        let execution = CommandExecution {
            route,
            method,
            risk: risk.id(),
            confirm_required: true,
            digest_bound: true,
            parameters: parameters
                .iter()
                .map(|(pname, kind, required)| ExecParameter {
                    name: (*pname).to_string(),
                    kind,
                    required: *required,
                })
                .collect(),
        };
        self.rows.push(CommandRow {
            label: name.to_string(),
            cli_invocation,
            id,
            parent_id: parent.map(str::to_string),
            summary,
            category: category.id(),
            scope: scope.id(),
            risk: risk.id(),
            availability: Availability::Web.id(),
            route: Some(route),
            reason: None,
            capabilities,
            execution: Some(execution),
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn cli_only(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        reason: &'static str,
        capabilities: &'static [&'static str],
    ) {
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            risk,
            Availability::CliOnly,
            None,
            Some(reason),
            capabilities,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn provider_required(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        capabilities: &'static [&'static str],
    ) {
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            risk,
            Availability::ProviderRequired,
            None,
            Some(REASON_PROVIDER),
            capabilities,
        );
    }

    fn group(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
    ) {
        self.cli_only(
            parent,
            name,
            summary,
            category,
            scope,
            Risk::Read,
            REASON_GROUP,
            &[],
        );
    }

    fn build(mut self) -> Vec<CommandRow> {
        self.enumerate();
        self.rows
    }

    /// Rows are emitted in the exact order the Clap tree declares, so the
    /// parity oracle and the rendered catalog stay visually in sync.
    fn enumerate(&mut self) {
        use Availability::*;
        use Category::*;
        use Risk::*;
        use Scope::*;
        let caps_registry: &[&str] = &["registry_read"];
        let caps_registry_write: &[&str] = &["registry_read", "registry_write"];
        let caps_local: &[&str] = &["local_filesystem"];
        let caps_git: &[&str] = &["git"];
        let caps_git_remote: &[&str] = &["git", "network"];
        let caps_native: &[&str] = &["native_toolchain", "local_filesystem"];
        let caps_provider: &[&str] = &["external_provider"];
        let caps_provider_gh: &[&str] = &["external_provider", "github_cli"];
        let caps_loopback: &[&str] = &["loopback_bind"];
        let caps_web: &[&str] = &["admin_session", "projects_read"];
        // The workbench upgrade is a genuine local write gated by a
        // confirm + plan-digest round trip, so it declares its own capability
        // rather than reusing the read-only project label.
        let caps_workbench_write: &[&str] = &["admin_session", "project_upgrade"];
        let none: &[&str] = &[];

        // ---------------------------------------------------------------- registry
        self.web(
            None,
            "list",
            "List registered projects.",
            Registry,
            Workspace,
            caps_web,
        );
        self.web_at(
            None,
            "inspect",
            "Inspect one registered project by id or path.",
            Registry,
            Workspace,
            Read,
            WEB_ROUTE_PROJECT_DETAIL,
            caps_web,
        );
        self.web_exec(
            None,
            "register",
            "Validate the manifest in a directory and persist the project.",
            Registry,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PROJECT_REGISTER,
            "POST",
            &[("project", "string", true)],
            caps_local,
        );
        self.web_exec(
            None,
            "import",
            "Inspect an existing repository and, on acceptance, adopt it.",
            Registry,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PROJECT_IMPORT,
            "POST",
            &[
                ("project", "string", true),
                ("profile", "string", false),
                ("id", "string", false),
            ],
            caps_local,
        );
        self.group(
            None,
            "workspace",
            "Converge a workspace root into the registry with a per-directory report.",
            Registry,
            Workspace,
        );
        self.cli_only(
            Some("workspace"),
            "sync",
            "Scan the immediate children of a workspace root and register or adopt each one.",
            Registry,
            Workspace,
            LocalWrite,
            REASON_LOCAL_FS,
            caps_local,
        );
        self.group(
            None,
            "graduation",
            "Graduate a local `platform.idea-graduation` artifact into a project.",
            Registry,
            Workspace,
        );
        self.leaf(
            Some("graduation"),
            "preview",
            "Validate a graduation artifact and show the mapped brief without choosing a destination (read-only).",
            Registry,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("graduation"),
            "import",
            "Validate a graduation artifact and, with `--confirm`, create the project.",
            Registry,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );

        // ---------------------------------------------------------------- creation
        self.web_exec(
            None,
            "new",
            "Create a new project deterministically from pinned profile assets.",
            Creation,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PROJECT_NEW,
            "POST",
            &[
                ("project", "string", true),
                ("profile", "string", true),
                ("name", "string", false),
                ("features", "string_array", false),
            ],
            caps_local,
        );

        // profile
        self.group(
            None,
            "profile",
            "Inspect versioned MVP profile descriptors and compatibility.",
            Creation,
            Profile,
        );
        self.leaf(
            Some("profile"),
            "list",
            "List all MVP profiles with descriptor versions.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("profile"),
            "inspect",
            "Inspect one MVP profile descriptor.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("profile"),
            "resolve",
            "Resolve a profile plus requested capabilities without changing files.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.cli_only(
            Some("profile"),
            "preflight",
            "Preflight the profile's required toolchain without claiming it was tested.",
            Creation,
            Profile,
            Read,
            REASON_NATIVE,
            caps_native,
        );

        // kit
        self.group(
            None,
            "kit",
            "Repack or verify the committed shared-layer kit feed.",
            Creation,
            Forge,
        );
        self.cli_only(Some("kit"), "pack", "Repack the committed feed from a checked-out sibling library (nothing outside the repository is written).", Creation, Forge, LocalWrite, REASON_LOCAL_FS, caps_local);
        self.cli_only(Some("kit"), "verify", "Check a committed feed against the kit version a project declares; this is the drift gate.", Creation, Project, Read, REASON_LOCAL_FS, caps_local);
        self.cli_only(
            Some("kit"),
            "upgrade",
            "Show, and on confirmation apply, an explicit shared-layer kit upgrade.",
            Creation,
            Project,
            LocalWrite,
            REASON_LOCAL_FS,
            caps_local,
        );

        // feature
        self.group(
            None,
            "feature",
            "Resolve and manage versioned features.",
            Creation,
            Profile,
        );
        self.leaf(
            Some("feature"),
            "list",
            "List all catalog features with tested versions and strategies.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("feature"),
            "inspect",
            "Inspect one catalog feature descriptor.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(Some("feature"), "resolve", "Resolve requested capabilities into a deterministic install plan without changing files.", Creation, Profile, Read, NotYetWeb, none);
        self.web_exec(
            Some("feature"),
            "add",
            "Add a feature (plus missing dependencies) to a project.",
            Creation,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_FEATURE,
            "POST",
            &[("feature", "string", true), ("version", "string", false)],
            caps_local,
        );
        self.web_exec(
            Some("feature"),
            "remove",
            "Remove a feature from a project.",
            Creation,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_FEATURE_REMOVE,
            "POST",
            &[("feature", "string", true)],
            caps_local,
        );
        self.web_exec(
            Some("feature"),
            "upgrade",
            "Upgrade a feature to the tested catalog version.",
            Creation,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_FEATURE_UPGRADE,
            "POST",
            &[("feature", "string", true), ("version", "string", false)],
            caps_local,
        );

        self.web_at(
            None,
            "upgrade",
            "Plan and apply deterministic project/fleet upgrades with conflict handoff.",
            Creation,
            Workspace,
            LocalWrite,
            WEB_ROUTE_PROJECT_PLAN,
            caps_workbench_write,
        );

        // component
        self.group(
            None,
            "component",
            "Discover, resolve and promote semantic components.",
            Creation,
            Profile,
        );
        self.leaf(
            Some("component"),
            "list",
            "List the versioned semantic component catalog.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("component"),
            "inspect",
            "Inspect one catalog component (contract, evidence, compatibility).",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("component"),
            "resolve",
            "Resolve the named components for a profile with quality-aware selection.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(Some("component"), "qualify", "Promote a component's quality level (evidence-gated; preserves the prior level on failure).", Creation, Profile, LocalWrite, NotYetWeb, caps_local);

        // ui-pattern
        self.group(
            None,
            "ui-pattern",
            "Discover, resolve and install semantic UI patterns across web and Flutter.",
            Creation,
            Profile,
        );
        self.leaf(
            Some("ui-pattern"),
            "list",
            "List the versioned semantic UI pattern catalog.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("ui-pattern"),
            "inspect",
            "Inspect one catalog UI pattern (state, design, adapter, evidence).",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("ui-pattern"),
            "resolve",
            "Resolve the named UI patterns for a profile with quality-aware selection.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(Some("ui-pattern"), "install", "Install a UI pattern's ordinary source and write a metadata receipt (refuses to overwrite customized files).", Creation, Project, LocalWrite, NotYetWeb, caps_local);

        // intent
        self.group(None, "intent", "Validate a structured intent, resolve it into a deterministic assembly plan, and apply it with explicit confirmation.", Creation, Profile);
        self.leaf(
            Some("intent"),
            "validate",
            "Validate a structured intent without resolving or applying it.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(Some("intent"), "resolve", "Resolve a validated intent into a reviewable deterministic assembly plan and persist its receipt.", Creation, Profile, LocalWrite, NotYetWeb, caps_local);
        self.leaf(
            Some("intent"),
            "apply",
            "Re-validate and apply a previously persisted plan (requires `--confirm`).",
            Creation,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("intent"),
            "list",
            "List the persisted plan receipts under `.forge/planner/`.",
            Creation,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );

        // procedure
        self.group(
            None,
            "procedure",
            "Discover, inspect and validate portable AI procedures over stable Core operations.",
            Creation,
            Profile,
        );
        self.leaf(
            Some("procedure"),
            "list",
            "List the named AI procedures in the catalog.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("procedure"),
            "inspect",
            "Inspect one named AI procedure (prerequisites, ordered steps, verification).",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.cli_only(
            Some("procedure"),
            "validate",
            "Validate a procedure spec from a JSON file (typed refusals).",
            Creation,
            Project,
            Read,
            REASON_FILE_STDIN,
            caps_local,
        );

        // standard
        self.group(
            None,
            "standard",
            "List, inspect, check, diff and upgrade versioned standard-pack snapshots.",
            Creation,
            Profile,
        );
        self.leaf(
            Some("standard"),
            "list",
            "List all versioned standard packs with support and evidence state.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("standard"),
            "inspect",
            "Inspect one standard pack version (`<pack>@<version>`).",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("standard"),
            "check",
            "Verify a project's `.standard/` snapshot against its ownership receipt.",
            Creation,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("standard"),
            "diff",
            "Show what an upgrade to a pack version would change (read-only).",
            Creation,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(Some("standard"), "upgrade", "Upgrade a project's snapshot to a pack version; refuses modified files without `--confirm`.", Creation, Project, LocalWrite, NotYetWeb, caps_local);

        // ---------------------------------------------------------------- quality
        self.web_at(
            None,
            "doctor",
            "Inspect project health and evidence-based maturity without changing files.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_PROJECT_DETAIL,
            caps_web,
        );
        self.web_at(
            None,
            "check",
            "Report the in-process doctor, governance and readiness status for one managed project (read-only).",
            Quality,
            Project,
            Read,
            WEB_ROUTE_PROJECT_STATUS,
            caps_web,
        );
        self.cli_only(
            None,
            "gate",
            "Execute the project's declared shared gate runtime and journal revision-bound evidence.",
            Quality,
            Project,
            LocalWrite,
            REASON_NATIVE,
            caps_native,
        );
        self.cli_only(
            None,
            "test",
            "Run the profile's native test command on the named project.",
            Quality,
            Project,
            LocalWrite,
            REASON_NATIVE,
            caps_native,
        );
        self.cli_only(
            None,
            "commit",
            "Stage the named paths and create a single scoped commit.",
            Quality,
            Project,
            LocalWrite,
            REASON_GIT,
            caps_git,
        );

        // spec
        self.group(None, "spec", "Generate bounded spec proposals and route findings to deterministic, semantic or manual remediation.", Quality, Project);
        self.web_exec(
            Some("spec"),
            "generate",
            "Generate a bounded spec for the named project and finding set.",
            Quality,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_SPEC,
            "POST",
            &[
                ("findings", "string_array", true),
                ("reason", "string", false),
            ],
            caps_local,
        );
        self.leaf(
            Some("spec"),
            "list",
            "List all generated specs under `.forge/specs/` for the named project.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("spec"),
            "inspect",
            "Show the full bounded proposal for a generated spec.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("spec"),
            "route",
            "Classify one finding into a deterministic, semantic or manual route.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.web_exec(
            Some("spec"),
            "apply",
            "Apply a routing decision: deterministic action is recorded, semantic produces a spec, manual is noted.",
            Quality,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_SPEC_APPLY,
            "POST",
            &[
                ("findings", "string_array", true),
                ("reason", "string", false),
            ],
            caps_local,
        );

        // remediate
        self.group(
            None,
            "remediate",
            "Plan and apply ownership-safe local remediation.",
            Quality,
            Project,
        );
        self.leaf(
            Some("remediate"),
            "scan",
            "Inspect automatic findings and produce a read-only plan.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("remediate"),
            "plan",
            "Produce a versioned read-only remediation plan.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("remediate"),
            "diff",
            "Show the files a remediation plan would change.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("remediate"),
            "apply",
            "Apply a plan after explicit confirmation.",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );

        // describe
        self.group(None, "describe", "Suggest, list, show, approve and reject semantic project descriptions (`forge-semantic-proposal/0.1.0`).", Quality, Project);
        self.leaf(
            Some("describe"),
            "suggest",
            "Suggest a bounded description for the named project.",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("describe"),
            "list",
            "List every recorded semantic proposal for the named project.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("describe"),
            "show",
            "Show the full proposal manifest for one proposal id.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("describe"),
            "approve",
            "Approve a `Suggested` proposal after explicit confirmation.",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("describe"),
            "reject",
            "Reject a `Suggested` proposal after explicit confirmation.",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );

        // classify
        self.group(None, "classify", "Suggest, list, show, approve and reject semantic project classifications (domain, portfolio tags, profile, lifecycle).", Quality, Project);
        self.leaf(
            Some("classify"),
            "suggest",
            "Suggest a bounded classification for the named project.",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("classify"),
            "list",
            "List every recorded classification proposal for the named project.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("classify"),
            "show",
            "Show the full proposal manifest for one proposal id.",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("classify"),
            "approve",
            "Approve a `Suggested` proposal after explicit confirmation.",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("classify"),
            "reject",
            "Reject a `Suggested` proposal after explicit confirmation.",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );

        // contract
        self.group(
            None,
            "contract",
            "Vendor, inspect and project platform contracts into envelopes.",
            Quality,
            Forge,
        );
        self.leaf(
            Some("contract"),
            "list",
            "List every versioned surface and its platform mapping.",
            Quality,
            Forge,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("contract"),
            "inspect",
            "Inspect one platform family and its schema.",
            Quality,
            Forge,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("contract"),
            "emit",
            "Project a Core record into a platform envelope (read-only).",
            Quality,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.cli_only(
            Some("contract"),
            "validate",
            "Validate a contract envelope against the vendored schemas (file or stdin).",
            Quality,
            Forge,
            Read,
            REASON_FILE_STDIN,
            caps_local,
        );

        // governance
        self.group(
            None,
            "governance",
            "Inspect and select standalone or optional external governance providers.",
            Quality,
            Workspace,
        );
        self.leaf(
            Some("governance"),
            "list",
            "List the local provider and the configured optional provider.",
            Quality,
            Workspace,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("governance"),
            "status",
            "Run the selected provider and record a bounded observation.",
            Quality,
            Workspace,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("governance"),
            "inspect",
            "Alias for status that returns the full normalized observation.",
            Quality,
            Workspace,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("governance"),
            "use",
            "Select a provider without changing forge.yaml or registry identity.",
            Quality,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );

        // ---------------------------------------------------------------- release
        // release
        self.group(
            None,
            "release",
            "Prepare, apply, list and inspect gated resumable releases.",
            Release,
            Project,
        );
        self.web_at(Some("release"), "prepare", "Capture semver, changelog, source revision and the doctor/test/DriftWatch evidence into a reviewable plan.", Release, Project, Read, WEB_ROUTE_ADMIN_RELEASE_PLAN, caps_local);
        self.web_exec(Some("release"), "apply", "Apply a verified release plan: walks every stage with safe retry and per-stage records (tag, push, package, container, notes).", Release, Project, RemoteWrite, WEB_ROUTE_ADMIN_RELEASE, "POST", &[("version", "string", true)], caps_git_remote);
        self.leaf(
            Some("release"),
            "list",
            "List all persisted releases under the project's release directory.",
            Release,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("release"),
            "inspect",
            "Inspect a single persisted release record.",
            Release,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );

        // deploy
        self.group(
            None,
            "deploy",
            "Plan, apply, observe and inspect adapter-based deployments.",
            Release,
            Project,
        );
        self.web_at(Some("deploy"), "plan", "Capture target, source revision and the configured health check into a reviewable plan.", Release, Project, Read, WEB_ROUTE_ADMIN_DEPLOY_PLAN, caps_local);
        self.web_exec(Some("deploy"), "apply", "Apply a verified deploy plan: invokes the adapter and captures the health observation (requires `--confirm`).", Release, Project, RemoteWrite, WEB_ROUTE_ADMIN_DEPLOY, "POST", &[("target", "string", false)], caps_local);
        self.push(
            Some("deploy"),
            "observe",
            "Re-run the health check on a previously applied deploy (requires `--confirm`).",
            Release,
            Project,
            RemoteWrite,
            ProjectCapabilityRequired,
            None,
            Some(REASON_PROJECT_CAPABILITY),
            caps_local,
        );
        self.leaf(
            Some("deploy"),
            "list",
            "List all persisted deploys for the named project.",
            Release,
            Project,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("deploy"),
            "inspect",
            "Inspect a single persisted deploy by its id.",
            Release,
            Project,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("deploy"),
            "status",
            "Show the persisted Forge publish/deploy status without contacting a provider.",
            Release,
            Project,
            Read,
            NotYetWeb,
            caps_registry,
        );

        // publish — the group itself performs the bare publish, unlike other
        // groups. The provider, project and committed revision are resolved
        // server-side, so the executable row carries no browser-supplied
        // parameters.
        self.web_exec(
            None,
            "publish",
            "Publish a registered project through the server-configured provider; the provider id, project and commit revision are resolved server-side.",
            Release,
            Project,
            RemoteWrite,
            WEB_ROUTE_ADMIN_PUBLISH,
            "POST",
            &[],
            caps_provider,
        );
        // publish.provider
        self.group(
            Some("publish"),
            "provider",
            "Inspect or switch standalone publish providers.",
            Release,
            Provider,
        );
        self.leaf(
            Some("publish.provider"),
            "list",
            "List configured providers and their enabled state.",
            Release,
            Provider,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("publish.provider"),
            "inspect",
            "Inspect one configured provider without invoking it.",
            Release,
            Provider,
            Read,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("publish.provider"),
            "enable",
            "Enable an already configured provider.",
            Release,
            Provider,
            LocalWrite,
            NotYetWeb,
            none,
        );
        self.leaf(
            Some("publish.provider"),
            "disable",
            "Disable an already configured provider.",
            Release,
            Provider,
            LocalWrite,
            NotYetWeb,
            none,
        );
        self.provider_required(
            Some("publish"),
            "sync",
            "Sync a project source tree to the Mac via SSH/rsync (Stage 1).",
            Release,
            Project,
            RemoteWrite,
            caps_provider,
        );
        self.provider_required(
            Some("publish"),
            "db",
            "Provision the shared PostgreSQL database on the Mac (Stage 2).",
            Release,
            Project,
            RemoteWrite,
            caps_provider,
        );
        self.provider_required(
            Some("publish"),
            "fleet",
            "Publish every project in an explicit inventory that has a Compose contract.",
            Release,
            Workspace,
            RemoteWrite,
            caps_provider,
        );
        self.provider_required(
            Some("publish"),
            "prepare",
            "Allocate ports and prepare the project environment on the Mac (Stage 2).",
            Release,
            Project,
            RemoteWrite,
            caps_provider,
        );
        self.provider_required(
            Some("publish"),
            "deploy",
            "Trigger the Jenkins deploy job on the Mac (Stage 3).",
            Release,
            Project,
            RemoteWrite,
            caps_provider,
        );
        self.provider_required(
            Some("publish"),
            "all",
            "Run sync, prepare and deploy in order; stops at the first failure.",
            Release,
            Project,
            RemoteWrite,
            caps_provider,
        );

        self.cli_only(
            None,
            "push",
            "Push the named ref to the project's remote (requires `--confirm`).",
            Release,
            Project,
            RemoteWrite,
            REASON_GIT,
            caps_git_remote,
        );
        self.cli_only(
            None,
            "mirror",
            "Distribute a project's refs to its canonical primary and configured one-way mirrors (requires `--confirm`).",
            Release,
            Project,
            RemoteWrite,
            REASON_GIT,
            caps_git_remote,
        );

        // docs
        self.group(
            None,
            "docs",
            "Translate the canonical documentation into enabled derivative locales.",
            Release,
            Project,
        );
        self.provider_required(Some("docs"), "translate", "Translate the canonical source document into one enabled locale (or every enabled locale with `--all`).", Release, Project, LocalWrite, caps_provider);

        // readiness
        self.group(
            None,
            "readiness",
            "Generate the supported profile fixtures natively and evaluate release readiness.",
            Release,
            Forge,
        );
        self.cli_only(Some("readiness"), "matrix", "Generate every supported profile fixture into a disposable directory and run its native build/test without Forge on PATH.", Release, Forge, LocalWrite, REASON_NATIVE, caps_native);
        self.cli_only(
            Some("readiness"),
            "artifact",
            "Report the platform-native Forge binary evidence (path, SHA-256, version smoke).",
            Release,
            Forge,
            Read,
            REASON_NATIVE,
            caps_native,
        );
        self.cli_only(
            Some("readiness"),
            "check",
            "Evaluate the release gate over the selected matrix rows plus the artifact smoke.",
            Release,
            Forge,
            Read,
            REASON_NATIVE,
            caps_native,
        );

        // provider (evidence)
        self.group(
            None,
            "provider",
            "Record opt-in controlled evidence for the external provider boundaries.",
            Release,
            Provider,
        );
        self.leaf(Some("provider"), "matrix", "Report the provider matrix; without `--live` every row is `not-run` and never claims support.", Release, Provider, Read, NotYetWeb, none);
        self.provider_required(Some("provider"), "run", "Drive one controlled round trip for a provider with provenance and teardown (live rows require FORGE_PROVIDER_LIVE=1).", Release, Provider, LocalWrite, caps_provider);
        self.leaf(Some("provider"), "inspect", "Describe one provider's boundary, binary override, secret and teardown rules without probing.", Release, Provider, Read, NotYetWeb, none);

        // ---------------------------------------------------------------- fleet & catalog surfaces
        // fleet
        self.group(
            None,
            "fleet",
            "Observe the portfolio declared by an external workspace registry, read-only.",
            Registry,
            Workspace,
        );
        self.web(Some("fleet"), "list", "List the portfolio declared by the workspace registry as timestamped fleet observations.", Registry, Workspace, caps_web);
        self.web_at(
            Some("fleet"),
            "status",
            "Report the fleet registry health only (source, freshness, counts; no entries).",
            Registry,
            Workspace,
            Read,
            WEB_ROUTE_FLEET_STATUS,
            caps_web,
        );
        self.leaf(Some("fleet"), "inspect", "Inspect one declared fleet entry by id (read-only; unmanaged entries can never be operated on through the mirror).", Registry, Workspace, Read, NotYetWeb, caps_registry);
        self.provider_required(Some("fleet"), "online", "Probe the served router rules and target containers to verdict whether every routed host is ONLINE/DOWN/NO-ROUTE/NOT-DEPLOYED (read-only).", Registry, Provider, Read, caps_provider);

        // project (catalog)
        self.group(
            None,
            "project",
            "Query the normalized read-only project catalog (`forge-project-catalog/0.1.0`).",
            Registry,
            Workspace,
        );
        self.leaf(
            Some("project"),
            "list",
            "List the normalized project catalog with provenance.",
            Registry,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("project"),
            "inspect",
            "Inspect every catalog record for one project id (all sources).",
            Registry,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("project"),
            "tags",
            "List distinct tags across the filtered catalog with counts.",
            Registry,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("project"),
            "languages",
            "List distinct languages across the filtered catalog with counts.",
            Registry,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("project"),
            "gaps",
            "Report evidence-backed project metadata gaps (`forge-project-evidence/0.1.0`).",
            Registry,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.group(Some("project"), "github", "Optional GitHub metadata observation and approved change surface (`github-project-metadata-adapter`).", Registry, Provider);
        self.provider_required(Some("project.github"), "observe", "Observe one or more GitHub repositories through the configured adapter (`forge-github-metadata/0.1.0`); read-only.", Registry, Provider, Read, caps_provider_gh);
        self.provider_required(Some("project.github"), "propose", "Propose an approved metadata change through the adapter (pull request by default; direct mode needs an echoed confirmation).", Registry, Provider, RemoteWrite, caps_provider_gh);
        self.cli_only(Some("project.github"), "auth", "Probe the installed GitHub CLI authentication state without reading or displaying the credential.", Registry, Provider, Read, REASON_LOCAL_TOOLCHAIN, caps_provider_gh);
        self.cli_only(Some("project.github"), "clone", "Clone an existing GitHub repository through the installed `gh` CLI to a local destination (requires `--confirm`).", Registry, Project, LocalWrite, REASON_LOCAL_TOOLCHAIN, caps_provider_gh);
        self.provider_required(Some("project.github"), "create", "Create a remote GitHub repository for an existing local project (private default; public and source push need explicit confirmations).", Registry, Provider, RemoteWrite, caps_provider_gh);
        self.provider_required(Some("project.github"), "pull-request", "Open a draft pull request for a registered project through the installed `gh` CLI (requires `--confirm`).", Registry, Provider, RemoteWrite, caps_provider_gh);

        // inventory
        self.group(None, "inventory", "Load, validate and report a portable project inventory (`forge-project-inventory/0.1.0`); read-only.", Registry, Forge);
        self.web(Some("inventory"), "show", "Load, validate and project an inventory into the fleet classification report (never invokes a provider or mutates the registry).", Registry, Forge, caps_web);

        // ---------------------------------------------------------------- portfolio
        self.group(
            None,
            "portfolio",
            "Manage user-owned portfolio metadata and import source-owned evidence snapshots.",
            Portfolio,
            Workspace,
        );
        self.group(
            Some("portfolio"),
            "tag",
            "Manage user-owned tags.",
            Portfolio,
            Workspace,
        );
        self.leaf(
            Some("portfolio.tag"),
            "add",
            "Attach a tag to a project, creating the tag on first use.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.tag"),
            "remove",
            "Detach a tag from a project; the tag itself is kept.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.tag"),
            "list",
            "List every tag, or the tags on one project.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "relation",
            "Manage user-owned relationships between projects.",
            Portfolio,
            Workspace,
        );
        self.leaf(
            Some("portfolio.relation"),
            "add",
            "Link two projects; the same link twice stays one row.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.relation"),
            "remove",
            "Remove one declared relation.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.relation"),
            "list",
            "List relations touching one project, or every relation.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "review",
            "Record a review decision and the current classification.",
            Portfolio,
            Workspace,
        );
        self.leaf(Some("portfolio.review"), "set", "Record a review decision and optionally the lifecycle, next action and blocker for one project.", Portfolio, Workspace, LocalWrite, NotYetWeb, caps_registry_write);
        self.leaf(
            Some("portfolio.review"),
            "list",
            "List the review history for one project.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "goal",
            "Manage user-owned portfolio goals.",
            Portfolio,
            Workspace,
        );
        self.leaf(
            Some("portfolio.goal"),
            "add",
            "Create a goal; re-running with the same title updates it.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.goal"),
            "link",
            "Attach a project to a goal, creating the goal when new.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.goal"),
            "list",
            "List every goal with its projects.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "evidence",
            "Import source-owned observations as append-only snapshots.",
            Portfolio,
            Workspace,
        );
        self.leaf(Some("portfolio.evidence"), "import", "Append one source-owned observation as a snapshot (redacted before storage, never rewritten).", Portfolio, Workspace, LocalWrite, NotYetWeb, caps_registry_write);
        self.leaf(
            Some("portfolio.evidence"),
            "list",
            "List every snapshot for one project, newest first.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("portfolio"),
            "show",
            "Show one project's whole portfolio projection.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "share",
            "Manage the explicit allowlist of projects that may be published.",
            Portfolio,
            Workspace,
        );
        // The share family is the delivery-controls web surface
        // (`forge-web-delivery-controls`): every row resolves to a typed,
        // session-gated route whose mutations all require an explicit
        // confirmation plus the reviewed manifest digest.
        self.web_at(Some("portfolio.share"), "set", "Define or replace one project's public share record; nothing is public until approved and published.", Portfolio, Workspace, Risk::LocalWrite, WEB_ROUTE_DELIVERY_ALLOWLIST, caps_registry_write);
        self.web_at(
            Some("portfolio.share"),
            "remove",
            "Withdraw a project's share record; it leaves the public catalog.",
            Portfolio,
            Workspace,
            Risk::LocalWrite,
            WEB_ROUTE_DELIVERY_ALLOWLIST_REMOVE,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.share"),
            "show",
            "Show one project's share record and its allowlisted surfaces.",
            Portfolio,
            Workspace,
            Risk::Read,
            WEB_ROUTE_DELIVERY_OVERVIEW,
            caps_registry,
        );
        self.web_at(
            Some("portfolio.share"),
            "list",
            "List every share record, newest state first.",
            Portfolio,
            Workspace,
            Risk::Read,
            WEB_ROUTE_DELIVERY_OVERVIEW,
            caps_registry,
        );
        self.web_at(
            Some("portfolio.share"),
            "preview",
            "Preview the candidate manifest: exact canonical bytes, hash and findings (read-only).",
            Portfolio,
            Workspace,
            Risk::Read,
            WEB_ROUTE_DELIVERY_PREVIEW,
            caps_registry,
        );
        self.web_at(
            Some("portfolio.share"),
            "approve",
            "Approve one exact manifest hash for publication.",
            Portfolio,
            Workspace,
            Risk::LocalWrite,
            WEB_ROUTE_DELIVERY_APPROVE,
            caps_registry_write,
        );
        self.web_at(Some("portfolio.share"), "publish", "Publish the approved manifest through the default-safe local export; the optional adapter stays CLI-only and the browser never names a path.", Portfolio, Workspace, Risk::RemoteWrite, WEB_ROUTE_DELIVERY_PUBLISH, caps_registry_write);
        self.web_at(
            Some("portfolio.share"),
            "reconcile",
            "Resolve a partial publication reported as `unknown`.",
            Portfolio,
            Workspace,
            Risk::LocalWrite,
            WEB_ROUTE_DELIVERY_RECONCILE,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.share"),
            "audit",
            "Show the approval and publication audit trail.",
            Portfolio,
            Workspace,
            Risk::Read,
            WEB_ROUTE_DELIVERY_OVERVIEW,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "interest",
            "Import and compare aggregate, privacy-safe interest evidence.",
            Portfolio,
            Workspace,
        );
        self.leaf(
            Some("portfolio.interest"),
            "import",
            "Import a versioned batch of aggregate snapshots; every record is reported separately.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.interest"),
            "list",
            "List one project's stored snapshots, or every project's.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("portfolio.interest"),
            "show",
            "Show one project's whole interest projection with its refusals.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("portfolio.interest"),
            "compare",
            "Compare projects on the allowlisted metrics, labeled with source and freshness.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("portfolio.interest"),
            "trend",
            "One metric's windowed history for one project.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("portfolio.interest"),
            "audit",
            "The refusals this store recorded, newest first.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.group(Some("portfolio"), "activation", "Read-only readiness verdict on whether aggregate evidence justifies the activation follow-up. No billing.", Portfolio, Workspace);
        self.leaf(Some("portfolio.activation"), "readiness", "Verdict on whether a project's aggregate interest evidence justifies the product-owned activation follow-up (read-only, no billing).", Portfolio, Workspace, Read, NotYetWeb, caps_registry);

        // analytics
        self.group(None, "analytics", "Inspect the existing content / analytics providers configured for a project and aggregate timestamped project metrics.", Portfolio, Project);
        self.provider_required(Some("analytics"), "inspect", "Inspect the manifest's analytics block, recording timestamped health observations for each configured provider.", Portfolio, Project, Read, caps_provider);
        self.leaf(Some("analytics"), "metrics", "Aggregate timestamped project metrics from the registry, doctor, deploy and external observations.", Portfolio, Project, Read, NotYetWeb, caps_registry);

        // ---------------------------------------------------------------- delivery
        self.group(None, "delivery", "Coordinate the staged delivery workflow (preflight → stage → production) and the optional post-deploy Hermora onboarding.", Delivery, Project);
        self.web_at(Some("delivery"), "status", "Read the project's current delivery phase, revision and most-recent per-verb journal evidence.", Delivery, Project, Read, WEB_ROUTE_ADMIN_DELIVERY_STATUS, caps_registry);
        self.web_exec(
            Some("delivery"),
            "preflight",
            "Invoke the publish provider's `preflight` operation and record the terminal evidence.",
            Delivery,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT,
            "POST",
            &[],
            caps_provider,
        );
        self.web_exec(Some("delivery"), "stage", "Invoke the provider's `publish` for the stage environment (requires `confirm_operation_id` from a healthy preflight).", Delivery, Project, RemoteWrite, WEB_ROUTE_ADMIN_DELIVERY_STAGE, "POST", &[("confirm_operation_id", "string", true)], caps_provider);
        self.web_exec(Some("delivery"), "promote", "Invoke the provider's `publish` for production (requires `confirm_revision` matching the registered source revision).", Delivery, Project, RemoteWrite, WEB_ROUTE_ADMIN_DELIVERY_PROMOTE, "POST", &[("confirm_revision", "string", true)], caps_provider);
        self.web_exec(Some("delivery"), "hermora-retry", "Retry the optional Hermora site onboarding for a healthy deployment; never republishes.", Delivery, Project, RemoteWrite, WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY, "POST", &[("deployment_url", "string", true), ("secret_ref", "string", true)], caps_provider);

        // studio
        self.group(None, "studio", "Site Studio: review an AppSpec, run a bounded preview, and journal scoped refinement requests (`forge-studio-preview-refinement`).", Delivery, Project);
        self.leaf(Some("studio"), "spec", "Validate an AppSpec from a YAML file and print the closed `forge-app-spec/0.1.0` envelope; persists only with `--confirm yes`.", Delivery, Project, LocalWrite, NotYetWeb, caps_local);
        self.leaf(Some("studio"), "preview", "Read the current Studio session record; `--start`/`--stop` bind the reserved port only with a confirmation token.", Delivery, Project, LocalWrite, NotYetWeb, caps_local);
        self.leaf(Some("studio"), "refine", "Submit a refinement request; validates, journals a `studio.refine` row and bumps `app_revision`.", Delivery, Project, LocalWrite, NotYetWeb, caps_registry_write);

        // ---------------------------------------------------------------- identity
        self.group(
            None,
            "identity",
            "Validate, challenge, complete and terminate per-project OIDC admin sessions.",
            Identity,
            Project,
        );
        self.cli_only(Some("identity"), "setup", "Initialize the one Forge-wide portal administrator (password is read without terminal echo).", Identity, Forge, SessionAdmin, REASON_TTY_HIDDEN, none);
        self.cli_only(Some("identity"), "change-password", "Replace the Forge-wide administrator password without changing the email; revokes every active browser session (new password read without terminal echo).", Identity, Forge, SessionAdmin, REASON_TTY_HIDDEN, none);
        self.cli_only(Some("identity"), "generate-password", "Print one strong random password from operating-system entropy without reading or writing the registry.", Identity, Forge, Read, REASON_LOCAL_SECRET, none);
        self.leaf(
            Some("identity"),
            "validate-config",
            "Validate the manifest's `identity:` block without contacting any provider.",
            Identity,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("identity"),
            "build-challenge",
            "Build a fresh OIDC authorization request (state, nonce, PKCE) for the named project.",
            Identity,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.cli_only(Some("identity"), "complete-auth", "Complete the OIDC round trip from a provider callback + claims and mint a per-project admin session.", Identity, Project, SessionAdmin, REASON_TTY_HIDDEN, none);
        self.leaf(
            Some("identity"),
            "session-list",
            "List every persisted admin session for the named project.",
            Identity,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("identity"),
            "session-inspect",
            "Inspect one persisted admin session.",
            Identity,
            Project,
            Read,
            NotYetWeb,
            caps_local,
        );
        self.cli_only(Some("identity"), "session-validate", "Validate a session id against the named project and permission (read-only diagnostic).", Identity, Project, Read, REASON_LOCAL_FS, caps_local);
        self.leaf(
            Some("identity"),
            "session-terminate",
            "Terminate the named admin session and remove its persisted state.",
            Identity,
            Project,
            SessionAdmin,
            NotYetWeb,
            caps_local,
        );

        // ---------------------------------------------------------------- transports
        self.group(
            None,
            "api",
            "Serve the optional HTTP transport over Core on a loopback listener.",
            Transports,
            Forge,
        );
        self.cli_only(Some("api"), "serve", "Run the HTTP/1.1 API server on a loopback listener; authorization is required for every route other than `/healthz`.", Transports, Forge, Read, REASON_TRANSPORT, caps_loopback);
        self.group(
            None,
            "web",
            "Serve standalone Forge browser assets from the Rust static web server.",
            Transports,
            Forge,
        );
        self.cli_only(
            Some("web"),
            "serve",
            "Serve files from `frontend/` on an independent loopback web listener.",
            Transports,
            Forge,
            Read,
            REASON_TRANSPORT,
            caps_loopback,
        );
        self.cli_only(
            None,
            "mcp",
            "Run the MCP stdio server over the mature Core operations.",
            Transports,
            Forge,
            Read,
            REASON_TRANSPORT,
            caps_loopback,
        );
        self.cli_only(
            Some("mcp"),
            "serve",
            "Run the JSON-RPC 2.0 stdio server until stdin closes.",
            Transports,
            Forge,
            Read,
            REASON_TRANSPORT,
            none,
        );
        self.group(
            None,
            "portal",
            "Render the optional control-plane portal dashboard and per-section views.",
            Transports,
            Forge,
        );
        self.cli_only(Some("portal"), "dashboard", "Render the top-level control-plane dashboard for one project or the whole registry (server-side HTML).", Transports, Forge, Read, REASON_LEGACY_HTML, none);
        self.cli_only(
            Some("portal"),
            "view",
            "Render a single legacy portal section (server-side HTML).",
            Transports,
            Forge,
            Read,
            REASON_LEGACY_HTML,
            none,
        );

        // ---------------------------------------------------------------- agent
        self.group(
            None,
            "agent",
            "Manage agent sessions for the named project.",
            Creation,
            Project,
        );
        self.cli_only(
            Some("agent"),
            "start",
            "Start the project's agent session through the configured adapter.",
            Creation,
            Project,
            LocalWrite,
            REASON_AGENT,
            caps_local,
        );
        self.cli_only(
            Some("agent"),
            "pause",
            "Pause the project's active agent session.",
            Creation,
            Project,
            LocalWrite,
            REASON_AGENT,
            caps_local,
        );
        self.cli_only(
            Some("agent"),
            "takeover",
            "Take over the project's agent session from the adapter.",
            Creation,
            Project,
            LocalWrite,
            REASON_AGENT,
            caps_local,
        );
        self.cli_only(
            Some("agent"),
            "resume",
            "Resume a paused agent session.",
            Creation,
            Project,
            LocalWrite,
            REASON_AGENT,
            caps_local,
        );
        self.cli_only(
            Some("agent"),
            "restart",
            "Restart the project's agent session.",
            Creation,
            Project,
            LocalWrite,
            REASON_AGENT,
            caps_local,
        );
        self.cli_only(
            Some("agent"),
            "new-session",
            "Begin a fresh agent session for the project.",
            Creation,
            Project,
            LocalWrite,
            REASON_AGENT,
            caps_local,
        );
        self.leaf(
            Some("agent"),
            "status",
            "Show the current agent session status for the project.",
            Creation,
            Project,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("agent"),
            "list",
            "List the agent sessions Forge knows about.",
            Creation,
            Project,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.cli_only(
            Some("agent"),
            "run-spec",
            "Run a generated spec through the project's agent adapter.",
            Creation,
            Project,
            LocalWrite,
            REASON_AGENT,
            caps_local,
        );

        // ---------------------------------------------------------------- reference
        self.cli_only(
            None,
            "help",
            "Print the CLI's complete command tree as terminal help text.",
            Reference,
            Forge,
            Read,
            REASON_HELP,
            none,
        );
    }
}

/// The complete catalog, built once from the static rows above.
pub fn rows() -> &'static [CommandRow] {
    static CATALOG: OnceLock<Vec<CommandRow>> = OnceLock::new();
    CATALOG.get_or_init(|| CatalogBuilder::new().build())
}

/// Catalog integrity problems; empty means the catalog satisfies every
/// invariant design.md requires (unique ids, valid parents, reason/state
/// rules, resolvable web routes, complete category coverage).
pub fn problems() -> Vec<String> {
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    let ids: BTreeSet<&str> = rows().iter().map(|row| row.id.as_str()).collect();
    for row in rows() {
        if !seen.insert(row.id.as_str()) {
            issues.push(format!("duplicate catalog id `{}`", row.id));
        }
        if let Some(parent) = &row.parent_id {
            if !ids.contains(parent.as_str()) {
                issues.push(format!(
                    "row `{}` references unknown parent `{}`",
                    row.id, parent
                ));
            }
        } else if row.id.contains('.') {
            issues.push(format!("row `{}` has a dotted id but no parent_id", row.id));
        }
        if row.summary.is_empty() {
            issues.push(format!("row `{}` has an empty summary", row.id));
        }
        match row.availability {
            "web" => {
                if row.reason.is_some() {
                    issues.push(format!("web row `{}` must not carry a reason", row.id));
                }
                match row.route {
                    Some(route) if IMPLEMENTED_WEB_ROUTES.contains(&route) => {}
                    Some(route) => issues.push(format!(
                        "web row `{}` names unimplemented route `{}`",
                        row.id, route
                    )),
                    None => issues.push(format!("web row `{}` has no route", row.id)),
                }
            }
            "disabled"
            | "cli_only"
            | "provider_required"
            | "project_capability_required"
            | "not_yet_web" => {
                if row.reason.is_none_or(|reason| reason.is_empty()) {
                    issues.push(format!(
                        "non-web row `{}` (state {}) must carry a plain-language reason",
                        row.id, row.availability
                    ));
                }
                if row.route.is_some() {
                    issues.push(format!("non-web row `{}` must not name a route", row.id));
                }
            }
            other => issues.push(format!(
                "row `{}` has unknown availability `{}`",
                row.id, other
            )),
        }
        match row.risk {
            "read" | "local_write" | "remote_write" | "session_admin" => {}
            other => issues.push(format!("row `{}` has unknown risk `{}`", row.id, other)),
        }
        match row.scope {
            "workspace" | "project" | "forge" | "profile" | "provider" => {}
            other => issues.push(format!("row `{}` has unknown scope `{}`", row.id, other)),
        }
        if !ALL_CATEGORIES.iter().any(|cat| cat.id() == row.category) {
            issues.push(format!("row `{}` has unknown category", row.id));
        }
        if let Some(prefix) = row.cli_invocation.strip_prefix("forge ") {
            if prefix.replace(' ', ".") != row.id {
                issues.push(format!(
                    "row `{}` has an inconsistent cli_invocation",
                    row.id
                ));
            }
        } else {
            issues.push(format!("row `{}` has a malformed cli_invocation", row.id));
        }
    }
    let used: BTreeSet<&str> = rows().iter().map(|row| row.category).collect();
    for cat in ALL_CATEGORIES {
        if !used.contains(cat.id()) {
            issues.push(format!("category `{}` is never used", cat.id()));
        }
    }
    issues
}

/// The serialized `categories` list for the API envelope.
fn categories_json() -> serde_json::Value {
    let rows = rows();
    let value: Vec<_> = ALL_CATEGORIES
        .iter()
        .map(|cat| {
            json!({
                "id": cat.id(),
                "label": cat.label(),
                "count": rows.iter().filter(|row| row.category == cat.id()).count(),
            })
        })
        .collect();
    json!(value)
}

/// The `GET /v1/admin/commands` body: `{commands, categories, contract}`.
pub fn envelope() -> serde_json::Value {
    json!({
        "contract": CONTRACT_VERSION,
        "categories": categories_json(),
        "commands": rows(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn catalog_has_no_integrity_problems() {
        let issues = problems();
        assert!(issues.is_empty(), "catalog problems: {issues:?}");
    }

    #[test]
    fn catalog_covers_every_clap_path() {
        // The row count equals the 228 Clap paths (probe-verified from
        // `Cli::command()`, including the `identity change-password` and
        // `identity generate-password` leaf commands and the `workspace` /
        // `workspace.sync` bulk-convergence paths) plus the explicit
        // top-level `help` row.
        assert_eq!(rows().len(), 229);
        assert!(rows().iter().any(|row| row.id == "help"));
    }

    #[test]
    fn web_rows_only_point_at_implemented_routes() {
        for row in rows().iter().filter(|row| row.availability == "web") {
            let route = row.route.expect("web row must name a route");
            assert!(
                IMPLEMENTED_WEB_ROUTES.contains(&route),
                "row {} points at unimplemented route {route}",
                row.id
            );
        }
        let web: Vec<(&str, &str)> = rows()
            .iter()
            .filter(|row| row.availability == "web")
            .map(|row| (row.id.as_str(), row.route.expect("web row route")))
            .collect();
        assert_eq!(
            web,
            vec![
                ("list", WEB_ROUTE_PROJECTS),
                ("inspect", WEB_ROUTE_PROJECT_DETAIL),
                ("register", WEB_ROUTE_ADMIN_PROJECT_REGISTER),
                ("import", WEB_ROUTE_ADMIN_PROJECT_IMPORT),
                ("new", WEB_ROUTE_ADMIN_PROJECT_NEW),
                ("feature.add", WEB_ROUTE_ADMIN_FEATURE),
                ("feature.remove", WEB_ROUTE_ADMIN_FEATURE_REMOVE),
                ("feature.upgrade", WEB_ROUTE_ADMIN_FEATURE_UPGRADE),
                ("upgrade", WEB_ROUTE_PROJECT_PLAN),
                ("doctor", WEB_ROUTE_PROJECT_DETAIL),
                ("check", WEB_ROUTE_PROJECT_STATUS),
                ("spec.generate", WEB_ROUTE_ADMIN_SPEC),
                ("spec.apply", WEB_ROUTE_ADMIN_SPEC_APPLY),
                ("release.prepare", WEB_ROUTE_ADMIN_RELEASE_PLAN),
                ("release.apply", WEB_ROUTE_ADMIN_RELEASE),
                ("deploy.plan", WEB_ROUTE_ADMIN_DEPLOY_PLAN),
                ("deploy.apply", WEB_ROUTE_ADMIN_DEPLOY),
                ("publish", WEB_ROUTE_ADMIN_PUBLISH),
                ("fleet.list", WEB_ROUTE_PROJECTS),
                ("fleet.status", WEB_ROUTE_FLEET_STATUS),
                ("inventory.show", WEB_ROUTE_PROJECTS),
                ("portfolio.share.set", WEB_ROUTE_DELIVERY_ALLOWLIST),
                (
                    "portfolio.share.remove",
                    WEB_ROUTE_DELIVERY_ALLOWLIST_REMOVE
                ),
                ("portfolio.share.show", WEB_ROUTE_DELIVERY_OVERVIEW),
                ("portfolio.share.list", WEB_ROUTE_DELIVERY_OVERVIEW),
                ("portfolio.share.preview", WEB_ROUTE_DELIVERY_PREVIEW),
                ("portfolio.share.approve", WEB_ROUTE_DELIVERY_APPROVE),
                ("portfolio.share.publish", WEB_ROUTE_DELIVERY_PUBLISH),
                ("portfolio.share.reconcile", WEB_ROUTE_DELIVERY_RECONCILE),
                ("portfolio.share.audit", WEB_ROUTE_DELIVERY_OVERVIEW),
                ("delivery.status", WEB_ROUTE_ADMIN_DELIVERY_STATUS),
                ("delivery.preflight", WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT),
                ("delivery.stage", WEB_ROUTE_ADMIN_DELIVERY_STAGE),
                ("delivery.promote", WEB_ROUTE_ADMIN_DELIVERY_PROMOTE),
                (
                    "delivery.hermora-retry",
                    WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY
                ),
            ]
        );
    }

    #[test]
    fn publish_routes_are_implemented_web_routes() {
        // Both publish routes are registered by the router and listed as
        // implemented web routes, so the executable `publish` row (which points
        // at the apply route) can never name a route that does not exist and
        // the read-only plan route stays a first-class implemented endpoint.
        assert!(
            IMPLEMENTED_WEB_ROUTES.contains(&WEB_ROUTE_ADMIN_PUBLISH_PLAN),
            "publish plan route must be implemented"
        );
        assert!(
            IMPLEMENTED_WEB_ROUTES.contains(&WEB_ROUTE_ADMIN_PUBLISH),
            "publish apply route must be implemented"
        );
    }

    #[test]
    fn delivery_routes_are_implemented_web_routes() {
        // Every staged delivery route is registered by the router and listed
        // as an implemented web route, so the recatalogued delivery rows can
        // never name a route that does not exist.
        for route in [
            WEB_ROUTE_ADMIN_DELIVERY_STATUS,
            WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT,
            WEB_ROUTE_ADMIN_DELIVERY_STAGE,
            WEB_ROUTE_ADMIN_DELIVERY_PROMOTE,
            WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY,
        ] {
            assert!(
                IMPLEMENTED_WEB_ROUTES.contains(&route),
                "delivery route {route} must be implemented"
            );
        }
    }

    #[test]
    fn web_execution_rows_are_well_formed_and_point_at_implemented_routes() {
        // Layer C: the catalog's `execution` block is the browser's runnable
        // contract. The project creation/registration rows plus the project
        // feature/spec/deploy lifecycle write rows carry one; every one must be a
        // `web` row resolving to a real implemented route with a well-formed typed
        // parameter list, and no non-`web` row may ever carry one.
        let executable_ids: Vec<&str> = rows()
            .iter()
            .filter(|row| row.execution.is_some())
            .map(|row| row.id.as_str())
            .collect();
        assert_eq!(
            executable_ids,
            vec![
                "register",
                "import",
                "new",
                "feature.add",
                "feature.remove",
                "feature.upgrade",
                "spec.generate",
                "spec.apply",
                "release.apply",
                "deploy.apply",
                "publish",
                "delivery.preflight",
                "delivery.stage",
                "delivery.promote",
                "delivery.hermora-retry",
            ]
        );
        // Each executable row's typed parameter list must match the mandatory
        // fields that route's `authoring_descriptor` gate accepts, in the exact
        // order, so the browser's generated controls are the route's real input
        // surface and nothing more.
        let expected: BTreeMap<&str, Vec<(&str, &str, bool)>> = BTreeMap::from([
            ("register", vec![("project", "string", true)]),
            (
                "import",
                vec![
                    ("project", "string", true),
                    ("profile", "string", false),
                    ("id", "string", false),
                ],
            ),
            (
                "new",
                vec![
                    ("project", "string", true),
                    ("profile", "string", true),
                    ("name", "string", false),
                    ("features", "string_array", false),
                ],
            ),
            (
                "feature.add",
                vec![("feature", "string", true), ("version", "string", false)],
            ),
            ("feature.remove", vec![("feature", "string", true)]),
            (
                "feature.upgrade",
                vec![("feature", "string", true), ("version", "string", false)],
            ),
            (
                "spec.generate",
                vec![
                    ("findings", "string_array", true),
                    ("reason", "string", false),
                ],
            ),
            (
                "spec.apply",
                vec![
                    ("findings", "string_array", true),
                    ("reason", "string", false),
                ],
            ),
            ("release.apply", vec![("version", "string", true)]),
            ("deploy.apply", vec![("target", "string", false)]),
            // The bare publish resolves the provider, project and revision
            // server-side, so it has no browser-supplied typed parameters.
            ("publish", vec![]),
            // Delivery preflight likewise resolves every input server-side.
            ("delivery.preflight", vec![]),
            (
                "delivery.stage",
                vec![("confirm_operation_id", "string", true)],
            ),
            (
                "delivery.promote",
                vec![("confirm_revision", "string", true)],
            ),
            (
                "delivery.hermora-retry",
                vec![
                    ("deployment_url", "string", true),
                    ("secret_ref", "string", true),
                ],
            ),
        ]);
        for row in rows() {
            match &row.execution {
                Some(execution) => {
                    assert_eq!(
                        row.availability, "web",
                        "execution row {} must be web",
                        row.id
                    );
                    assert!(
                        IMPLEMENTED_WEB_ROUTES.contains(&execution.route),
                        "execution row {} names unimplemented route {}",
                        row.id,
                        execution.route
                    );
                    assert_eq!(
                        execution.route,
                        row.route.expect("web row route"),
                        "execution row {} route disagrees with row route",
                        row.id
                    );
                    assert_eq!(execution.method, "POST", "row {}", row.id);
                    assert!(execution.confirm_required, "row {}", row.id);
                    assert!(execution.digest_bound, "row {}", row.id);
                    assert!(
                        !execution.parameters.is_empty()
                            || matches!(row.id.as_str(), "publish" | "delivery.preflight"),
                        "row {} must carry typed parameters unless every input is server-resolved",
                        row.id
                    );
                    // Pin the exact typed parameter list against the route.
                    let expected_params = expected
                        .get(row.id.as_str())
                        .unwrap_or_else(|| panic!("unexpected execution row {}", row.id));
                    let actual: Vec<(&str, &str, bool)> = execution
                        .parameters
                        .iter()
                        .map(|p| (p.name.as_str(), p.kind, p.required))
                        .collect();
                    assert_eq!(
                        &actual, expected_params,
                        "row {} parameters disagree with the route's typed fields",
                        row.id
                    );
                    for param in &execution.parameters {
                        assert!(!param.name.is_empty(), "row {} empty param", row.id);
                        assert!(
                            matches!(param.kind, "string" | "string_array" | "boolean"),
                            "row {} parameter {} has unknown kind {}",
                            row.id,
                            param.name,
                            param.kind
                        );
                    }
                }
                None => {
                    assert!(
                        row.availability == "web" || row.execution.is_none(),
                        "non-web row {} must not carry execution",
                        row.id
                    );
                }
            }
        }
        // The execution block is purely additive: it must not introduce any
        // catalog integrity problem.
        let issues = problems();
        assert!(issues.is_empty(), "catalog problems: {issues:?}");
    }

    #[test]
    fn ids_are_unique_and_parented_rows_follow_dot_paths() {
        let mut seen = BTreeSet::new();
        for row in rows() {
            assert!(seen.insert(row.id.as_str()), "duplicate {}", row.id);
            match &row.parent_id {
                Some(parent) => assert!(
                    row.id.starts_with(&format!("{parent}.")),
                    "row {} is not a child of {}",
                    row.id,
                    parent
                ),
                None => assert!(!row.id.contains('.'), "top row {} is dotted", row.id),
            }
        }
    }
}
