//! Command catalog rows: fleet and registry.
//! Auto-generated module split by row family; row order is preserved verbatim.

use super::builder::CatalogBuilder;
use super::model::{Availability, Category, Risk, Scope};
use super::routes::REASON_LOCAL_TOOLCHAIN;
use super::routes::{
    WEB_ROUTE_ADMIN_PORTFOLIO_EVIDENCE_IMPORT, WEB_ROUTE_ADMIN_PORTFOLIO_GOALS,
    WEB_ROUTE_ADMIN_PORTFOLIO_PROJECT, WEB_ROUTE_ADMIN_PORTFOLIO_READ_EVIDENCE,
    WEB_ROUTE_ADMIN_PORTFOLIO_READ_GOALS, WEB_ROUTE_ADMIN_PORTFOLIO_READ_RELATIONS,
    WEB_ROUTE_ADMIN_PORTFOLIO_READ_REVIEWS, WEB_ROUTE_ADMIN_PORTFOLIO_READ_TAGS,
    WEB_ROUTE_ADMIN_PORTFOLIO_RELATIONS, WEB_ROUTE_ADMIN_PORTFOLIO_RELATION_REMOVE,
    WEB_ROUTE_ADMIN_PORTFOLIO_REVIEWS, WEB_ROUTE_ADMIN_PORTFOLIO_TAGS,
    WEB_ROUTE_ADMIN_PORTFOLIO_TAG_REMOVE, WEB_ROUTE_DELIVERY_ALLOWLIST,
    WEB_ROUTE_DELIVERY_ALLOWLIST_REMOVE, WEB_ROUTE_DELIVERY_APPROVE, WEB_ROUTE_DELIVERY_OVERVIEW,
    WEB_ROUTE_DELIVERY_PREVIEW, WEB_ROUTE_DELIVERY_PUBLISH, WEB_ROUTE_DELIVERY_RECONCILE,
    WEB_ROUTE_FLEET_STATUS,
};

