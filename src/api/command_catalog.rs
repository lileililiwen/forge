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

/// Routes a `web` row may honestly point at today. A row naming any other
/// route is a catalog bug and is reported by [`problems`].
const IMPLEMENTED_WEB_ROUTES: &[&str] = &[WEB_ROUTE_PROJECTS];

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
const REASON_MACHINE_STDOUT: &str = "This command emits a machine-pure stdout document for external tooling (DriftWatchdog-compatible); re-rendering it through a browser API would change the contract. Next step: pipe it to the consumer in a terminal.";
const REASON_TTY_HIDDEN: &str = "This command reads hidden terminal input (password without echo) or pastes manual provider-callback values; neither may ever cross a browser form or the JSON API. Next step: run it in a terminal.";
const REASON_LEGACY_HTML: &str = "The legacy portal renders server-side HTML sections; the standalone frontend (this page) replaced that surface for browsers. Next step: use this dashboard, or run it in a terminal for the HTML view.";
const REASON_AGENT: &str = "This command drives local agent adapter processes against a project directory (start, pause, takeover, resume, restart, new sessions); these are terminal-session operations with no typed JSON route. Next step: run it in a terminal.";
const REASON_LOCAL_TOOLCHAIN: &str = "This command shells out to a locally installed developer CLI (gh) and its credential store; the browser cannot reach that installation. Next step: install and authenticate the CLI, then run it in a terminal.";
const REASON_NOT_YET_WEB: &str = "A web workflow for this command is planned in the staged web packages (workbench, portfolio controls, delivery controls) but not implemented yet; it stays visible as a tracked gap, never labeled supported. Next step: use the CLI for now.";
const REASON_PUBLISH_BARE: &str = "Bare `forge publish` runs a real publish through the selected external provider immediately; provider credentials and the explicit confirmation stay in the terminal. Next step: run it in a terminal, or use `--dry-run` first.";

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

