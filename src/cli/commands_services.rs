//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use clap::Subcommand;
use forge::catalog::{
    DEFAULT_LIMIT as CATALOG_DEFAULT_LIMIT,
    DEFAULT_MAX_AGE_SECONDS as CATALOG_DEFAULT_MAX_AGE_SECONDS,
};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum ApiCommands {
    /// Run the HTTP/1.1 API server on a loopback listener. Authorization is required for every route other than the local `/healthz` health check.
    Serve {
        /// Bind address (default `127.0.0.1`; an explicit `0.0.0.0` is the operator's choice and is never the default).
        #[arg(long)]
        bind: Option<std::net::IpAddr>,
        /// TCP port (default `8765`).
        #[arg(long)]
        port: Option<u16>,
        /// Maximum request body size in bytes (default 1 MiB).
        #[arg(long)]
        max_body_bytes: Option<usize>,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum McpCommands {
    /// Run the JSON-RPC 2.0 stdio server until stdin closes.
    Serve,
}
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum IdentityCommands {
    /// Initialize the one Forge-wide portal administrator (password is read without terminal echo).
    Setup {
        /// Administrator email used by the Forge browser login.
        #[arg(long)]
        email: String,
        /// Read exactly one password line from stdin instead of prompting; no terminal or confirmation is required.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Replace the Forge-wide administrator password without changing the email; revokes every active browser session (new password read without terminal echo).
    ChangePassword {
        /// Read exactly one password line from stdin instead of prompting; no terminal or confirmation is required.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Print one strong random password from operating-system entropy without reading or writing the registry.
    GeneratePassword {
        /// Password length in characters (default 20; accepted 12 to 128).
        #[arg(long, default_value_t = 20)]
        length: usize,
    },
    /// Report whether the Forge-wide administrator is configured and its email (never the password).
    Status,
    /// Validate the manifest's `identity:` block without contacting any provider.
    ValidateConfig {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Build a fresh OIDC authorization request (state, nonce, PKCE) for the named project.
    BuildChallenge {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Complete the OIDC round trip from a provider callback + claims and mint a per-project admin session.
    CompleteAuth {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// State value the provider echoed in the callback (must match the issued challenge).
        #[arg(long)]
        state: String,
        /// Authorization code the provider returned.
        #[arg(long)]
        code: String,
        /// Optional provider-reported error (e.g. `access_denied`).
        #[arg(long)]
        error: Option<String>,
        /// Optional human-readable error description from the provider.
        #[arg(long)]
        error_description: Option<String>,
        /// Subject (`sub` claim) the provider authenticated.
        #[arg(long)]
        subject: String,
        /// Provider-issued `iss` claim. Defaults to the manifest's configured issuer.
        #[arg(long)]
        issuer: Option<String>,
        /// Provider-issued `aud` claim. Defaults to the manifest's configured audience.
        #[arg(long)]
        audience: Option<String>,
        /// Nonce the provider echoed in the id_token.
        #[arg(long)]
        nonce: String,
        /// Provider-issued `iat` (RFC 3339). Defaults to now.
        #[arg(long)]
        issued_at: Option<String>,
        /// Provider-issued `exp` (RFC 3339). Defaults to `issued_at + 60s`.
        #[arg(long)]
        expires_at: Option<String>,
        /// Comma-separated scopes the provider granted.
        #[arg(long)]
        scope: Option<String>,
        /// Value of the project's configured admin_claim (e.g. the `groups` claim).
        #[arg(long)]
        admin_claim_value: String,
    },
    /// List every persisted admin session for the named project.
    SessionList {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Inspect one persisted admin session.
    SessionInspect {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Session id (hex).
        #[arg(long)]
        session: String,
    },
    /// Validate a session id against the named project and permission (read-only check).
    SessionValidate {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Session id (hex).
        #[arg(long)]
        session: String,
        /// Required permission (default: `admin:access`).
        #[arg(long, default_value = "admin:access")]
        permission: String,
    },
    /// Terminate the named admin session and remove its persisted state.
    SessionTerminate {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Session id (hex).
        #[arg(long)]
        session: String,
    },
}
/// Shared source selection and filter flags for the catalog commands.
#[derive(Debug, Clone, clap::Args)]
pub(crate) struct CatalogFilterArgs {
    /// Source to read (repeatable): local, git, workspace-registry,
    /// inventory or github. Defaults to `local`; only declared sources are
    /// read and no parent directory is ever scanned.
    #[arg(long = "source", value_name = "KIND")]
    pub(super) sources: Vec<String>,
    /// Tag predicate (repeatable; values are OR within the predicate).
    #[arg(long = "tag", value_name = "TAG")]
    pub(super) tags: Vec<String>,
    /// Language predicate (repeatable).
    #[arg(long = "language", value_name = "LANGUAGE")]
    pub(super) languages: Vec<String>,
    /// Profile predicate (repeatable).
    #[arg(long = "profile", value_name = "PROFILE")]
    pub(super) profiles: Vec<String>,
    /// Lifecycle predicate (repeatable).
    #[arg(long = "lifecycle", value_name = "LIFECYCLE")]
    pub(super) lifecycles: Vec<String>,
    /// Repository substring predicate (repeatable).
    #[arg(long = "repository", value_name = "REPO")]
    pub(super) repositories: Vec<String>,
    /// CI-state predicate (repeatable).
    #[arg(long = "ci", value_name = "STATE")]
    pub(super) ci: Vec<String>,
    /// Compose-state predicate (repeatable).
    #[arg(long = "compose", value_name = "STATE")]
    pub(super) compose: Vec<String>,
    /// Evidence-state predicate (repeatable).
    #[arg(long = "evidence", value_name = "STATE")]
    pub(super) evidence: Vec<String>,
    /// Generic `key=value` filter; an unknown key is a `catalog-invalid`
    /// refusal before any source is contacted.
    #[arg(long = "filter", value_name = "KEY=VALUE")]
    pub(super) filters: Vec<String>,
    /// Explicit Git working tree for `--source git` (repeatable).
    #[arg(long = "git-repository", value_name = "PATH")]
    pub(super) git_repositories: Vec<PathBuf>,
    /// Workspace registry document for `--source workspace-registry`
    /// (defaults to $FORGE_WORKSPACE_REGISTRY).
    #[arg(long = "workspace-registry", value_name = "PATH")]
    pub(super) workspace_registry: Option<PathBuf>,
    /// Portable inventory source for `--source inventory`
    /// (defaults to $FORGE_INVENTORY_SOURCE).
    #[arg(long = "inventory", value_name = "PATH")]
    pub(super) inventory: Option<PathBuf>,
    /// Explicit GitHub repository for `--source github` (repeatable;
    /// `owner/repo`). When no value is given, the GitHub source walks
    /// the local registry for projects whose `git_remote` points to
    /// github.com.
    #[arg(long = "github-repository", value_name = "OWNER/REPO")]
    pub(super) github_repositories: Vec<String>,
    /// Read-time staleness window in seconds (1..=31536000; default 86400).
    #[arg(
        long = "max-age",
        value_name = "SECONDS",
        default_value_t = CATALOG_DEFAULT_MAX_AGE_SECONDS
    )]
    pub(super) max_age: i64,
    /// Maximum records per page (1..=1000; default 50).
    #[arg(long = "limit", value_name = "N", default_value_t = CATALOG_DEFAULT_LIMIT)]
    pub(super) limit: usize,
    /// Opaque cursor returned by a previous page.
    #[arg(long = "cursor", value_name = "CURSOR")]
    pub(super) cursor: Option<String>,
}
#[derive(Debug, Subcommand)]
pub(crate) enum FeatureCommands {
    /// List all catalog features with tested versions and strategies.
    List,
    /// Inspect one catalog feature descriptor.
    Inspect {
        /// Feature id (e.g. `auth`).
        id: String,
    },
    /// Resolve requested capabilities into a deterministic install plan without changing files.
    Resolve {
        /// Profile id (e.g. `rust-web`).
        profile: String,
        /// Requested capability (repeatable, e.g. `--feature admin`).
        #[arg(long = "feature")]
        features: Vec<String>,
    },
    /// Add a feature (plus missing dependencies) to a project.
    Add {
        /// Feature id (e.g. `auth`).
        feature: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Explicit version (default: tested catalog version).
        #[arg(long)]
        version: Option<String>,
    },
    /// Remove a feature from a project.
    Remove {
        /// Feature id (e.g. `auth`).
        feature: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Upgrade a feature to the tested catalog version.
    Upgrade {
        /// Feature id (e.g. `auth`).
        feature: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Explicit version (default: tested catalog version).
        #[arg(long)]
        version: Option<String>,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum ReleaseCommands {
    /// Capture semver, changelog, source revision and the doctor/test/DriftWatch evidence into a reviewable plan.
    Prepare {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Semver version for this release (e.g. `1.2.3`).
        #[arg(long)]
        version: String,
        /// Read-only plan: capture checks without persisting a release record.
        #[arg(long)]
        dry_run: bool,
    },
    /// Apply a verified release plan: walks every stage with safe retry and per-stage records.
    Apply {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Semver version for this release (e.g. `1.2.3`).
        #[arg(long)]
        version: String,
        /// Required explicit confirmation for tag, push, package, container and notes side effects.
        #[arg(long)]
        confirm: bool,
        /// Re-run only stages that have not yet delivered at the current revision.
        #[arg(long)]
        retry: bool,
        /// Read-only plan: report the would-apply plan without contacting any provider.
        #[arg(long)]
        dry_run: bool,
        /// Override the manifest's default stage list (repeatable).
        #[arg(long = "stage", value_name = "STAGE")]
        stages: Vec<String>,
    },
    /// List all persisted releases under the project's release directory.
    List {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Inspect a single persisted release record.
    Inspect {
        /// Release id (e.g. `app-1.2.3-deadbeefcafe`).
        release_id: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum WebCommands {
    /// Serve files from `frontend/` on an independent loopback web listener.
    Serve {
        /// Bind address (default `127.0.0.1`).
        #[arg(long, default_value = "127.0.0.1")]
        bind: std::net::IpAddr,
        /// TCP port (default `4173`).
        #[arg(long, default_value_t = 4173)]
        port: u16,
        /// Standalone frontend directory (default `frontend`).
        #[arg(long, default_value = "frontend")]
        root: PathBuf,
    },
}
