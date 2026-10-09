//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use clap::Subcommand;
use forge::fleet::DEFAULT_MAX_AGE_SECONDS;
use std::path::PathBuf;

use super::commands::SemanticSuggestArgs;
use super::commands_services::CatalogFilterArgs;

#[derive(Debug, Subcommand)]
pub(crate) enum KitCommands {
    /// Repack the committed feed from a checked-out sibling library.
    ///
    /// The feed is committed, so it is regenerated rather than hand-copied.
    /// The sibling is only ever read: nothing outside this repository is
    /// written, and nothing is published or signed.
    Pack {
        /// Path to the sibling library checkout
        /// (default: `$FORGE_PLATFORM_LIBS`, then the workspace convention).
        #[arg(long = "platform-libs")]
        platform_libs: Option<PathBuf>,
    },
    /// Check a committed feed against the kit version a project declares.
    ///
    /// Fails on a version mismatch, a missing package, an undeclared package,
    /// or a tampered byte. This is the drift gate: a check only a human runs is
    /// a check CI does not run.
    Verify {
        /// Project directory to check against its own `forge.yaml`
        /// (default: check Forge's committed feed against the compiled-in kit).
        #[arg(default_value = "")]
        path: String,
    },
    /// Show, and on confirmation apply, an explicit shared-layer kit upgrade.
    ///
    /// The one sanctioned way a pinned project moves between kit versions.
    /// Without `--confirm` this writes nothing and only prints the reviewable
    /// per-file diff. Nothing else can move a pinned project: `forge new`,
    /// `forge doctor`, `forge list`, a CI run and a background process all
    /// leave an existing project's tree byte-identical.
    Upgrade {
        /// Project directory holding the pinned kit.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Target kit version to move to, e.g. `platform-ui-web@0.2.0`.
        #[arg(long = "to")]
        to: String,
        /// Apply the upgrade. Without it the command is a read-only review.
        #[arg(long)]
        confirm: bool,
        /// Replace an owned file the operator edited. The default refuses and
        /// leaves the edited file exactly as it was.
        #[arg(long)]
        force: bool,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum DeployCommands {
    /// Capture target, source revision and the configured health check into a reviewable plan.
    Plan {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Target name (default: `deployment.default`, else the only declared target).
        #[arg(long)]
        target_name: Option<String>,
    },
    /// Apply a verified deploy plan: invokes the adapter and captures the health observation.
    Apply {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Target name (default: `deployment.default`, else the only declared target).
        #[arg(long)]
        target_name: Option<String>,
        /// Required explicit confirmation for the remote write.
        #[arg(long)]
        confirm: bool,
        /// Read-only plan: report the would-apply plan without contacting the adapter.
        #[arg(long)]
        dry_run: bool,
    },
    /// Re-run the health check on a previously applied deploy (R2 boundary handling).
    Observe {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Target name (default: `deployment.default`, else the only declared target).
        #[arg(long)]
        target_name: Option<String>,
        /// Required explicit confirmation for the remote write.
        #[arg(long)]
        confirm: bool,
    },
    /// List all persisted deploys for the named project.
    List {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Inspect a single persisted deploy by its id.
    Inspect {
        /// Deploy id (e.g. `app-home-deadbeefcafe`).
        deploy_id: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Show the persisted Forge publish/deploy status without contacting a provider.
    Status {
        /// Limit the report to one project id.
        #[arg(long)]
        project: Option<String>,
        /// Limit the report to one fleet queue id.
        #[arg(long)]
        queue: Option<String>,
        /// Maximum number of journal entries to return.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Poll the matching queue until it reaches a terminal state
        /// or the deadline expires. Only valid with `--queue`.
        #[arg(long)]
        watch: bool,
        /// Polling interval in seconds for `--watch` (1..=60).
        #[arg(long, default_value_t = 2)]
        interval_secs: u64,
        /// Maximum wall-clock seconds for `--watch` (1..=86400).
        #[arg(long, default_value_t = 600)]
        deadline_secs: u64,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum ProjectCommands {
    /// List the normalized project catalog with provenance.
    List {
        #[command(flatten)]
        filters: CatalogFilterArgs,
    },
    /// Inspect every catalog record for one project id (all sources).
    Inspect {
        /// Project id to inspect.
        project: String,
        #[command(flatten)]
        filters: CatalogFilterArgs,
    },
    /// List distinct tags across the filtered catalog with counts.
    Tags {
        #[command(flatten)]
        filters: CatalogFilterArgs,
    },
    /// List distinct languages across the filtered catalog with counts.
    Languages {
        #[command(flatten)]
        filters: CatalogFilterArgs,
    },
    /// Report evidence-backed project metadata gaps
    /// (`forge-project-evidence/0.1.0`).
    Gaps {
        /// Project id to assess (default: every project in the catalog).
        project: Option<String>,
        #[command(flatten)]
        filters: CatalogFilterArgs,
        /// Category predicate (repeatable; values are OR within the predicate).
        #[arg(long = "category", value_name = "CATEGORY")]
        categories: Vec<String>,
        /// Status predicate (repeatable; values are OR within the predicate).
        #[arg(long = "status", value_name = "STATUS")]
        statuses: Vec<String>,
        /// Remediation-class predicate (repeatable; values are OR within
        /// the predicate).
        #[arg(long = "remediation-class", value_name = "CLASS")]
        remediation_classes: Vec<String>,
    },
    /// Optional GitHub metadata observation and approved change surface
    /// (`github-project-metadata-adapter`).
    Github {
        #[command(subcommand)]
        command: GithubCommands,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum InventoryCommands {
    /// Load, validate and project an inventory into the fleet
    /// classification report. Read-only; never invokes a
    /// provider or mutates the registry.
    Show {
        /// Path to a local inventory JSON file or an external
        /// adapter executable. Defaults to $FORGE_INVENTORY_SOURCE.
        #[arg(value_name = "SOURCE")]
        source: Option<PathBuf>,
        /// Public domain used for `<project>.<domain>` routing
        /// (default `tooosall.uk`).
        #[arg(long, value_name = "DOMAIN", default_value = "tooosall.uk")]
        domain: String,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum StandardCommands {
    /// List all versioned standard packs with support and evidence state.
    List,
    /// Inspect one standard pack version (`<pack>@<version>`).
    Inspect {
        /// Pack selector, e.g. `baseline-service@1.1.0`.
        pack: String,
    },
    /// Verify a project's `.standard/` snapshot against its ownership receipt.
    Check {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Show what an upgrade to a pack version would change. Read-only.
    Diff {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Target pack selector, e.g. `baseline-service@1.1.0`.
        #[arg(long)]
        against: String,
    },
    /// Upgrade a project's snapshot to a pack version; refuses modified files.
    Upgrade {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Target pack selector, e.g. `baseline-service@1.1.0`.
        #[arg(long)]
        to: String,
        /// Required explicit confirmation for the file writes.
        #[arg(long)]
        confirm: bool,
        /// Replace modified or foreign owned files after reviewing
        /// `standard diff` (the reviewed conflict resolution).
        #[arg(long)]
        force: bool,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum ProviderCommands {
    /// Report the provider matrix. Without `--live` every row is `not-run`; a not-run row never claims support.
    Matrix {
        /// Attempt live sandbox reachability (requires `FORGE_PROVIDER_LIVE=1`).
        #[arg(long)]
        live: bool,
    },
    /// Drive one controlled round trip for a provider with provenance and teardown.
    Run {
        /// Evidence provider id (e.g. `driftwatch-policy`).
        provider: String,
        /// Registered project id or filesystem path the evidence is attributed to (default: disposable probe, no project invented).
        #[arg(value_name = "TARGET")]
        target: Option<String>,
        /// Attempt the real sandbox binary (requires `FORGE_PROVIDER_LIVE=1`).
        #[arg(long)]
        live: bool,
        /// Explicit fixture binary standing in for the sandbox (labels the row `sandbox: fixture`).
        #[arg(long, value_name = "PATH")]
        fixture: Option<PathBuf>,
        /// Analytics project ref scoping the analytics probe (default: `owner/repo`).
        #[arg(long, default_value = "owner/repo")]
        project_ref: String,
    },
    /// Describe one provider's boundary, binary override, secret and teardown rules without probing.
    Inspect {
        /// Evidence provider id (e.g. `deploy`).
        provider: String,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum DescribeCommands {
    /// Suggest a bounded description for the named project.
    Suggest(SemanticSuggestArgs),
    /// List every recorded semantic proposal for the named project.
    List {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Show the full proposal manifest for one proposal id.
    Show {
        /// Proposal id (`<kind>-<hash>`, e.g. `description-deadbeef`).
        proposal: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Approve a `Suggested` proposal after explicit confirmation.
    Approve {
        /// Proposal id.
        proposal: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Required explicit confirmation; without it the command refuses.
        #[arg(long)]
        confirm: bool,
    },
    /// Reject a `Suggested` proposal after explicit confirmation.
    Reject {
        /// Proposal id.
        proposal: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Required explicit confirmation; without it the command refuses.
        #[arg(long)]
        confirm: bool,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum ProfileCommands {
    /// List all MVP profiles with descriptor versions.
    List,
    /// Inspect one MVP profile descriptor.
    Inspect {
        /// Profile id (e.g. `rust-web`).
        id: String,
    },
    /// Resolve a profile plus requested capabilities without changing files.
    Resolve {
        /// Profile id (e.g. `flutter-app`).
        id: String,
        /// Requested capability (repeatable, e.g. `--feature postgres`).
        #[arg(long = "feature")]
        features: Vec<String>,
    },
    /// Preflight the profile's required toolchain without claiming it was tested.
    Preflight {
        /// Profile id (e.g. `rust-web`).
        id: String,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum SpecCommands {
    /// Generate a bounded spec for the named project and finding set.
    Generate {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Finding id to include in the bounded proposal (repeatable).
        #[arg(long = "finding", value_name = "FINDING")]
        findings: Vec<String>,
        /// Optional human reason for the generation; defaults to the finding-derived summary.
        #[arg(long)]
        reason: Option<String>,
    },
    /// List all generated specs under `.forge/specs/` for the named project.
    List {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Show the full bounded proposal for a generated spec.
    Inspect {
        /// Spec id (e.g. `my-app-abcdef012345`) or its hash prefix.
        spec_id: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Classify one finding into a deterministic, semantic or manual route.
    Route {
        /// Finding id to route (matches a doctor, driftwatch-* or semantic-* id).
        finding: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Apply a routing decision: deterministic action is recorded, semantic produces a spec, manual is noted.
    Apply {
        /// Finding id to route through remediation.
        finding: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Optional reason recorded on the spec when the route is semantic.
        #[arg(long)]
        reason: Option<String>,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum ComponentCommands {
    /// List the versioned semantic component catalog.
    List,
    /// Inspect one catalog component (contract, evidence, compatibility).
    Inspect {
        /// Component id (e.g. `paginated-query`).
        id: String,
    },
    /// Resolve the named components for a profile with quality-aware selection.
    Resolve {
        /// Profile id (e.g. `rust-web`).
        #[arg(long)]
        profile: String,
        /// Component id (repeatable, e.g. `--component paginated-query`).
        #[arg(long = "component", value_name = "COMPONENT")]
        components: Vec<String>,
    },
    /// Promote a component's quality level (evidence-gated; preserves prior level on failure).
    Qualify {
        /// Component id (e.g. `toast`).
        id: String,
        /// Target quality (`experimental`, `verified`, `certified`, `deprecated`).
        #[arg(long = "to")]
        to: String,
        /// Human reason for the promotion.
        #[arg(long)]
        reason: String,
        /// Override test coverage (default: 0.95 for `certified`).
        #[arg(long)]
        coverage: Option<f32>,
        /// Override the last-verified timestamp (RFC 3339). Default: now.
        #[arg(long)]
        last_verified: Option<String>,
        /// Mark known issues (repeatable).
        #[arg(long = "known-issue")]
        known_issues: Vec<String>,
        /// Set security review status (default: `true` for `certified`).
        #[arg(long)]
        security_review: Option<bool>,
        /// Project directory the receipt is written under (default: current directory).
        #[arg(long)]
        path: Option<PathBuf>,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum AnalyticsCommands {
    /// Inspect the manifest's analytics block, recording timestamped health observations for each configured provider.
    Inspect {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Plan the round trip without invoking any provider adapter.
        #[arg(long)]
        dry_run: bool,
    },
    /// Aggregate timestamped project metrics from the registry, doctor, deploy and external observations.
    Metrics {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Default observation window in days for growth-style metrics (1..=90).
        #[arg(long)]
        window_days: Option<u32>,
        /// Aggregate over the entire registered registry instead of one project.
        #[arg(long)]
        all: bool,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum ReadinessCommands {
    /// Generate every supported profile fixture into a disposable directory and run its native build/test without Forge on PATH.
    Matrix {
        /// Selected profile id (repeatable, e.g. `--profile rust-web`). Defaults to every supported profile in stable catalog order.
        #[arg(long = "profile")]
        profiles: Vec<String>,
    },
    /// Report the platform-native Forge binary evidence (path, SHA-256, version smoke).
    Artifact,
    /// Evaluate the release gate over the selected matrix rows plus the artifact smoke.
    Check {
        /// Selected profile id (repeatable, e.g. `--profile rust-web`). Defaults to every supported profile in stable catalog order.
        #[arg(long = "profile")]
        profiles: Vec<String>,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum FleetCommands {
    /// List the portfolio declared by the workspace registry as timestamped fleet observations.
    List {
        /// Workspace registry document (`projects.json`). Defaults to $FORGE_WORKSPACE_REGISTRY.
        #[arg(long, value_name = "PATH")]
        workspace_registry: Option<PathBuf>,
        /// Registry age in seconds beyond which the report is stale (1..=31536000; default 86400).
        #[arg(long, value_name = "SECONDS", default_value_t = DEFAULT_MAX_AGE_SECONDS)]
        max_age: i64,
    },
    /// Report the fleet registry health only (source, freshness, counts; no entries).
    Status {
        #[arg(long, value_name = "PATH")]
        workspace_registry: Option<PathBuf>,
        #[arg(long, value_name = "SECONDS", default_value_t = DEFAULT_MAX_AGE_SECONDS)]
        max_age: i64,
    },
    /// Inspect one declared fleet entry by id (read-only; unmanaged entries can never be operated on through the mirror).
    Inspect {
        /// Declared fleet entry id.
        entry: String,
        #[arg(long, value_name = "PATH")]
        workspace_registry: Option<PathBuf>,
        #[arg(long, value_name = "SECONDS", default_value_t = DEFAULT_MAX_AGE_SECONDS)]
        max_age: i64,
    },
    /// Probe the served router rules and target containers to verdict whether every routed host is online
    /// (`ONLINE`/`DOWN`/`NO-ROUTE`/`NOT-DEPLOYED`). Read-only: no journal rows, no registry writes, no target writes.
    Online {
        /// Explicit inventory source (local JSON file or executable adapter path). Takes precedence over `--fleet-registry`.
        #[arg(long = "inventory")]
        inventory: Option<std::path::PathBuf>,
        /// Path to the legacy workspace-governance `projects.json` registry. Used only when `--inventory` is absent.
        #[arg(long = "fleet-registry")]
        fleet_registry: Option<std::path::PathBuf>,
        /// Workspace root (where project paths in the legacy registry are resolved against).
        #[arg(long)]
        workspace_root: Option<std::path::PathBuf>,
        /// Public domain used for `<project>.<domain>` routing (default `tooosall.uk`).
        #[arg(long, value_name = "DOMAIN", default_value = "tooosall.uk")]
        domain: String,
        /// Per-probe HTTP timeout in seconds (1..=120; default 12).
        #[arg(
            long,
            value_name = "SECONDS",
            default_value_t = forge::fleet::online::DEFAULT_HTTP_TIMEOUT_SECS
        )]
        timeout_secs: u64,
        /// Read-only plan: render the probe plan without contacting the target or any origin.
        #[arg(long)]
        dry_run: bool,
    },
}
/// Subcommands of `forge project github`.
#[derive(Debug, Subcommand)]
pub(crate) enum GithubCommands {
    /// Observe one or more GitHub repositories through the configured
    /// adapter (`forge-github-metadata/0.1.0`). Read-only; no
    /// repository state is changed.
    Observe {
        /// Repository identities (`owner/repo`) to observe. When no
        /// argument is supplied, the command walks the local registry
        /// for projects whose `git_remote` points to github.com.
        #[arg(value_name = "OWNER/REPO")]
        repositories: Vec<String>,
        /// GitHub host (default: `github.com`).
        #[arg(long, value_name = "HOST", default_value = "github.com")]
        host: String,
    },
    /// Propose an approved metadata change through the adapter. Pull
    /// request is the default; direct mode requires
    /// `--mode direct --confirm <token>` and the adapter echoes the
    /// token back before any write.
    Propose {
        /// Repository identity (`owner/repo`).
        #[arg(value_name = "OWNER/REPO")]
        repository: String,
        /// GitHub host (default: `github.com`).
        #[arg(long, value_name = "HOST", default_value = "github.com")]
        host: String,
        /// Mutation mode: `pull-request` (default) or `direct`.
        #[arg(long, value_name = "MODE", default_value = "pull-request")]
        mode: String,
        /// Required confirmation string for `direct` mode. The
        /// adapter must echo it back, otherwise the call is refused.
        #[arg(long, value_name = "TOKEN")]
        confirm: Option<String>,
        /// Approved change in `field=value` form (repeatable).
        /// Closed field set: `topic`, `description`, `homepage`,
        /// `language`.
        #[arg(long = "set", value_name = "FIELD=VALUE")]
        sets: Vec<String>,
    },
    /// Probe the installed GitHub CLI authentication state without
    /// reading or displaying the credential
    /// (`forge-github-cli-workflows/0.1.0`).
    Auth {
        /// GitHub host (default: `github.com`).
        #[arg(long, value_name = "HOST", default_value = "github.com")]
        host: String,
    },
    /// Clone an existing GitHub repository through the installed
    /// `gh` CLI to a local destination directory.
    Clone {
        /// Repository identity (`owner/repo`).
        #[arg(value_name = "OWNER/REPO")]
        repository: String,
        /// Local destination directory.
        #[arg(value_name = "PATH")]
        destination: PathBuf,
        /// Required explicit confirmation. The command refuses any
        /// clone without this flag so an unexpected remote read is
        /// never implicit.
        #[arg(long)]
        confirm: bool,
    },
    /// Create a remote GitHub repository for an existing local
    /// project. Private is the default; public requires
    /// `--visibility public --confirm-public`. Initial source push
    /// requires `--push-source --confirm`.
    Create {
        /// Project identifier or registered path.
        #[arg(value_name = "PROJECT")]
        project: String,
        /// Repository identity to create on GitHub (`owner/name`).
        #[arg(long = "repo", value_name = "OWNER/NAME")]
        repo: String,
        /// Visibility (`private` default, `public` requires
        /// `--confirm-public`).
        #[arg(long, value_name = "VISIBILITY", default_value = "private")]
        visibility: String,
        /// Required to make a public repository visible without
        /// review.
        #[arg(long = "confirm-public")]
        confirm_public: bool,
        /// Push the local source to the new remote on creation.
        #[arg(long = "push-source")]
        push_source: bool,
        /// Required explicit confirmation that this is a remote
        /// write.
        #[arg(long)]
        confirm: bool,
        /// Validate `forge.yaml` and register the project when it is
        /// not yet in the registry, then create the remote.
        #[arg(long = "register-if-missing")]
        register_if_missing: bool,
    },
    /// Open a draft pull request for a registered project through
    /// the installed `gh` CLI.
    PullRequest {
        /// Project identifier or registered path.
        #[arg(value_name = "PROJECT")]
        project: String,
        /// Pull-request title.
        #[arg(long, value_name = "TITLE")]
        title: String,
        /// Pull-request body.
        #[arg(long, value_name = "BODY")]
        body: String,
        /// Open the pull request as a draft.
        #[arg(long)]
        draft: bool,
        /// Required explicit confirmation that this is a remote
        /// write.
        #[arg(long)]
        confirm: bool,
    },
}
