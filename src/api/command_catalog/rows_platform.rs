//! Command catalog rows: platform services.
//! Auto-generated module split by row family; row order is preserved verbatim.

use super::builder::CatalogBuilder;
use super::model::{Availability, Category, Risk, Scope};
use super::routes::{
    REASON_AGENT, REASON_HELP, REASON_LEGACY_HTML, REASON_LOCAL_FS, REASON_LOCAL_SECRET,
    REASON_TRANSPORT, REASON_TTY_HIDDEN,
};
use super::routes::{
    WEB_ROUTE_ADMIN_AGENT_LIST, WEB_ROUTE_ADMIN_AGENT_STATUS, WEB_ROUTE_ADMIN_ANALYTICS_METRICS,
    WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY, WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT,
    WEB_ROUTE_ADMIN_DELIVERY_PROMOTE, WEB_ROUTE_ADMIN_DELIVERY_STAGE,
    WEB_ROUTE_ADMIN_DELIVERY_STATUS, WEB_ROUTE_ADMIN_IDENTITY_CONFIG,
    WEB_ROUTE_ADMIN_IDENTITY_SESSIONS, WEB_ROUTE_ADMIN_IDENTITY_SESSION_INSPECT,
    WEB_ROUTE_ADMIN_STUDIO_PREVIEW, WEB_ROUTE_ADMIN_STUDIO_REFINE,
    WEB_ROUTE_ADMIN_STUDIO_SPEC_SAVE,
};