/// One catalog row: `{id,parent_id,label,summary,category,scope,risk,
/// availability,route,cli_invocation,reason,capabilities}`.
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
        self.leaf(
            None,
            "inspect",
            "Inspect one registered project by id or path.",
            Registry,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.cli_only(
            None,
            "register",
            "Validate the manifest in a directory and persist the project.",
            Registry,
            Workspace,
            LocalWrite,
            REASON_LOCAL_FS,
            caps_local,
        );
        self.cli_only(
            None,
            "import",
            "Inspect an existing repository and, on acceptance, adopt it.",
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
        self.cli_only(
            None,
            "new",
            "Create a new project deterministically from pinned profile assets.",
            Creation,
            Workspace,
            LocalWrite,
            REASON_LOCAL_FS,
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
        self.leaf(
            Some("feature"),
            "add",
            "Add a feature (plus missing dependencies) to a project.",
            Creation,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("feature"),
            "remove",
            "Remove a feature from a project.",
            Creation,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.leaf(
            Some("feature"),
            "upgrade",
            "Upgrade a feature to the tested catalog version.",
            Creation,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );

        self.leaf(
            None,
            "upgrade",
            "Plan and apply deterministic project/fleet upgrades with conflict handoff.",
            Creation,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_local,
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
        self.cli_only(
            None,
            "doctor",
            "Inspect project health and evidence-based maturity without changing files.",
            Quality,
            Project,
            Read,
            REASON_LOCAL_FS,
            caps_local,
        );
        self.cli_only(
            None,
            "check",
            "Emit a Driftwatchdog-compatible external-checker document on stdout (machine-pure, read-only).",
            Quality,
            Project,
            Read,
            REASON_MACHINE_STDOUT,
            caps_local,
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
        self.leaf(
            Some("spec"),
            "generate",
            "Generate a bounded spec for the named project and finding set.",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
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
        self.leaf(Some("spec"), "apply", "Apply a routing decision: deterministic action is recorded, semantic produces a spec, manual is noted.", Quality, Project, LocalWrite, NotYetWeb, caps_local);

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
        self.leaf(Some("release"), "prepare", "Capture semver, changelog, source revision and the doctor/test/DriftWatch evidence into a reviewable plan.", Release, Project, LocalWrite, NotYetWeb, caps_local);
        self.provider_required(Some("release"), "apply", "Apply a verified release plan: walks every stage with safe retry and per-stage records (tag, push, package, container, notes).", Release, Project, RemoteWrite, caps_git_remote);
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
        self.push(Some("deploy"), "plan", "Capture target, source revision and the configured health check into a reviewable plan.", Release, Project, Read, ProjectCapabilityRequired, None, Some(REASON_PROJECT_CAPABILITY), caps_local);
        self.push(Some("deploy"), "apply", "Apply a verified deploy plan: invokes the adapter and captures the health observation (requires `--confirm`).", Release, Project, RemoteWrite, ProjectCapabilityRequired, None, Some(REASON_PROJECT_CAPABILITY), caps_local);
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

        // publish — the group itself performs the bare publish, unlike other groups.
        self.push(
            None,
            "publish",
            "Publish a registered project to the Jenkins/Mac infrastructure with subdomain routing; bare `forge publish` publishes the discovered project.",
            Release,
            Project,
            RemoteWrite,
            ProviderRequired,
            None,
            Some(REASON_PUBLISH_BARE),
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
        self.web(
            Some("fleet"),
            "status",
            "Report the fleet registry health only (source, freshness, counts; no entries).",
            Registry,
            Workspace,
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
        self.leaf(Some("portfolio.share"), "set", "Define or replace one project's public share record; nothing is public until approved and published.", Portfolio, Workspace, LocalWrite, NotYetWeb, caps_registry_write);
        self.leaf(
            Some("portfolio.share"),
            "remove",
            "Withdraw a project's share record; it leaves the public catalog.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.share"),
            "show",
            "Show one project's share record and its allowlisted surfaces.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("portfolio.share"),
            "list",
            "List every share record, newest state first.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("portfolio.share"),
            "preview",
            "Preview the candidate manifest: exact canonical bytes, hash and findings (read-only).",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.leaf(
            Some("portfolio.share"),
            "approve",
            "Approve one exact manifest hash for publication.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(Some("portfolio.share"), "publish", "Publish the approved manifest through the default-safe local export or an optional adapter.", Portfolio, Workspace, RemoteWrite, NotYetWeb, caps_registry_write);
        self.leaf(
            Some("portfolio.share"),
            "reconcile",
            "Resolve a partial publication reported as `unknown`.",
            Portfolio,
            Workspace,
            LocalWrite,
            NotYetWeb,
            caps_registry_write,
        );
        self.leaf(
            Some("portfolio.share"),
            "audit",
            "Show the approval and publication audit trail.",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
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
        self.leaf(Some("delivery"), "status", "Read the project's current delivery phase, revision and most-recent per-verb journal evidence.", Delivery, Project, Read, NotYetWeb, caps_registry);
        self.provider_required(
            Some("delivery"),
            "preflight",
            "Invoke the publish provider's `preflight` operation and record the terminal evidence.",
            Delivery,
            Project,
            LocalWrite,
            caps_provider,
        );
        self.provider_required(Some("delivery"), "stage", "Invoke the provider's `publish` for the stage environment (requires `--confirm-operation-id` from a healthy preflight).", Delivery, Project, RemoteWrite, caps_provider);
        self.provider_required(Some("delivery"), "promote", "Invoke the provider's `publish` for production (requires `--confirm-revision` matching the registered source revision).", Delivery, Project, RemoteWrite, caps_provider);
        self.provider_required(Some("delivery"), "hermora-retry", "Retry the optional Hermora site onboarding for a healthy deployment; never republishes.", Delivery, Project, RemoteWrite, caps_provider);

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
    use super::*;

    #[test]
    fn catalog_has_no_integrity_problems() {
        let issues = problems();
        assert!(issues.is_empty(), "catalog problems: {issues:?}");
    }

    #[test]
    fn catalog_covers_every_clap_path() {
        // The row count equals the 224 Clap paths (probe-verified from
        // `Cli::command()`) plus the explicit top-level `help` row.
        assert_eq!(rows().len(), 225);
        assert!(rows().iter().any(|row| row.id == "help"));
    }

    #[test]
    fn web_rows_only_point_at_the_projects_route() {
        for row in rows().iter().filter(|row| row.availability == "web") {
            assert_eq!(row.route, Some(WEB_ROUTE_PROJECTS), "row {}", row.id);
        }
        let web_ids: Vec<&str> = rows()
            .iter()
            .filter(|row| row.availability == "web")
            .map(|row| row.id.as_str())
            .collect();
        assert_eq!(
            web_ids,
            vec!["list", "fleet.list", "fleet.status", "inventory.show"]
        );
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