impl CatalogBuilder {
    pub(super) fn fleet_rows(&mut self) {
        use Availability::*;
        use Category::*;
        use Risk::*;
        use Scope::*;
        let caps_registry: &[&str] = &["registry_read"];
        let caps_registry_write: &[&str] = &["registry_read", "registry_write"];
        let caps_provider: &[&str] = &["external_provider"];
        let caps_provider_gh: &[&str] = &["external_provider", "github_cli"];
        let caps_web: &[&str] = &["admin_session", "projects_read"];
        self.group(
            None,
            "fleet",
            "Observe the portfolio declared by an external workspace registry, read-only.",
            Registry,
            Workspace,
        );
        self.web(
            Some("fleet"),
            "list",
            "List the portfolio declared by the workspace registry as timestamped fleet observations.",
            Registry,
            Workspace,
            caps_web,
        );
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
        self.leaf(
            Some("fleet"),
            "inspect",
            "Inspect one declared fleet entry by id (read-only; unmanaged entries can never be operated on through the mirror).",
            Registry,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
        self.provider_required(
            Some("fleet"),
            "online",
            "Probe the served router rules and target containers to verdict whether every routed host is ONLINE/DOWN/NO-ROUTE/NOT-DEPLOYED (read-only).",
            Registry,
            Provider,
            Read,
            caps_provider,
        );
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
        self.group(
            Some("project"),
            "github",
            "Optional GitHub metadata observation and approved change surface (`github-project-metadata-adapter`).",
            Registry,
            Provider,
        );
        self.provider_required(
            Some("project.github"),
            "observe",
            "Observe one or more GitHub repositories through the configured adapter (`forge-github-metadata/0.1.0`); read-only.",
            Registry,
            Provider,
            Read,
            caps_provider_gh,
        );
        self.provider_required(
            Some("project.github"),
            "propose",
            "Propose an approved metadata change through the adapter (pull request by default; direct mode needs an echoed confirmation).",
            Registry,
            Provider,
            RemoteWrite,
            caps_provider_gh,
        );
        self.cli_only(
            Some("project.github"),
            "auth",
            "Probe the installed GitHub CLI authentication state without reading or displaying the credential.",
            Registry,
            Provider,
            Read,
            REASON_LOCAL_TOOLCHAIN,
            caps_provider_gh,
        );
        self.cli_only(
            Some("project.github"),
            "clone",
            "Clone an existing GitHub repository through the installed `gh` CLI to a local destination (requires `--confirm`).",
            Registry,
            Project,
            LocalWrite,
            REASON_LOCAL_TOOLCHAIN,
            caps_provider_gh,
        );
        self.provider_required(
            Some("project.github"),
            "create",
            "Create a remote GitHub repository for an existing local project (private default; public and source push need explicit confirmations).",
            Registry,
            Provider,
            RemoteWrite,
            caps_provider_gh,
        );
        self.provider_required(
            Some("project.github"),
            "pull-request",
            "Open a draft pull request for a registered project through the installed `gh` CLI (requires `--confirm`).",
            Registry,
            Provider,
            RemoteWrite,
            caps_provider_gh,
        );
        self.group(
            None,
            "inventory",
            "Load, validate and report a portable project inventory (`forge-project-inventory/0.1.0`); read-only.",
            Registry,
            Forge,
        );
        self.web(
            Some("inventory"),
            "show",
            "Load, validate and project an inventory into the fleet classification report (never invokes a provider or mutates the registry).",
            Registry,
            Forge,
            caps_web,
        );
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
        self.web_at(
            Some("portfolio.tag"),
            "add",
            "Attach a tag to a project, creating the tag on first use.",
            Portfolio,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PORTFOLIO_TAGS,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.tag"),
            "remove",
            "Detach a tag from a project; the tag itself is kept.",
            Portfolio,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PORTFOLIO_TAG_REMOVE,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.tag"),
            "list",
            "List every tag, or the tags on one project.",
            Portfolio,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_PORTFOLIO_READ_TAGS,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "relation",
            "Manage user-owned relationships between projects.",
            Portfolio,
            Workspace,
        );
        self.web_at(
            Some("portfolio.relation"),
            "add",
            "Link two projects; the same link twice stays one row.",
            Portfolio,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PORTFOLIO_RELATIONS,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.relation"),
            "remove",
            "Remove one declared relation.",
            Portfolio,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PORTFOLIO_RELATION_REMOVE,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.relation"),
            "list",
            "List relations touching one project, or every relation.",
            Portfolio,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_PORTFOLIO_READ_RELATIONS,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "review",
            "Record a review decision and the current classification.",
            Portfolio,
            Workspace,
        );
        self.web_at(
            Some("portfolio.review"),
            "set",
            "Record a review decision and optionally the lifecycle, next action and blocker for one project.",
            Portfolio,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PORTFOLIO_REVIEWS,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.review"),
            "list",
            "List the review history for one project.",
            Portfolio,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_PORTFOLIO_READ_REVIEWS,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "goal",
            "Manage user-owned portfolio goals.",
            Portfolio,
            Workspace,
        );
        self.web_at(
            Some("portfolio.goal"),
            "add",
            "Create a goal; re-running with the same title updates it.",
            Portfolio,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PORTFOLIO_GOALS,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.goal"),
            "link",
            "Attach a project to a goal, creating the goal when new.",
            Portfolio,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PORTFOLIO_GOALS,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.goal"),
            "list",
            "List every goal with its projects.",
            Portfolio,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_PORTFOLIO_READ_GOALS,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "evidence",
            "Import source-owned observations as append-only snapshots.",
            Portfolio,
            Workspace,
        );
        self.web_at(
            Some("portfolio.evidence"),
            "import",
            "Append one source-owned observation as a snapshot (redacted before storage, never rewritten).",
            Portfolio,
            Workspace,
            LocalWrite,
            WEB_ROUTE_ADMIN_PORTFOLIO_EVIDENCE_IMPORT,
            caps_registry_write,
        );
        self.web_at(
            Some("portfolio.evidence"),
            "list",
            "List every snapshot for one project, newest first.",
            Portfolio,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_PORTFOLIO_READ_EVIDENCE,
            caps_registry,
        );
        self.web_at(
            Some("portfolio"),
            "show",
            "Show one project's whole portfolio projection.",
            Portfolio,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_PORTFOLIO_PROJECT,
            caps_registry,
        );
        self.group(
            Some("portfolio"),
            "share",
            "Manage the explicit allowlist of projects that may be published.",
            Portfolio,
            Workspace,
        );
        self.web_at(
            Some("portfolio.share"),
            "set",
            "Define or replace one project's public share record; nothing is public until approved and published.",
            Portfolio,
            Workspace,
            Risk::LocalWrite,
            WEB_ROUTE_DELIVERY_ALLOWLIST,
            caps_registry_write,
        );
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
        self.web_at(
            Some("portfolio.share"),
            "publish",
            "Publish the approved manifest through the default-safe local export; the optional adapter stays CLI-only and the browser never names a path.",
            Portfolio,
            Workspace,
            Risk::RemoteWrite,
            WEB_ROUTE_DELIVERY_PUBLISH,
            caps_registry_write,
        );
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
        self.group(
            Some("portfolio"),
            "activation",
            "Read-only readiness verdict on whether aggregate evidence justifies the activation follow-up. No billing.",
            Portfolio,
            Workspace,
        );
        self.leaf(
            Some("portfolio.activation"),
            "readiness",
            "Verdict on whether a project's aggregate interest evidence justifies the product-owned activation follow-up (read-only, no billing).",
            Portfolio,
            Workspace,
            Read,
            NotYetWeb,
            caps_registry,
        );
    }
}
