//! Command catalog rows: release and delivery.
//! Auto-generated module split by row family; row order is preserved verbatim.

use super::builder::CatalogBuilder;
use super::model::{Availability, Category, Risk, Scope};
use super::routes::{REASON_GIT, REASON_NATIVE, REASON_PROJECT_CAPABILITY};
use super::routes::{
    WEB_ROUTE_ADMIN_DEPLOY, WEB_ROUTE_ADMIN_DEPLOY_HISTORY, WEB_ROUTE_ADMIN_DEPLOY_INSPECT,
    WEB_ROUTE_ADMIN_DEPLOY_PLAN, WEB_ROUTE_ADMIN_DEPLOY_STATUS, WEB_ROUTE_ADMIN_PUBLISH,
    WEB_ROUTE_ADMIN_RELEASE, WEB_ROUTE_ADMIN_RELEASE_HISTORY, WEB_ROUTE_ADMIN_RELEASE_INSPECT,
    WEB_ROUTE_ADMIN_RELEASE_PLAN,
};
use super::routes::{
    WEB_ROUTE_ADMIN_EVIDENCE_MATRIX, WEB_ROUTE_ADMIN_EVIDENCE_PROVIDER_INSPECT,
    WEB_ROUTE_ADMIN_PUBLISH_PROVIDERS, WEB_ROUTE_ADMIN_PUBLISH_PROVIDER_INSPECT,
    WEB_ROUTE_ADMIN_SHIPPING_PLUGINS,
};

impl CatalogBuilder {
    pub(super) fn shipping_rows(&mut self) {
        use Availability::*;
        use Category::*;
        use Risk::*;
        use Scope::*;
        let caps_registry: &[&str] = &["registry_read"];
        let caps_local: &[&str] = &["local_filesystem"];
        let caps_git_remote: &[&str] = &["git", "network"];
        let caps_native: &[&str] = &["native_toolchain", "local_filesystem"];
        let caps_provider: &[&str] = &["external_provider"];
        let none: &[&str] = &[];
        self.group(
            None,
            "release",
            "Prepare, apply, list and inspect gated resumable releases.",
            Release,
            Project,
        );
        self.web_at(
            Some("release"),
            "prepare",
            "Capture semver, changelog, source revision and the doctor/test/DriftWatch evidence into a reviewable plan.",
            Release,
            Project,
            Read,
            WEB_ROUTE_ADMIN_RELEASE_PLAN,
            caps_local,
        );
        self.web_exec(
            Some("release"),
            "apply",
            "Apply a verified release plan: walks every stage with safe retry and per-stage records (tag, push, package, container, notes).",
            Release,
            Project,
            RemoteWrite,
            WEB_ROUTE_ADMIN_RELEASE,
            "POST",
            &[("version", "string", true)],
            caps_git_remote,
        );
        self.web_at(
            Some("release"),
            "list",
            "List all persisted releases under the project's release directory.",
            Release,
            Project,
            Read,
            WEB_ROUTE_ADMIN_RELEASE_HISTORY,
            caps_local,
        );
        self.web_at(
            Some("release"),
            "inspect",
            "Inspect a single persisted release record.",
            Release,
            Project,
            Read,
            WEB_ROUTE_ADMIN_RELEASE_INSPECT,
            caps_local,
        );
        self.group(
            None,
            "deploy",
            "Plan, apply, observe and inspect adapter-based deployments.",
            Release,
            Project,
        );
        self.web_at(
            Some("deploy"),
            "plan",
            "Capture target, source revision and the configured health check into a reviewable plan.",
            Release,
            Project,
            Read,
            WEB_ROUTE_ADMIN_DEPLOY_PLAN,
            caps_local,
        );
        self.web_exec(
            Some("deploy"),
            "apply",
            "Apply a verified deploy plan: invokes the adapter and captures the health observation (requires `--confirm`).",
            Release,
            Project,
            RemoteWrite,
            WEB_ROUTE_ADMIN_DEPLOY,
            "POST",
            &[("target", "string", false)],
            caps_local,
        );
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
        self.web_at(
            Some("deploy"),
            "list",
            "List all persisted deploys for the named project.",
            Release,
            Project,
            Read,
            WEB_ROUTE_ADMIN_DEPLOY_HISTORY,
            caps_registry,
        );
        self.web_at(
            Some("deploy"),
            "inspect",
            "Inspect a single persisted deploy by its id.",
            Release,
            Project,
            Read,
            WEB_ROUTE_ADMIN_DEPLOY_INSPECT,
            caps_registry,
        );
        self.web_at(
            Some("deploy"),
            "status",
            "Show the persisted Forge publish/deploy status without contacting a provider.",
            Release,
            Project,
            Read,
            WEB_ROUTE_ADMIN_DEPLOY_STATUS,
            caps_registry,
        );
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
        self.group(
            Some("publish"),
            "provider",
            "Inspect or switch standalone publish providers.",
            Release,
            Provider,
        );
        self.web_at(
            Some("publish.provider"),
            "list",
            "List configured providers and their enabled state.",
            Release,
            Provider,
            Read,
            WEB_ROUTE_ADMIN_PUBLISH_PROVIDERS,
            none,
        );
        self.web_at(
            Some("publish.provider"),
            "inspect",
            "Inspect one configured provider without invoking it.",
            Release,
            Provider,
            Read,
            WEB_ROUTE_ADMIN_PUBLISH_PROVIDER_INSPECT,
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
        self.group(
            None,
            "docs",
            "Translate the canonical documentation into enabled derivative locales.",
            Release,
            Project,
        );
        self.provider_required(
            Some("docs"),
            "translate",
            "Translate the canonical source document into one enabled locale (or every enabled locale with `--all`).",
            Release,
            Project,
            LocalWrite,
            caps_provider,
        );
        self.group(
            None,
            "readiness",
            "Generate the supported profile fixtures natively and evaluate release readiness.",
            Release,
            Forge,
        );
        self.cli_only(
            Some("readiness"),
            "matrix",
            "Generate every supported profile fixture into a disposable directory and run its native build/test without Forge on PATH.",
            Release,
            Forge,
            LocalWrite,
            REASON_NATIVE,
            caps_native,
        );
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
        self.group(
            None,
            "provider",
            "Record opt-in controlled evidence for the external provider boundaries.",
            Release,
            Provider,
        );
        self.web_at(
            Some("provider"),
            "matrix",
            "Report the provider matrix; without `--live` every row is `not-run` and never claims support.",
            Release,
            Provider,
            Read,
            WEB_ROUTE_ADMIN_EVIDENCE_MATRIX,
            none,
        );
        self.provider_required(
            Some("provider"),
            "run",
            "Drive one controlled round trip for a provider with provenance and teardown (live rows require FORGE_PROVIDER_LIVE=1).",
            Release,
            Provider,
            LocalWrite,
            caps_provider,
        );
        self.web_at(
            Some("provider"),
            "inspect",
            "Describe one provider's boundary, binary override, secret and teardown rules without probing.",
            Release,
            Provider,
            Read,
            WEB_ROUTE_ADMIN_EVIDENCE_PROVIDER_INSPECT,
            none,
        );
        self.group(
            None,
            "plugins",
            "List every configured plugin (GitHub, OpenPanel, any future remote) with its kind, enabled state and declared capabilities.",
            Release,
            Provider,
        );
        self.web_at(
            Some("plugins"),
            "list",
            "List every configured plugin with its kind, state and capabilities.",
            Release,
            Provider,
            Read,
            WEB_ROUTE_ADMIN_SHIPPING_PLUGINS,
            none,
        );
    }
}
