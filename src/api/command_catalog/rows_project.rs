//! Command catalog rows: project authoring.
//! Auto-generated module split by row family; row order is preserved verbatim.

use super::builder::CatalogBuilder;
use super::model::{Availability, Category, Risk, Scope};
use super::routes::{REASON_FILE_STDIN, REASON_GIT, REASON_LOCAL_FS, REASON_NATIVE};
use super::routes::{
    WEB_ROUTE_ADMIN_FEATURE, WEB_ROUTE_ADMIN_FEATURE_REMOVE, WEB_ROUTE_ADMIN_FEATURE_UPGRADE,
    WEB_ROUTE_ADMIN_PROJECT_IMPORT, WEB_ROUTE_ADMIN_PROJECT_NEW, WEB_ROUTE_ADMIN_PROJECT_REGISTER,
    WEB_ROUTE_PROJECT_DETAIL, WEB_ROUTE_PROJECT_PLAN, WEB_ROUTE_PROJECT_STATUS,
};

impl CatalogBuilder {
    pub(super) fn project_rows(&mut self) {
        use Availability::*;
        use Category::*;
        use Risk::*;
        use Scope::*;
        let caps_local: &[&str] = &["local_filesystem"];
        let caps_git: &[&str] = &["git"];
        let caps_native: &[&str] = &["native_toolchain", "local_filesystem"];
        let caps_web: &[&str] = &["admin_session", "projects_read"];
        let caps_workbench_write: &[&str] = &["admin_session", "project_upgrade"];
        let none: &[&str] = &[];
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
        self.group(
            None,
            "kit",
            "Repack or verify the committed shared-layer kit feed.",
            Creation,
            Forge,
        );
        self.cli_only(
            Some("kit"),
            "pack",
            "Repack the committed feed from a checked-out sibling library (nothing outside the repository is written).",
            Creation,
            Forge,
            LocalWrite,
            REASON_LOCAL_FS,
            caps_local,
        );
        self.cli_only(
            Some("kit"),
            "verify",
            "Check a committed feed against the kit version a project declares; this is the drift gate.",
            Creation,
            Project,
            Read,
            REASON_LOCAL_FS,
            caps_local,
        );
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
        self.leaf(
            Some("feature"),
            "resolve",
            "Resolve requested capabilities into a deterministic install plan without changing files.",
            Creation,
            Profile,
            Read,
            NotYetWeb,
            none,
        );
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
        self.leaf(
            Some("component"),
            "qualify",
            "Promote a component's quality level (evidence-gated; preserves the prior level on failure).",
            Creation,
            Profile,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
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
        self.leaf(
            Some("ui-pattern"),
            "install",
            "Install a UI pattern's ordinary source and write a metadata receipt (refuses to overwrite customized files).",
            Creation,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.group(
            None,
            "intent",
            "Validate a structured intent, resolve it into a deterministic assembly plan, and apply it with explicit confirmation.",
            Creation,
            Profile,
        );
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
        self.leaf(
            Some("intent"),
            "resolve",
            "Resolve a validated intent into a reviewable deterministic assembly plan and persist its receipt.",
            Creation,
            Profile,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
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
        self.leaf(
            Some("standard"),
            "upgrade",
            "Upgrade a project's snapshot to a pack version; refuses modified files without `--confirm`.",
            Creation,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
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
    }
}
