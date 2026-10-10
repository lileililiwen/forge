//! Command catalog rows: spec and governance review.
//! Auto-generated module split by row family; row order is preserved verbatim.

use super::builder::CatalogBuilder;
use super::model::{Availability, Category, Risk, Scope};
use super::routes::REASON_FILE_STDIN;
use super::routes::{
    WEB_ROUTE_ADMIN_CLASSIFY_LIST, WEB_ROUTE_ADMIN_CLASSIFY_SHOW, WEB_ROUTE_ADMIN_CONTRACTS,
    WEB_ROUTE_ADMIN_CONTRACT_EMIT, WEB_ROUTE_ADMIN_CONTRACT_INSPECT, WEB_ROUTE_ADMIN_DESCRIBE_LIST,
    WEB_ROUTE_ADMIN_DESCRIBE_SHOW, WEB_ROUTE_ADMIN_GOVERNANCE_INSPECT,
    WEB_ROUTE_ADMIN_GOVERNANCE_LIST, WEB_ROUTE_ADMIN_GOVERNANCE_STATUS,
    WEB_ROUTE_ADMIN_REMEDIATE_APPLY, WEB_ROUTE_ADMIN_REMEDIATE_DIFF,
    WEB_ROUTE_ADMIN_REMEDIATE_PLAN, WEB_ROUTE_ADMIN_REMEDIATE_SCAN, WEB_ROUTE_ADMIN_SPEC,
    WEB_ROUTE_ADMIN_SPECS, WEB_ROUTE_ADMIN_SPEC_APPLY, WEB_ROUTE_ADMIN_SPEC_INSPECT,
    WEB_ROUTE_ADMIN_SPEC_ROUTE, WEB_ROUTE_CLASSIFY_APPLY, WEB_ROUTE_CLASSIFY_APPROVE,
    WEB_ROUTE_CLASSIFY_REJECT,
};