impl CatalogBuilder {
    pub(super) fn platform_rows(&mut self) {
        use Availability::*;
        use Category::*;
        use Risk::*;
        use Scope::*;
        let caps_registry: &[&str] = &["registry_read"];
        let caps_registry_write: &[&str] = &["registry_read", "registry_write"];
        let caps_local: &[&str] = &["local_filesystem"];
        let caps_provider: &[&str] = &["external_provider"];
        let caps_loopback: &[&str] = &["loopback_bind"];
        let none: &[&str] = &[];
        self.group(
            None,
            "analytics",
            "Inspect the existing content / analytics providers configured for a project and aggregate timestamped project metrics.",
            Portfolio,
            Project,
        );
        self.provider_required(
            Some("analytics"),
            "inspect",
            "Inspect the manifest's analytics block, recording timestamped health observations for each configured provider.",
            Portfolio,
            Project,
            Read,
            caps_provider,
        );
        self.web_at(
            Some("analytics"),
            "metrics",
            "Aggregate timestamped project metrics from the registry, doctor, deploy and external observations.",
            Portfolio,
            Project,
            Read,
            WEB_ROUTE_ADMIN_ANALYTICS_METRICS,
            caps_registry,
        );
        self.group(
            None,
            "delivery",
            "Coordinate the staged delivery workflow (preflight → stage → production) and the optional post-deploy Hermora onboarding.",
            Delivery,
            Project,
        );
        self.web_at(
            Some("delivery"),
            "status",
            "Read the project's current delivery phase, revision and most-recent per-verb journal evidence.",
            Delivery,
            Project,
            Read,
            WEB_ROUTE_ADMIN_DELIVERY_STATUS,
            caps_registry,
        );
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
        self.web_exec(
            Some("delivery"),
            "stage",
            "Invoke the provider's `publish` for the stage environment (requires `confirm_operation_id` from a healthy preflight).",
            Delivery,
            Project,
            RemoteWrite,
            WEB_ROUTE_ADMIN_DELIVERY_STAGE,
            "POST",
            &[("confirm_operation_id", "string", true)],
            caps_provider,
        );
        self.web_exec(
            Some("delivery"),
            "promote",
            "Invoke the provider's `publish` for production (requires `confirm_revision` matching the registered source revision).",
            Delivery,
            Project,
            RemoteWrite,
            WEB_ROUTE_ADMIN_DELIVERY_PROMOTE,
            "POST",
            &[("confirm_revision", "string", true)],
            caps_provider,
        );
        self.web_exec(
            Some("delivery"),
            "hermora-retry",
            "Retry the optional Hermora site onboarding for a healthy deployment; never republishes.",
            Delivery,
            Project,
            RemoteWrite,
            WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY,
            "POST",
            &[("deployment_url", "string", true), ("secret_ref", "string", true)],
            caps_provider,
        );
        self.group(
            None,
            "studio",
            "Site Studio: review an AppSpec, run a bounded preview, and journal scoped refinement requests (`forge-studio-preview-refinement`).",
            Delivery,
            Project,
        );
        self.web_exec(
            Some("studio"),
            "spec",
            "Validate an AppSpec from a YAML file and print the closed `forge-app-spec/0.1.0` envelope; persists only with `--confirm yes`.",
            Delivery,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_STUDIO_SPEC_SAVE,
            "POST",
            &[("spec", "string", true), ("expected_revision", "string", false)],
            caps_local,
        );
        self.web_at(
            Some("studio"),
            "preview",
            "Read the current Studio session record; `--start`/`--stop` bind the reserved port only with a confirmation token.",
            Delivery,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_STUDIO_PREVIEW,
            caps_local,
        );
        self.web_exec(
            Some("studio"),
            "refine",
            "Submit a refinement request; validates, journals a `studio.refine` row and bumps `app_revision`.",
            Delivery,
            Project,
            LocalWrite,
            WEB_ROUTE_ADMIN_STUDIO_REFINE,
            "POST",
            &[
                ("request", "string", true),
                ("expected_revision", "string", true),
                ("selected_files", "string_array", false),
            ],
            caps_registry_write,
        );
        self.group(
            None,
            "identity",
            "Validate, challenge, complete and terminate per-project OIDC admin sessions.",
            Identity,
            Project,
        );
        self.cli_only(
            Some("identity"),
            "setup",
            "Initialize the one Forge-wide portal administrator; the password is read without terminal echo, or from stdin with `--password-stdin`.",
            Identity,
            Forge,
            SessionAdmin,
            REASON_TTY_HIDDEN,
            none,
        );
        self.cli_only(
            Some("identity"),
            "change-password",
            "Replace the Forge-wide administrator password without changing the email; revokes every active browser session (new password read without terminal echo, or from stdin with `--password-stdin`).",
            Identity,
            Forge,
            SessionAdmin,
            REASON_TTY_HIDDEN,
            none,
        );
        self.cli_only(
            Some("identity"),
            "generate-password",
            "Print one strong random password from operating-system entropy without reading or writing the registry.",
            Identity,
            Forge,
            Read,
            REASON_LOCAL_SECRET,
            none,
        );
        self.cli_only(
            Some("identity"),
            "status",
            "Report whether the one Forge-wide administrator is configured and the stored email (never the password); reads the local registry directly.",
            Identity,
            Forge,
            Read,
            REASON_LOCAL_FS,
            caps_local,
        );
        self.web_at(
            Some("identity"),
            "validate-config",
            "Validate the manifest's `identity:` block without contacting any provider.",
            Identity,
            Project,
            Read,
            WEB_ROUTE_ADMIN_IDENTITY_CONFIG,
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
        self.cli_only(
            Some("identity"),
            "complete-auth",
            "Complete the OIDC round trip from a provider callback + claims and mint a per-project admin session.",
            Identity,
            Project,
            SessionAdmin,
            REASON_TTY_HIDDEN,
            none,
        );
        self.web_at(
            Some("identity"),
            "session-list",
            "List every persisted admin session for the named project.",
            Identity,
            Project,
            Read,
            WEB_ROUTE_ADMIN_IDENTITY_SESSIONS,
            caps_local,
        );
        self.web_at(
            Some("identity"),
            "session-inspect",
            "Inspect one persisted admin session.",
            Identity,
            Project,
            Read,
            WEB_ROUTE_ADMIN_IDENTITY_SESSION_INSPECT,
            caps_local,
        );
        self.cli_only(
            Some("identity"),
            "session-validate",
            "Validate a session id against the named project and permission (read-only diagnostic).",
            Identity,
            Project,
            Read,
            REASON_LOCAL_FS,
            caps_local,
        );
        self.cli_only(
            Some("identity"),
            "session-terminate",
            "Terminate the named admin session and remove its persisted state.",
            Identity,
            Project,
            SessionAdmin,
            REASON_LOCAL_FS,
            caps_local,
        );
        self.group(
            None,
            "api",
            "Serve the optional HTTP transport over Core on a loopback listener.",
            Transports,
            Forge,
        );
        self.cli_only(
            Some("api"),
            "serve",
            "Run the HTTP/1.1 API server on a loopback listener; authorization is required for every route other than `/healthz`.",
            Transports,
            Forge,
            Read,
            REASON_TRANSPORT,
            caps_loopback,
        );
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
        self.cli_only(
            Some("portal"),
            "dashboard",
            "Render the top-level control-plane dashboard for one project or the whole registry (server-side HTML).",
            Transports,
            Forge,
            Read,
            REASON_LEGACY_HTML,
            none,
        );
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
        self.web_at(
            Some("agent"),
            "status",
            "Show the current agent session status for the project.",
            Creation,
            Project,
            Read,
            WEB_ROUTE_ADMIN_AGENT_STATUS,
            caps_registry,
        );
        self.web_at(
            Some("agent"),
            "list",
            "List the agent sessions Forge knows about.",
            Creation,
            Project,
            Read,
            WEB_ROUTE_ADMIN_AGENT_LIST,
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