impl CatalogBuilder {
    pub(super) fn assurance_rows(&mut self) {
        use Availability::*;
        use Category::*;
        use Risk::*;
        use Scope::*;
        let caps_local: &[&str] = &["local_filesystem"];
        let caps_provider: &[&str] = &["external_provider"];
        let none: &[&str] = &[];
        self.group(
            None,
            "spec",
            "Generate bounded spec proposals and route findings to deterministic, semantic or manual remediation.",
            Quality,
            Project,
        );
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
        self.web_at(
            Some("spec"),
            "list",
            "List all generated specs under `.forge/specs/` for the named project.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_SPECS,
            caps_local,
        );
        self.web_at(
            Some("spec"),
            "inspect",
            "Show the full bounded proposal for a generated spec.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_SPEC_INSPECT,
            caps_local,
        );
        self.web_at(
            Some("spec"),
            "route",
            "Classify one finding into a deterministic, semantic or manual route.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_SPEC_ROUTE,
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
            &[("findings", "string_array", true), ("reason", "string", false)],
            caps_local,
        );
        self.group(
            None,
            "remediate",
            "Plan and apply ownership-safe local remediation.",
            Quality,
            Project,
        );
        self.web_at(
            Some("remediate"),
            "scan",
            "Inspect automatic findings and produce a read-only plan.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_REMEDIATE_SCAN,
            caps_local,
        );
        self.web_exec(
            Some("remediate"),
            "plan",
            "Produce a versioned read-only remediation plan.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_REMEDIATE_PLAN,
            "POST",
            &[("finding", "string", true), ("pack", "string", false)],
            caps_local,
        );
        self.web_at(
            Some("remediate"),
            "diff",
            "Show the files a remediation plan would change.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_REMEDIATE_DIFF,
            caps_local,
        );
        self.web_exec(
            Some("remediate"),
            "apply",
            "Apply a plan after explicit confirmation.",
            Quality,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_REMEDIATE_APPLY,
            "POST",
            &[("finding", "string", true), ("pack", "string", false)],
            caps_local,
        );
        self.group(
            None,
            "describe",
            "Suggest, list, show, approve and reject semantic project descriptions (`forge-semantic-proposal/0.1.0`).",
            Quality,
            Project,
        );
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
        self.web_at(
            Some("describe"),
            "list",
            "List every recorded semantic proposal for the named project.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_DESCRIBE_LIST,
            caps_local,
        );
        self.web_at(
            Some("describe"),
            "show",
            "Show the full proposal manifest for one proposal id.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_DESCRIBE_SHOW,
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
        self.group(
            None,
            "classify",
            "Suggest, list, show, approve and reject semantic project classifications (domain, portfolio tags, profile, lifecycle).",
            Quality,
            Project,
        );
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
            "derive",
            "Derive classification proposals from repository evidence (no model, no network, no project-field write).",
            Quality,
            Project,
            LocalWrite,
            NotYetWeb,
            caps_local,
        );
        self.web_exec(
            Some("classify"),
            "apply",
            "Apply the approved classification set outward through the configured metadata plugin (PR mode only).",
            Quality,
            Project,
            RemoteWrite,
            WEB_ROUTE_CLASSIFY_APPLY,
            "POST",
            &[],
            caps_provider,
        );
        self.web_at(
            Some("classify"),
            "list",
            "List every recorded classification proposal for the named project.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_CLASSIFY_LIST,
            caps_local,
        );
        self.web_at(
            Some("classify"),
            "show",
            "Show the full proposal manifest for one proposal id.",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_CLASSIFY_SHOW,
            caps_local,
        );
        self.web_exec(
            Some("classify"),
            "approve",
            "Approve a `Suggested` classification proposal after preview and explicit confirmation.",
            Quality,
            Project,
            LocalWrite,
            WEB_ROUTE_CLASSIFY_APPROVE,
            "POST",
            &[("proposal", "string", true)],
            caps_local,
        );
        self.web_exec(
            Some("classify"),
            "reject",
            "Reject a `Suggested` classification proposal after preview and explicit confirmation.",
            Quality,
            Project,
            LocalWrite,
            WEB_ROUTE_CLASSIFY_REJECT,
            "POST",
            &[("proposal", "string", true)],
            caps_local,
        );
        self.group(
            None,
            "contract",
            "Vendor, inspect and project platform contracts into envelopes.",
            Quality,
            Forge,
        );
        self.web_at(
            Some("contract"),
            "list",
            "List every versioned surface and its platform mapping.",
            Quality,
            Forge,
            Read,
            WEB_ROUTE_ADMIN_CONTRACTS,
            none,
        );
        self.web_at(
            Some("contract"),
            "inspect",
            "Inspect one platform family and its schema.",
            Quality,
            Forge,
            Read,
            WEB_ROUTE_ADMIN_CONTRACT_INSPECT,
            none,
        );
        self.web_at(
            Some("contract"),
            "emit",
            "Project a Core record into a platform envelope (read-only).",
            Quality,
            Project,
            Read,
            WEB_ROUTE_ADMIN_CONTRACT_EMIT,
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
        self.group(
            None,
            "governance",
            "Inspect and select standalone or optional external governance providers.",
            Quality,
            Workspace,
        );
        self.web_at(
            Some("governance"),
            "list",
            "List the local provider and the configured optional provider.",
            Quality,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_GOVERNANCE_LIST,
            caps_local,
        );
        self.web_at(
            Some("governance"),
            "status",
            "Run the selected provider and record a bounded observation.",
            Quality,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_GOVERNANCE_STATUS,
            caps_local,
        );
        self.web_at(
            Some("governance"),
            "inspect",
            "Alias for status that returns the full normalized observation.",
            Quality,
            Workspace,
            Read,
            WEB_ROUTE_ADMIN_GOVERNANCE_INSPECT,
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
    }
}
