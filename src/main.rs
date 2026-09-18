//! `forge` CLI transport: argument parsing and output rendering only.
//!
//! All project rules live in Core; this layer maps typed outcomes to
//! human text or JSON plus exit codes, independently of any GUI, AI or
//! network service.

use clap::{Parser, Subcommand, ValueEnum};
use forge::agent::{
    apply_transition as apply_agent_transition, list_sessions, new_session, read_session,
    run_spec as run_session_spec, status_for, AgentProvider, AgentTransitionOutcome,
    SessionTransition,
};
use forge::component::{
    component_catalog, inspect_component, record_qualification,
    render_outcome_human as render_component_outcome_human, render_qualify_human, resolve_outcome,
    ComponentQualifyEvidence, ComponentQualifyRequest, ComponentQuality,
};
use forge::core::ForgeError;
use forge::deploy::{DeployAdapterConfig, DeployRequest};
use forge::distribution::{
    apply_mirror, distribution_config_from_manifest, plan_mirror, DistributionConfig, MirrorRequest,
};
use forge::docs::{
    docs_config_from_manifest, run_translate, TranslateReport, TranslateRequest, TranslatorConfig,
};
use forge::doctor::{
    parse_target_level, render_report_human, run_doctor, FindingStatus, RegistryObservation,
    Remediation,
};
use forge::feature::{
    add_feature, feature_catalog, inspect_feature, remove_feature, render_outcome_human,
    render_plan_human as render_feature_plan_human, resolve_plan, upgrade_feature,
};
use forge::generate::{generate, normalize_explicit, parse_interactive, verify_native};
use forge::gitops::{commit_paths, push_ref, run_test, CommitOutcome, PushOutcome, TestOutcome};
use forge::import::{adopt_import, inspect_import, render_proposal_human};
use forge::planner::{
    apply_plan as apply_planner_plan, intent_hash, plans_dir, render_apply_human,
    render_intent_validation_human, render_plan_human, resolve_plan as resolve_planner_plan,
    validate_intent, write_plan_receipt, Intent, IntentAction, IntentConstraint,
    IntentResolveOutcome, IntentValidationOutcome, PLANNER_CONTRACT_VERSION,
};
use forge::policy::{run_driftwatch, DriftWatchConfig};
use forge::profile::{inspect_profile, list_profiles, preflight_profile, resolve_profile};
use forge::registry::{default_registry_path, ProjectRecord, Registry};
use forge::release::engine::{
    apply_release, list_releases, prepare_release, read_release, PlanReport as EnginePlanReport,
    ReleaseListEntry,
};
use forge::release::{
    release_config_from_manifest, render_report_human as render_release_report_human,
    ReleaseAdapterConfig, ReleaseReport, ReleaseRequest, ReleaseState, Semver,
};
use forge::spec::{
    apply_routing, ensure_single_project, generate_spec, list_specs, read_spec,
    render_proposal_markdown, route_finding, DoctorFindingInput, FindingSource, RoutingOutcome,
    SpecDraft, SpecGenerateOutcome, SpecListEntry, SpecRequest,
};
use forge::ui_pattern::{
    inspect_ui_pattern, install_pattern, render_install_human,
    render_outcome_human as render_ui_pattern_outcome_human,
    resolve_outcome as resolve_ui_pattern_outcome, ui_pattern_catalog, UiPatternInstallRequest,
    UiPatternRequest,
};
use forge::upgrade::{
    apply_upgrade, plan_upgrade, render_fleet_human, render_outcome_human as render_upgrade_human,
    render_plan_human as render_upgrade_plan_human, run_fleet, FleetReport, UpgradeOutcome,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Human,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "forge",
    version,
    about = "Forge developer control plane: versioned project model and registry foundation"
)]
struct Cli {
    /// Registry database path (overrides $FORGE_REGISTRY and the default).
    #[arg(long, global = true)]
    registry: Option<PathBuf>,

    /// Output format.
    #[arg(long, global = true, value_enum, default_value = "human")]
    format: Format,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// List registered projects.
    List,
    /// Inspect one registered project by id or path.
    Inspect {
        /// Registered project id or filesystem path.
        target: String,
    },
    /// Validate the manifest in a directory and persist the project.
    Register {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Explicit manifest filename for legacy `platform.yaml` import.
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    /// Inspect an existing repository and, on acceptance, adopt it.
    Import {
        /// Existing repository directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Explicit profile id resolving ambiguous detection.
        #[arg(long)]
        profile: Option<String>,
        /// Write the minimal manifest (when missing) and register.
        #[arg(long)]
        accept: bool,
        /// Explicit project id overriding the directory-name default.
        #[arg(long)]
        id: Option<String>,
    },
    /// Inspect versioned MVP profile descriptors and compatibility.
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },
    /// Create a new project deterministically from pinned profile assets.
    New {
        /// Destination directory for the new project.
        path: PathBuf,
        /// Explicit profile id (e.g. `rust-web`); prompts when missing.
        #[arg(long)]
        profile: Option<String>,
        /// Explicit project id (default: kebab-cased directory name).
        #[arg(long)]
        id: Option<String>,
        /// Explicit display name (default: project id).
        #[arg(long)]
        name: Option<String>,
        /// Requested capability (repeatable, e.g. `--feature auth`).
        #[arg(long = "feature")]
        features: Vec<String>,
        /// Run the profile's native build/test after generation.
        #[arg(long)]
        verify_native: bool,
    },
    /// Inspect project health and evidence-based maturity without changing files.
    Doctor {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Assessment target level overriding the manifest (`L0`..`L4`).
        #[arg(long)]
        target: Option<String>,
    },
    /// Resolve and manage versioned features.
    Feature {
        #[command(subcommand)]
        command: FeatureCommands,
    },
    /// Plan and apply deterministic project/fleet upgrades with conflict handoff.
    Upgrade {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(value_name = "TARGET", conflicts_with = "all")]
        target: Option<String>,
        /// Single feature to upgrade (default: every outdated installed feature).
        #[arg(long = "feature", value_name = "FEATURE")]
        feature: Option<String>,
        /// Run over the explicit captured registry selection with per-project journals.
        #[arg(long)]
        all: bool,
        /// Print pinned plans without changing files, registry rows or journals.
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate bounded spec proposals and route findings to deterministic, semantic or manual remediation.
    Spec {
        #[command(subcommand)]
        command: SpecCommands,
    },
    /// Manage agent sessions for the named project.
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
    /// Run the profile's native test command on the named project.
    Test {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Stage the named paths and create a single scoped commit.
    Commit {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Path to stage (repeatable). At least one path is required.
        #[arg(long = "path", value_name = "PATH")]
        paths: Vec<String>,
        /// Commit message.
        #[arg(long)]
        message: String,
    },
    /// Push the named ref to the project's remote (requires --confirm).
    Push {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Remote name (default: origin).
        #[arg(long, default_value = "origin")]
        remote: String,
        /// Ref name (default: HEAD).
        #[arg(long, default_value = "HEAD")]
        ref_name: String,
        /// Required explicit confirmation for the remote write.
        #[arg(long)]
        confirm: bool,
    },
    /// Run the MCP stdio server over the mature Core operations.
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },
    /// Distribute a project's refs to its canonical primary and configured one-way mirrors.
    Mirror {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Ref to distribute (repeatable, e.g. `--ref main`). At least one ref is required.
        #[arg(long = "ref", value_name = "REF")]
        refs: Vec<String>,
        /// Required explicit confirmation for the remote write.
        #[arg(long)]
        confirm: bool,
        /// Print the plan without contacting any remote.
        #[arg(long)]
        dry_run: bool,
        /// Re-push only refs not yet recorded as delivered.
        #[arg(long)]
        retry_failed: bool,
    },
    /// Translate the canonical documentation into enabled derivative locales.
    Docs {
        #[command(subcommand)]
        command: DocsCommands,
    },
    /// Prepare, apply, list and inspect gated resumable releases.
    Release {
        #[command(subcommand)]
        command: ReleaseCommands,
    },
    /// Plan, apply, observe and inspect adapter-based deployments.
    Deploy {
        #[command(subcommand)]
        command: DeployCommands,
    },
    /// Discover, resolve and promote semantic components.
    Component {
        #[command(subcommand)]
        command: ComponentCommands,
    },
    /// Discover, resolve and install semantic UI patterns across web and Flutter.
    UiPattern {
        #[command(subcommand)]
        command: UiPatternCommands,
    },
    /// Validate a structured intent, resolve it into a deterministic assembly plan, and apply the plan with explicit confirmation.
    Intent {
        #[command(subcommand)]
        command: IntentCommands,
    },
}

#[derive(Debug, Subcommand)]
enum ProfileCommands {
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
enum FeatureCommands {
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
enum SpecCommands {
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
enum McpCommands {
    /// Run the JSON-RPC 2.0 stdio server until stdin closes.
    Serve,
}

#[derive(Debug, Subcommand)]
enum DocsCommands {
    /// Translate the canonical source document into one enabled locale (or every enabled locale with --all).
    Translate {
        /// Locale tag to translate (e.g. `zh-CN`). Required unless `--all` is passed.
        #[arg(value_name = "LOCALE")]
        locale: Option<String>,
        /// Translate every enabled locale, skipping disabled ones without contacting any provider.
        #[arg(long)]
        all: bool,
        /// Project directory (default: current directory).
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ReleaseCommands {
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
enum DeployCommands {
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
}

#[derive(Debug, Subcommand)]
enum ComponentCommands {
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
enum UiPatternCommands {
    /// List the versioned semantic UI pattern catalog.
    List,
    /// Inspect one catalog UI pattern (state, design, adapter, evidence).
    Inspect {
        /// UI pattern id (e.g. `login`).
        id: String,
    },
    /// Resolve the named UI patterns for a profile with quality-aware selection.
    Resolve {
        /// Profile id (e.g. `react-web`, `nextjs-web`, `flutter-app`).
        #[arg(long)]
        profile: String,
        /// UI pattern id (repeatable, e.g. `--pattern login`).
        #[arg(long = "pattern", value_name = "PATTERN")]
        patterns: Vec<String>,
    },
    /// Install a UI pattern's ordinary source and write a metadata receipt (refuses to overwrite customized files).
    Install {
        /// UI pattern id (e.g. `form`).
        id: String,
        /// Profile id (e.g. `react-web`).
        #[arg(long)]
        profile: String,
        /// Human reason for the installation.
        #[arg(long)]
        reason: String,
        /// Project directory the receipt is written under (default: current directory).
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum IntentCommands {
    /// Validate a structured intent without resolving or applying it.
    Validate {
        /// Action verb (`create_project` or `extend_project`).
        #[arg(long)]
        action: String,
        /// Target profile id.
        #[arg(long)]
        profile: String,
        /// Required capability (repeatable).
        #[arg(long = "require", value_name = "CAPABILITY")]
        required: Vec<String>,
        /// Forbidden capability (repeatable).
        #[arg(long = "forbid", value_name = "CAPABILITY")]
        forbidden: Vec<String>,
        /// Constraint as `key=value` (repeatable).
        #[arg(long = "constraint", value_name = "KEY=VALUE")]
        constraints: Vec<String>,
    },
    /// Resolve a validated intent into a reviewable deterministic assembly plan and persist its receipt.
    Resolve {
        /// Action verb (`create_project` or `extend_project`).
        #[arg(long)]
        action: String,
        /// Target profile id.
        #[arg(long)]
        profile: String,
        /// Required capability (repeatable).
        #[arg(long = "require", value_name = "CAPABILITY")]
        required: Vec<String>,
        /// Forbidden capability (repeatable).
        #[arg(long = "forbid", value_name = "CAPABILITY")]
        forbidden: Vec<String>,
        /// Constraint as `key=value` (repeatable).
        #[arg(long = "constraint", value_name = "KEY=VALUE")]
        constraints: Vec<String>,
        /// Project directory the receipt is written under (default: current directory).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Re-validate and apply a previously persisted plan (requires `--confirm`).
    Apply {
        /// Plan id (e.g. `rust-web-42acd579408d-776132e5aa5f`).
        plan_id: String,
        /// Required explicit confirmation for the project mutation.
        #[arg(long)]
        confirm: bool,
        /// Project directory the receipt is read from (default: current directory).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// List the persisted plan receipts under `.forge/planner/`.
    List {
        /// Project directory the receipts are read from (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommands {
    /// Start a new managed agent session for the named project.
    Start {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Session id (kebab-case).
        #[arg(long)]
        session: String,
        /// Provider id (opencode, codex).
        #[arg(long, default_value = "opencode")]
        provider: String,
        /// Optional spec id to bind the session to.
        #[arg(long)]
        spec: Option<String>,
    },
    /// Pause the named session.
    Pause {
        #[arg(default_value = ".")]
        target: String,
        #[arg(long)]
        session: String,
    },
    /// Take over the named session from the current adapter.
    Takeover {
        #[arg(default_value = ".")]
        target: String,
        #[arg(long)]
        session: String,
    },
    /// Resume the named session.
    Resume {
        #[arg(default_value = ".")]
        target: String,
        #[arg(long)]
        session: String,
    },
    /// Restart the named session.
    Restart {
        #[arg(default_value = ".")]
        target: String,
        #[arg(long)]
        session: String,
    },
    /// Start a fresh session, preserving the named one as historical evidence.
    NewSession {
        #[arg(default_value = ".")]
        target: String,
        /// Existing session to supersede.
        #[arg(long)]
        session: String,
        /// New session id.
        #[arg(long)]
        new_session: String,
    },
    /// Show the recorded state of a session.
    Status {
        #[arg(default_value = ".")]
        target: String,
        #[arg(long)]
        session: String,
    },
    /// List all recorded sessions for the named project.
    List {
        #[arg(default_value = ".")]
        target: String,
    },
    /// Run the bound spec through the named session.
    RunSpec {
        #[arg(default_value = ".")]
        target: String,
        #[arg(long)]
        session: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let db_path = cli.registry.clone().unwrap_or_else(default_registry_path);

    // Fleet runs print their per-project report to stdout even when the
    // fleet is not healthy, so they own their exit code instead of using
    // the generic error path (which prints to stderr without stdout).
    if let Commands::Upgrade {
        feature,
        all: true,
        dry_run,
        ..
    } = &cli.command
    {
        return cmd_upgrade_fleet(&db_path, feature.as_deref(), *dry_run, cli.format);
    }

    let result = match &cli.command {
        Commands::List => cmd_list(&db_path, cli.format),
        Commands::Inspect { target } => cmd_inspect(&db_path, target, cli.format),
        Commands::Register { path, manifest } => {
            cmd_register(&db_path, path, manifest.as_deref(), cli.format)
        }
        Commands::Import {
            path,
            profile,
            accept,
            id,
        } => cmd_import(
            &db_path,
            path,
            profile.as_deref(),
            *accept,
            id.as_deref(),
            cli.format,
        ),
        Commands::Profile { command } => cmd_profile(command, cli.format),
        Commands::New {
            path,
            profile,
            id,
            name,
            features,
            verify_native,
        } => cmd_new(
            &db_path,
            path,
            profile.as_deref(),
            id.as_deref(),
            name.as_deref(),
            features,
            *verify_native,
            cli.format,
        ),
        Commands::Doctor { path, target } => {
            cmd_doctor(&db_path, path, target.as_deref(), cli.format)
        }
        Commands::Feature { command } => cmd_feature(&db_path, command, cli.format),
        Commands::Upgrade {
            feature,
            target,
            all: false,
            dry_run,
        } => cmd_upgrade(
            &db_path,
            target.as_deref().unwrap_or("."),
            feature.as_deref(),
            *dry_run,
            cli.format,
        ),
        Commands::Upgrade { all: true, .. } => {
            // Handled by the early `if let` above; this arm exists only
            // to keep the match exhaustive.
            return ExitCode::from(2);
        }
        Commands::Spec { command } => cmd_spec(command, cli.format),
        Commands::Agent { command } => cmd_agent(db_path.as_path(), command, cli.format),
        Commands::Test { path } => cmd_test(path, cli.format),
        Commands::Commit {
            path,
            paths,
            message,
        } => cmd_commit(path, paths.clone(), message.clone(), cli.format),
        Commands::Push {
            path,
            remote,
            ref_name,
            confirm,
        } => cmd_push(path, remote.clone(), ref_name.clone(), *confirm, cli.format),
        Commands::Mcp { command } => cmd_mcp(&db_path, command),
        Commands::Mirror {
            target,
            refs,
            confirm,
            dry_run,
            retry_failed,
        } => cmd_mirror(
            &db_path,
            target,
            refs.clone(),
            *confirm,
            *dry_run,
            *retry_failed,
            cli.format,
        ),
        Commands::Docs { command } => cmd_docs(&db_path, command, cli.format),
        Commands::Release { command } => cmd_release(&db_path, command, cli.format),
        Commands::Deploy { command } => cmd_deploy(&db_path, command, cli.format),
        Commands::Component { command } => cmd_component(&db_path, command, cli.format),
        Commands::UiPattern { command } => cmd_ui_pattern(&db_path, command, cli.format),
        Commands::Intent { command } => cmd_intent(&db_path, command, cli.format),
    };

    match result {
        Ok(Output::Human(text)) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Ok(Output::Json(value)) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
            ExitCode::SUCCESS
        }
        Err(err) => {
            render_error(&err, cli.format);
            ExitCode::from(err.exit_code() as u8)
        }
    }
}

enum Output {
    Human(String),
    Json(serde_json::Value),
}

fn open_registry(db_path: &Path) -> Result<Registry, ForgeError> {
    Registry::open(db_path)
}

fn as_output(format: Format, human: String, json: serde_json::Value) -> Output {
    match format {
        Format::Human => Output::Human(human),
        Format::Json => Output::Json(json),
    }
}

fn cmd_list(db_path: &Path, format: Format) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let projects = registry.list()?;
    if projects.is_empty() {
        return Ok(as_output(
            format,
            "No projects registered.".to_string(),
            serde_json::json!({"projects": []}),
        ));
    }
    let mut human = format!(
        "{:<20} {:<12} {:<7} {}",
        "Project", "Stack", "Level", "Health"
    );
    for p in &projects {
        human.push_str(&format!(
            "\n{:<20} {:<12} {:<7} {}",
            truncate(&p.id, 20),
            truncate(p.stack.as_deref().unwrap_or("unknown"), 12),
            p.maturity.as_deref().unwrap_or("unknown"),
            p.health(),
        ));
    }
    let json = serde_json::json!({"projects": projects});
    Ok(as_output(format, human, json))
}

fn cmd_inspect(db_path: &Path, target: &str, format: Format) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let p = registry.inspect(target)?;
    let human = render_record_human(&p);
    let json = serde_json::to_value(&p).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    Ok(as_output(format, human, json))
}

fn cmd_register(
    db_path: &Path,
    path: &Path,
    manifest: Option<&std::path::Path>,
    format: Format,
) -> Result<Output, ForgeError> {
    let mut registry = open_registry(db_path)?;
    let p = registry.register(path, manifest)?;
    let human = format!("registered {} ({})", p.id, p.path);
    let json = serde_json::json!({"registered": p});
    Ok(as_output(format, human, json))
}

fn cmd_import(
    db_path: &Path,
    path: &Path,
    profile: Option<&str>,
    accept: bool,
    id: Option<&str>,
    format: Format,
) -> Result<Output, ForgeError> {
    if accept {
        let mut registry = open_registry(db_path)?;
        let record = adopt_import(&mut registry, path, profile, id)?;
        let human = format!("imported {} ({})", record.id, record.path);
        let json = serde_json::json!({"imported": record});
        return Ok(as_output(format, human, json));
    }
    let proposal = inspect_import(path, profile)?;
    let human = render_proposal_human(&proposal);
    let json = serde_json::json!({"proposal": proposal});
    Ok(as_output(format, human, json))
}

#[allow(clippy::too_many_arguments)]
fn cmd_new(
    db_path: &Path,
    path: &Path,
    profile: Option<&str>,
    id: Option<&str>,
    name: Option<&str>,
    features: &[String],
    verify_native_flag: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let request = if profile.is_some() {
        normalize_explicit(profile, id, name, features, path)?
    } else {
        let stdin = std::io::stdin();
        let mut reader = std::io::BufReader::new(stdin.lock());
        let mut writer = std::io::stderr();
        parse_interactive(&mut reader, &mut writer, path, profile, id, name, features)?
    };
    let mut registry = open_registry(db_path)?;
    let mut generated = generate(&mut registry, &request)?;
    if verify_native_flag {
        let report = verify_native(&request.profile, &request.destination, None)?;
        generated.native_verified = report.verified;
        generated.native_note = format!(
            "native build '{}' and test '{}' succeeded for profile '{}'",
            report.build_command, report.test_command, report.profile
        );
    }
    let human = format!(
        "created {} ({}) from {}@{}\nfiles: {}\n{}",
        generated.record.id,
        generated.record.path,
        request.profile,
        forge::generate::GENERATOR_VERSION,
        generated.files.join(", "),
        generated.native_note
    );
    let json = serde_json::json!({
        "created": generated.record,
        "profile": request.profile,
        "generator": forge::generate::GENERATOR_VERSION,
        "files": generated.files,
        "native_verified": generated.native_verified,
        "native_note": generated.native_note,
    });
    Ok(as_output(format, human, json))
}

fn cmd_doctor(
    db_path: &Path,
    path: &Path,
    target: Option<&str>,
    format: Format,
) -> Result<Output, ForgeError> {
    let level = match target {
        Some(raw) => Some(parse_target_level(raw)?),
        None => None,
    };
    // Registry is consulted read-only for observation freshness; a project
    // that is unknown there is still assessed from local evidence.
    let observation = open_registry(db_path)
        .ok()
        .and_then(|registry| observation_for(registry, path));
    // DriftWatch execution is delegated to the policy adapter. The CLI
    // runs the adapter itself (rather than going through the registry)
    // so the project-scoped invocation cannot leak across the open
    // registry. The adapter is configured through environment
    // variables; an absent or non-functional binary surfaces as an
    // `unavailable` finding instead of a hard error.
    let policy_outcome = run_driftwatch(path, &DriftWatchConfig::from_env());
    let report = run_doctor(path, level, observation.as_ref(), Some(&policy_outcome))?;
    let human = render_report_human(&report);
    let json = serde_json::json!({"doctor": report});
    Ok(as_output(format, human, json))
}

/// Best-effort read-only registry observation for `path`: matches the
/// registered record whose canonical path equals `path`, if any.
fn observation_for(registry: Registry, path: &Path) -> Option<RegistryObservation> {
    let canonical = path.canonicalize().ok()?.display().to_string();
    let record = registry
        .list()
        .ok()?
        .into_iter()
        .find(|p| p.path == canonical)?;
    Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at),
    })
}

fn cmd_profile(command: &ProfileCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        ProfileCommands::List => {
            let profiles = list_profiles();
            let mut human = format!(
                "{:<16} {:<10} {:<18} {}",
                "Profile", "Version", "Adapter", "Language"
            );
            for p in &profiles {
                human.push_str(&format!(
                    "\n{:<16} {:<10} {:<18} {}",
                    p.id, p.version, p.adapter, p.language
                ));
            }
            let json = serde_json::json!({"profiles": profiles});
            Ok(as_output(format, human, json))
        }
        ProfileCommands::Inspect { id } => {
            let p = inspect_profile(id)?;
            let human = render_profile_human(&p);
            let json = serde_json::to_value(&p).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        ProfileCommands::Resolve { id, features } => {
            let resolved = resolve_profile(id, features)?;
            let human = format!(
                "resolved {}@{} via {} (language {}, toolchain {})",
                resolved.id,
                resolved.version,
                resolved.adapter,
                resolved.language,
                resolved.toolchain
            );
            let json = serde_json::json!({"resolved": resolved});
            Ok(as_output(format, human, json))
        }
        ProfileCommands::Preflight { id } => {
            let report = preflight_profile(id, None)?;
            let human = format!(
                "preflight ok: toolchain '{}' available for profile '{}' (version {})",
                report.toolchain, report.id, report.version
            );
            let json = serde_json::json!({"preflight": report});
            Ok(as_output(format, human, json))
        }
    }
}

fn cmd_feature(
    db_path: &Path,
    command: &FeatureCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        FeatureCommands::List => {
            let features = feature_catalog();
            let mut human = format!(
                "{:<18} {:<10} {:<24} {}",
                "Feature", "Version", "Strategy", "Profiles"
            );
            for f in &features {
                human.push_str(&format!(
                    "\n{:<18} {:<10} {:<24} {}",
                    f.id,
                    f.version,
                    f.install_strategy,
                    f.compatible_profiles.join(",")
                ));
            }
            let json = serde_json::json!({"features": features});
            Ok(as_output(format, human, json))
        }
        FeatureCommands::Inspect { id } => {
            let f = inspect_feature(id)?;
            let human = render_feature_human(&f);
            let json = serde_json::to_value(&f).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        FeatureCommands::Resolve { profile, features } => {
            let plan = resolve_plan(profile, features)?;
            let human = render_feature_plan_human(&plan);
            let json = serde_json::json!({"plan": plan});
            Ok(as_output(format, human, json))
        }
        FeatureCommands::Add {
            target,
            feature,
            version,
        } => {
            let mut registry = open_registry(db_path)?;
            let outcome = add_feature(&mut registry, target, feature, version.as_deref())?;
            let human = render_outcome_human(&outcome);
            let json = serde_json::json!({"feature": outcome});
            Ok(as_output(format, human, json))
        }
        FeatureCommands::Remove { target, feature } => {
            let mut registry = open_registry(db_path)?;
            let outcome = remove_feature(&mut registry, target, feature)?;
            let human = render_outcome_human(&outcome);
            let json = serde_json::json!({"feature": outcome});
            Ok(as_output(format, human, json))
        }
        FeatureCommands::Upgrade {
            target,
            feature,
            version,
        } => {
            let mut registry = open_registry(db_path)?;
            let outcome = upgrade_feature(&mut registry, target, feature, version.as_deref())?;
            let human = render_outcome_human(&outcome);
            let json = serde_json::json!({"feature": outcome});
            Ok(as_output(format, human, json))
        }
    }
}

fn cmd_upgrade(
    db_path: &Path,
    target: &str,
    feature: Option<&str>,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let plan = plan_upgrade(&registry, target, feature)?;
    if dry_run {
        let human = render_upgrade_plan_human(&plan);
        let json = serde_json::json!({"plan": plan});
        return Ok(as_output(format, human, json));
    }
    drop(registry);
    let mut registry = open_registry(db_path)?;
    let outcome = apply_upgrade(&mut registry, target, feature)?;
    let human = render_upgrade_human(&outcome);
    let json = upgrade_outcome_json(&outcome);
    Ok(as_output(format, human, json))
}

fn upgrade_outcome_json(outcome: &UpgradeOutcome) -> serde_json::Value {
    serde_json::json!({
        "upgrade": {
            "operation": outcome.operation,
            "project_id": outcome.project_id,
            "profile": outcome.profile,
            "changed": outcome.changed,
            "note": outcome.note,
            "files_changed": outcome.files_changed,
            "features": outcome.features,
            "validation": outcome.validation,
            "recovery": outcome.recovery,
            "plan": outcome.plan,
        }
    })
}

fn cmd_upgrade_fleet(
    db_path: &Path,
    feature: Option<&str>,
    dry_run: bool,
    format: Format,
) -> ExitCode {
    let mut registry = match open_registry(db_path) {
        Ok(registry) => registry,
        Err(err) => {
            render_error(&err, format);
            return ExitCode::from(err.exit_code() as u8);
        }
    };
    let report = match run_fleet(&mut registry, feature, dry_run) {
        Ok(report) => report,
        Err(err) => {
            render_error(&err, format);
            return ExitCode::from(err.exit_code() as u8);
        }
    };
    let output = fleet_output(&report, format);
    match output {
        Output::Human(text) => {
            println!("{text}");
        }
        Output::Json(value) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
    }
    if report.healthy() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn cmd_spec(command: &SpecCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        SpecCommands::Generate {
            target,
            findings,
            reason,
        } => {
            let project_path = resolve_spec_target(target)?;
            let request = SpecRequest {
                project_path,
                finding_ids: findings.clone(),
                reason: reason.clone(),
            };
            ensure_single_project(&request)?;
            let sources = Vec::new();
            let now = chrono::Utc::now();
            let outcome = generate_spec(&request, &sources, now)?;
            spec_generate_output(&outcome, format)
        }
        SpecCommands::List { target } => {
            let project_path = resolve_spec_target(target)?;
            let entries = list_specs(&project_path)?;
            spec_list_output(&entries, format)
        }
        SpecCommands::Inspect { spec_id, target } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_spec_id(target, &project_path, spec_id)?;
            match read_spec(&project_path, &id)? {
                Some(draft) => spec_inspect_output(&draft, format),
                None => Err(ForgeError::SpecInvalid {
                    reason: format!("spec `{spec_id}` was not found under `.forge/specs/`"),
                }),
            }
        }
        SpecCommands::Route { finding, target } => {
            let project_path = resolve_spec_target(target)?;
            let request = SpecRequest {
                project_path,
                finding_ids: vec![finding.clone()],
                reason: None,
            };
            let source = finding_source_for(target, &request.project_path, finding)?;
            let decision = route_finding(&request, &source)?;
            spec_route_output(&decision, format)
        }
        SpecCommands::Apply {
            finding,
            target,
            reason,
        } => {
            let project_path = resolve_spec_target(target)?;
            let request = SpecRequest {
                project_path,
                finding_ids: vec![finding.clone()],
                reason: reason.clone(),
            };
            let source = finding_source_for(target, &request.project_path, finding)?;
            let now = chrono::Utc::now();
            let outcome = apply_routing(&request, &source, now)?;
            spec_apply_output(&outcome, format)
        }
    }
}

fn resolve_spec_target(target: &str) -> Result<std::path::PathBuf, ForgeError> {
    let candidate = std::path::Path::new(target);
    if candidate.is_dir() {
        return candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            });
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
}

fn parse_spec_id(
    _target: &str,
    project_path: &std::path::Path,
    raw: &str,
) -> Result<forge::spec::SpecId, ForgeError> {
    if raw.contains('-') {
        if let Some((project_id, hash)) = raw.split_once('-') {
            let candidate = forge::spec::SpecId {
                project_id: project_id.to_string(),
                hash: hash.to_string(),
            };
            if read_spec(project_path, &candidate)?.is_some() {
                return Ok(candidate);
            }
        }
    }
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(project_path, None)?;
    let entries = list_specs(project_path)?;
    let project_id = manifest.project.id.clone();
    let matches: Vec<&SpecListEntry> = entries
        .iter()
        .filter(|e| e.id.project_id == project_id && e.id.hash.starts_with(raw))
        .collect();
    match matches.len() {
        1 => Ok(matches[0].id.clone()),
        0 => Err(ForgeError::SpecInvalid {
            reason: format!("no spec id matches `{raw}` for project `{project_id}`"),
        }),
        _ => Err(ForgeError::SpecInvalid {
            reason: format!(
                "spec id `{raw}` is ambiguous for project `{project_id}`; pass the full `<project>-<hash>`"
            ),
        }),
    }
}

fn finding_source_for(
    target: &str,
    project_path: &std::path::Path,
    finding: &str,
) -> Result<FindingSource, ForgeError> {
    if let Some(stripped) = finding.strip_prefix("driftwatch-") {
        return Ok(FindingSource::Policy(forge::policy::PolicyFinding {
            id: stripped.to_string(),
            category: "spec".to_string(),
            severity: forge::policy::PolicySeverity::Fail,
            applicable: true,
            message: format!("policy finding `{stripped}`"),
            evidence: Vec::new(),
            reason: None,
        }));
    }
    if let Some(stripped) = finding.strip_prefix("semantic-") {
        return Ok(FindingSource::Conflict(forge::upgrade::SemanticConflict {
            project_id: manifest_id_for(project_path)?,
            feature: stripped.to_string(),
            owned_file: format!(".forge/features/{stripped}.receipt"),
            reason: "drifted receipt reported by the CLI".to_string(),
            suggested_spec: format!("forge spec generate --project {target} --finding {finding}"),
        }));
    }
    Ok(FindingSource::Doctor(DoctorFindingInput {
        id: finding.to_string(),
        status: FindingStatus::Fail,
        remediation: Remediation::Manual,
        category: "spec".to_string(),
        detail: format!("finding `{finding}` routed by `forge spec apply`"),
    }))
}

fn manifest_id_for(project_path: &std::path::Path) -> Result<String, ForgeError> {
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(project_path, None)?;
    Ok(manifest.project.id)
}

fn spec_generate_output(
    outcome: &SpecGenerateOutcome,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "status": outcome.status_label(),
        "note": outcome.note,
        "files_written": outcome.files_written,
        "spec": &outcome.spec,
    });
    let human = format!(
        "spec {}: {}\nfiles: {}\n{}",
        outcome.status_label(),
        outcome.spec.id.dir_name(),
        if outcome.files_written.is_empty() {
            "(none)".to_string()
        } else {
            outcome.files_written.join(", ")
        },
        outcome.note,
    );
    Ok(as_output(format, human, json))
}

fn spec_list_output(entries: &[SpecListEntry], format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"specs": entries});
    let mut human = format!(
        "{:<48} {:<16} {:<10} {}",
        "Spec", "Project", "Findings", "Contract"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<48} {:<16} {:<10} {}",
            entry.id.dir_name(),
            truncate(&entry.project_id, 16),
            entry.finding_ids.len().to_string(),
            entry.contract,
        ));
    }
    Ok(as_output(format, human, json))
}

fn spec_inspect_output(draft: &SpecDraft, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "spec": draft,
        "proposal": render_proposal_markdown(draft),
    });
    let human = render_proposal_markdown(draft);
    Ok(as_output(format, human, json))
}

fn spec_route_output(
    decision: &forge::spec::RoutingDecision,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "finding_id": decision.finding_id,
        "finding_category": decision.finding_category,
        "finding_severity": decision.finding_severity,
        "route": decision.route.label(),
        "rationale": decision.rationale,
        "action": decision.action,
        "suggested_spec": decision.suggested_spec,
    });
    let human = format!(
        "finding: {}\nroute: {}\nrationale: {}\naction: {}\nspec: {}",
        decision.finding_id,
        decision.route.label(),
        decision.rationale,
        decision
            .action
            .clone()
            .unwrap_or_else(|| "(none)".to_string()),
        decision
            .suggested_spec
            .as_ref()
            .map(|s| s.dir_name())
            .unwrap_or_else(|| "(none)".to_string()),
    );
    Ok(as_output(format, human, json))
}

fn spec_apply_output(outcome: &RoutingOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "apply": outcome,
    });
    let human = format!(
        "finding: {}\nroute: {}\nstatus: {}\nnote: {}\nevidence: {}\nrecovery: {}\nfiles: {}",
        outcome.decision.finding_id,
        outcome.decision.route.label(),
        outcome.status.label(),
        outcome.note,
        if outcome.evidence.is_empty() {
            "(none)".to_string()
        } else {
            outcome.evidence.join("; ")
        },
        if outcome.recovery.is_empty() {
            "(none)".to_string()
        } else {
            outcome.recovery.join("; ")
        },
        if outcome.files_changed.is_empty() {
            "(none)".to_string()
        } else {
            outcome.files_changed.join(", ")
        },
    );
    Ok(as_output(format, human, json))
}

fn fleet_output(report: &FleetReport, format: Format) -> Output {
    match format {
        Format::Human => Output::Human(render_fleet_human(report)),
        Format::Json => Output::Json(serde_json::json!({"fleet": report})),
    }
}

fn render_feature_human(f: &forge::feature::FeatureDescriptor) -> String {
    [
        format!("id: {}", f.id),
        format!("version: {}", f.version),
        format!("profiles: {}", f.compatible_profiles.join(", ")),
        format!(
            "depends: {}",
            if f.depends.is_empty() {
                "none".to_string()
            } else {
                f.depends.join(", ")
            }
        ),
        format!(
            "conflicts: {}",
            if f.conflicts.is_empty() {
                "none".to_string()
            } else {
                f.conflicts.join(", ")
            }
        ),
        format!("install: {}", f.install_strategy),
        format!("upgrade: {}", f.upgrade_strategy),
        format!(
            "validation: {}",
            if f.validation.is_empty() {
                "manifest re-parse and graph re-resolution".to_string()
            } else {
                f.validation.join(", ")
            }
        ),
        format!("docs: {}", f.documentation),
        format!("tests: {}", f.tests),
    ]
    .join("\n")
}

fn render_profile_human(p: &forge::profile::ProfileDescriptor) -> String {
    let mut lines = vec![
        format!(
            "id: {}\nversion: {}\nsupport_status: {}\nadapter: {}\nlanguage: {}\n",
            p.id,
            p.version,
            match p.support_status {
                forge::profile::ProfileSupportStatus::Supported => "supported",
                forge::profile::ProfileSupportStatus::Planned => "planned",
            },
            p.adapter,
            p.language,
        ),
        format!(
            "toolchain: {}{}",
            p.toolchain,
            p.toolchain_version
                .as_deref()
                .map(|v| format!("@{v}"))
                .unwrap_or_default()
        ),
        format!("capabilities: {}", p.capabilities.join(", ")),
        format!("packages: {}", p.packages.join(", ")),
        format!("layout: {}", p.layout.join(", ")),
        format!("conventions: {}", p.conventions.join(", ")),
        format!("build: {}", p.build_command),
        format!("test: {}", p.test_command),
        format!(
            "deployment: {}",
            match (&p.deployment_type, &p.deployment_target) {
                (Some(t), Some(target)) => format!("{t} -> {target}"),
                (Some(t), None) => t.clone(),
                (None, Some(target)) => target.clone(),
                (None, None) => "unknown".to_string(),
            }
        ),
        format!("quality: {}", p.quality_policies.join(", ")),
        format!(
            "requires_database: {}",
            if p.requires_database { "yes" } else { "no" }
        ),
    ];
    if let Some(desc) = &p.description {
        lines.push(format!("description: {desc}"));
    }
    lines.join("\n")
}

fn render_record_human(p: &ProjectRecord) -> String {
    let mut lines = vec![
        format!("id: {}", p.id),
        format!("name: {}", p.name),
        format!("path: {}", p.path),
        format!("profile: {}", p.profile),
        format!("schema: {}", p.schema_version),
        format!("platform: {}", p.platform_version),
        format!("maturity: {}", p.maturity.as_deref().unwrap_or("unknown")),
        format!(
            "target_maturity: {}",
            p.target_maturity.as_deref().unwrap_or("unknown")
        ),
        format!("stack: {}", p.stack.as_deref().unwrap_or("unknown")),
        format!("runtime: {}", p.runtime.as_deref().unwrap_or("unknown")),
        format!(
            "deployment_target: {}",
            p.deployment_target.as_deref().unwrap_or("unknown")
        ),
        format!(
            "git_remote: {}",
            p.git_remote.as_deref().unwrap_or("unknown")
        ),
        format!(
            "last_commit: {}",
            p.last_commit.as_deref().unwrap_or("unknown")
        ),
        format!(
            "quality: {}",
            p.quality_status.as_deref().unwrap_or("unknown")
        ),
        format!("agent: {}", p.agent_status.as_deref().unwrap_or("unknown")),
        format!("docs: {}", p.docs_status.as_deref().unwrap_or("unknown")),
        format!("observed_at: {}", p.observed_at),
        format!(
            "availability: {}",
            if p.available {
                "available"
            } else {
                "unavailable"
            }
        ),
    ];
    if p.mirror_remotes.is_empty() {
        lines.push("mirrors: none".to_string());
    } else {
        lines.push(format!("mirrors: {}", p.mirror_remotes.join(", ")));
    }
    if p.features.is_empty() {
        lines.push("features: none".to_string());
    } else {
        let feats: Vec<String> = p.features.iter().map(|(k, v)| format!("{k}={v}")).collect();
        lines.push(format!("features: {}", feats.join(", ")));
    }
    lines.join("\n")
}

fn render_error(err: &ForgeError, format: Format) {
    match format {
        Format::Human => eprintln!("error[{}]: {}", err.code(), err),
        Format::Json => {
            let value = serde_json::json!({
                "error": {"code": err.code(), "message": err.to_string()}
            });
            eprintln!("{}", serde_json::to_string(&value).unwrap());
        }
    }
}

fn truncate(s: &str, width: usize) -> String {
    if s.len() <= width {
        s.to_string()
    } else {
        format!("{}…", &s[..width.saturating_sub(1)])
    }
}

fn parse_provider(raw: &str) -> Result<AgentProvider, ForgeError> {
    match raw {
        "opencode" => Ok(AgentProvider::Opencode),
        "codex" => Ok(AgentProvider::Codex),
        other => Err(ForgeError::AgentUnavailable {
            reason: format!("unknown agent provider `{other}`; expected one of: opencode, codex"),
        }),
    }
}

fn resolve_agent_project(target: &str) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let canonical = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
        return Ok((canonical, manifest.project.id));
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
}

fn cmd_agent(
    db_path: &Path,
    command: &AgentCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        AgentCommands::Start {
            target,
            session,
            provider,
            spec,
        } => {
            let (path, project_id) = resolve_agent_project(target)?;
            let provider = parse_provider(provider)?;
            let spec_id = spec.clone().unwrap_or_default();
            let now = chrono::Utc::now();
            let session_rec = new_session(&path, session, provider, &spec_id, now)?;
            let outcome = apply_agent_transition(session_rec, SessionTransition::Start, now)?;
            let files = forge::agent::write_session(&path, &outcome.session)?;
            let registry = open_registry(db_path)?;
            let detail = format!(
                "agent `{}` session `{}` provider `{}` spec `{}` -> `{}`",
                project_id,
                session,
                provider.label(),
                spec_id,
                outcome.state.label()
            );
            let _ = registry.record_operation("agent", &project_id, "done", &detail);
            agent_outcome_output(&outcome, &files, &detail, format)
        }
        AgentCommands::Pause { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            run_agent_transition(
                db_path,
                &path,
                &project_id,
                session,
                SessionTransition::Pause,
                format,
            )
        }
        AgentCommands::Takeover { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            run_agent_transition(
                db_path,
                &path,
                &project_id,
                session,
                SessionTransition::Takeover,
                format,
            )
        }
        AgentCommands::Resume { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            run_agent_transition(
                db_path,
                &path,
                &project_id,
                session,
                SessionTransition::Resume,
                format,
            )
        }
        AgentCommands::Restart { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            run_agent_transition(
                db_path,
                &path,
                &project_id,
                session,
                SessionTransition::Restart,
                format,
            )
        }
        AgentCommands::NewSession {
            target,
            session,
            new_session: new_id,
        } => {
            let (path, project_id) = resolve_agent_project(target)?;
            let now = chrono::Utc::now();
            let prior =
                read_session(&path, session)?.ok_or_else(|| ForgeError::AgentUnavailable {
                    reason: format!("session `{session}` was not found under `.forge/agents/`"),
                })?;
            let new_rec = new_session(&path, new_id, prior.provider, &prior.spec_id, now)?;
            let outcome = apply_agent_transition(new_rec, SessionTransition::NewSession, now)?;
            let files = forge::agent::write_session(&path, &outcome.session)?;
            let registry = open_registry(db_path)?;
            let detail = format!(
                "agent `{}` new session `{}` from `{}` -> `{}`",
                project_id,
                new_id,
                session,
                outcome.state.label()
            );
            let _ = registry.record_operation("agent", &project_id, "done", &detail);
            agent_outcome_output(&outcome, &files, &detail, format)
        }
        AgentCommands::Status { target, session } => {
            let (path, _) = resolve_agent_project(target)?;
            let session_rec =
                status_for(&path, session)?.ok_or_else(|| ForgeError::AgentUnavailable {
                    reason: format!("session `{session}` was not found under `.forge/agents/`"),
                })?;
            let json = serde_json::to_value(&session_rec).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            let human = render_session_human(&session_rec);
            Ok(as_output(format, human, json))
        }
        AgentCommands::List { target } => {
            let (path, _) = resolve_agent_project(target)?;
            let entries = list_sessions(&path)?;
            agent_list_output(&entries, format)
        }
        AgentCommands::RunSpec { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            let now = chrono::Utc::now();
            let outcome = run_session_spec(&path, session, now)?;
            let files = forge::agent::write_session(&path, &outcome.session)?;
            let registry = open_registry(db_path)?;
            let detail = format!(
                "agent `{}` run_spec session `{}` -> `{}`",
                project_id,
                session,
                outcome.state.label()
            );
            let _ = registry.record_operation("agent", &project_id, "done", &detail);
            agent_outcome_output(&outcome, &files, &detail, format)
        }
    }
}

fn run_agent_transition(
    db_path: &Path,
    path: &Path,
    project_id: &str,
    session_id: &str,
    transition: SessionTransition,
    format: Format,
) -> Result<Output, ForgeError> {
    let now = chrono::Utc::now();
    let session = read_session(path, session_id)?.ok_or_else(|| ForgeError::AgentUnavailable {
        reason: format!("session `{session_id}` was not found under `.forge/agents/`"),
    })?;
    let outcome = apply_agent_transition(session, transition, now)?;
    let files = forge::agent::write_session(path, &outcome.session)?;
    let registry = open_registry(db_path)?;
    let detail = format!(
        "agent `{}` session `{}` {} -> `{}`",
        project_id,
        session_id,
        transition.label(),
        outcome.state.label()
    );
    let _ = registry.record_operation("agent", project_id, "done", &detail);
    agent_outcome_output(&outcome, &files, &detail, format)
}

fn agent_outcome_output(
    outcome: &AgentTransitionOutcome,
    files: &[String],
    detail: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "transition": {
            "session_id": outcome.session.session_id,
            "project_id": outcome.session.project_id,
            "provider": outcome.session.provider.label(),
            "spec_id": outcome.session.spec_id,
            "state": outcome.state.label(),
            "requested": outcome.requested.label(),
            "evidence": outcome.evidence,
            "next_step": outcome.next_step,
            "note": outcome.note,
            "files_written": files,
            "registry_detail": detail,
        }
    });
    let human = format!(
        "session: {}\nproject: {}\nprovider: {}\nspec: {}\nrequested: {}\nstate: {}\nevidence: {}\nnext_step: {}\nnote: {}\nfiles: {}",
        outcome.session.session_id,
        outcome.session.project_id,
        outcome.session.provider.label(),
        outcome.session.spec_id,
        outcome.requested.label(),
        outcome.state.label(),
        if outcome.evidence.is_empty() { "(none)".to_string() } else { outcome.evidence.join("; ") },
        outcome.next_step.clone().unwrap_or_else(|| "(none)".to_string()),
        outcome.note,
        if files.is_empty() { "(none)".to_string() } else { files.join(", ") },
    );
    Ok(as_output(format, human, json))
}

fn agent_list_output(
    entries: &[forge::agent::SessionListEntry],
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"sessions": entries});
    let mut human = format!(
        "{:<24} {:<12} {:<12} {:<8} {}",
        "Session", "Provider", "State", "Specs", "Transitions"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<24} {:<12} {:<12} {:<8} {}",
            entry.session_id,
            entry.provider.label(),
            entry.state.label(),
            entry.spec_id,
            entry.transition_count
        ));
    }
    Ok(as_output(format, human, json))
}

fn render_session_human(session: &forge::agent::AgentSession) -> String {
    let mut lines = vec![
        format!("session: {}", session.session_id),
        format!("project: {}", session.project_id),
        format!("provider: {}", session.provider.label()),
        format!("spec: {}", session.spec_id),
        format!("state: {}", session.state.label()),
        format!("started_at: {}", session.started_at.to_rfc3339()),
        format!(
            "last_transition_at: {}",
            session.last_transition_at.to_rfc3339()
        ),
    ];
    if session.transitions.is_empty() {
        lines.push("transitions: (none)".to_string());
    } else {
        lines.push("transitions:".to_string());
        for t in &session.transitions {
            lines.push(format!(
                "  - {} {} state={} evidence={} note={}",
                t.at.to_rfc3339(),
                t.kind.label(),
                t.state.label(),
                t.evidence.join(";"),
                t.note
            ));
        }
    }
    lines.join("\n")
}

fn cmd_test(path: &Path, format: Format) -> Result<Output, ForgeError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: path.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let descriptor = forge::profile::inspect_profile(&manifest.project.profile)?;
    let test_command = descriptor.test_command.clone();
    let outcome = run_test(
        &manifest.project.id,
        &manifest.project.profile,
        Some(&test_command),
        &canonical,
    )?;
    test_output(&outcome, format)
}

fn test_output(outcome: &TestOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "test": {
            "contract": outcome.contract,
            "project_id": outcome.project_id,
            "profile": outcome.profile,
            "command": outcome.command,
            "status": outcome.status,
            "exit_code": outcome.exit_code,
            "note": outcome.note,
            "evidence": outcome.evidence,
        }
    });
    let human = format!(
        "project: {}\nprofile: {}\ncommand: {}\nstatus: {}\nexit_code: {}\nnote: {}\nevidence: {}",
        outcome.project_id,
        outcome.profile,
        outcome.command,
        outcome.status,
        outcome
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "(none)".to_string()),
        outcome.note,
        if outcome.evidence.is_empty() {
            "(none)".to_string()
        } else {
            outcome.evidence.join(" | ")
        }
    );
    Ok(as_output(format, human, json))
}

fn cmd_commit(
    path: &Path,
    paths: Vec<String>,
    message: String,
    format: Format,
) -> Result<Output, ForgeError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: path.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let outcome = commit_paths(&manifest.project.id, &canonical, &paths, &message)?;
    commit_output(&outcome, format)
}

fn commit_output(outcome: &CommitOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "commit": {
            "contract": outcome.contract,
            "project_id": outcome.project_id,
            "paths": outcome.paths,
            "message": outcome.message,
            "commit_sha": outcome.commit_sha,
            "files_changed": outcome.files_changed,
            "note": outcome.note,
            "evidence": outcome.evidence,
        }
    });
    let human = format!(
        "project: {}\npaths: {}\nfiles_changed: {}\ncommit: {}\nnote: {}",
        outcome.project_id,
        outcome.paths.join(", "),
        outcome.files_changed.join(", "),
        outcome
            .commit_sha
            .clone()
            .unwrap_or_else(|| "(none)".to_string()),
        outcome.note
    );
    Ok(as_output(format, human, json))
}

fn cmd_mcp(db_path: &Path, command: &McpCommands) -> Result<Output, ForgeError> {
    match command {
        McpCommands::Serve => {
            forge::mcp::serve_stdio(Some(db_path)).map_err(|err| ForgeError::McpInvalid {
                reason: err.to_string(),
            })?;
            Ok(Output::Human("mcp server exited cleanly".to_string()))
        }
    }
}

fn cmd_push(
    path: &Path,
    remote: String,
    ref_name: String,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: path.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let outcome = push_ref(
        &manifest.project.id,
        &canonical,
        &remote,
        &ref_name,
        confirm,
    )?;
    push_output(&outcome, format)
}

fn push_output(outcome: &PushOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "push": {
            "contract": outcome.contract,
            "project_id": outcome.project_id,
            "remote": outcome.remote,
            "ref": outcome.ref_name,
            "status": outcome.status,
            "commit_sha": outcome.commit_sha,
            "note": outcome.note,
            "evidence": outcome.evidence,
        }
    });
    let human = format!(
        "project: {}\nremote: {}\nref: {}\nstatus: {}\ncommit: {}\nnote: {}",
        outcome.project_id,
        outcome.remote,
        outcome.ref_name,
        outcome.status,
        outcome
            .commit_sha
            .clone()
            .unwrap_or_else(|| "(none)".to_string()),
        outcome.note
    );
    Ok(as_output(format, human, json))
}

fn cmd_mirror(
    db_path: &Path,
    target: &str,
    refs: Vec<String>,
    confirm: bool,
    dry_run: bool,
    retry_failed: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let candidate = Path::new(target);
    if !candidate.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: target.to_string(),
        });
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: target.to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let config: DistributionConfig = distribution_config_from_manifest(&manifest)?;
    let request = MirrorRequest {
        project_id: manifest.project.id.clone(),
        refs,
        confirm,
        dry_run,
        retry_failed,
    };
    let report = if dry_run {
        let state_path = forge::distribution::state_path_for(&canonical, &manifest.project.id)?;
        let state = forge::distribution::load_mirror_state(&state_path)?;
        plan_mirror(&config, &request, &state, &state_path)?
    } else {
        apply_mirror(&canonical, &config, &request)?
    };
    let registry = open_registry(db_path)?;
    let detail = format!(
        "mirror `{}` refs={} primary={} healthy={} dry_run={} retry={}",
        report.project_id,
        report.refs.join(","),
        report
            .primary
            .clone()
            .unwrap_or_else(|| "(none)".to_string()),
        report.healthy,
        report.dry_run,
        report.retry_failed
    );
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("mirror", &report.project_id, state_label, &detail);
    let output = mirror_output(&report, format)?;
    if report.healthy() {
        Ok(output)
    } else {
        // Partial failure: print the per-remote outcome
        // JSON to stdout so the caller sees exactly which
        // remotes succeeded and which failed, then surface
        // a typed exit-code error referencing the
        // distribution verdict. The error is rendered on
        // stderr by the generic error path so the exit
        // code matches the verdict without hiding the
        // per-remote evidence.
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
        }
        Err(ForgeError::DistributionInvalid {
            reason: report.note.clone(),
        })
    }
}

fn mirror_output(
    report: &forge::distribution::MirrorReport,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"mirror": report});
    let human = forge::distribution::render_report_human(report);
    Ok(as_output(format, human, json))
}

fn cmd_docs(db_path: &Path, command: &DocsCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        DocsCommands::Translate {
            locale,
            all,
            project,
        } => cmd_docs_translate(db_path, project, locale.clone(), *all, format),
    }
}

fn cmd_docs_translate(
    db_path: &Path,
    project: &Path,
    locale: Option<String>,
    all: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let canonical = project
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: project.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let config = docs_config_from_manifest(&manifest)?;
    let translator = TranslatorConfig::from_env();
    let request = TranslateRequest {
        project_id: manifest.project.id.clone(),
        locale,
        all,
    };
    let report = run_translate(&canonical, &config, &request, &translator)?;
    let registry = open_registry(db_path)?;
    let statuses: Vec<String> = report
        .outcomes
        .iter()
        .map(|o| format!("{}={}", o.locale, o.status))
        .collect();
    let detail = format!(
        "docs translate `{}` locales={} healthy={} all={}",
        report.project_id,
        statuses.join(","),
        report.healthy,
        report.all
    );
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("docs", &report.project_id, state_label, &detail);
    let output = docs_translate_output(&report, format)?;
    if report.healthy() {
        Ok(output)
    } else {
        // Partial failure: print the per-locale outcome JSON to
        // stdout so the caller sees exactly which locales were
        // translated and which failed, then surface a typed
        // exit-code error. The prior derivatives are intact.
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
        }
        Err(ForgeError::TranslationFailed {
            reason: report.note.clone(),
        })
    }
}

fn docs_translate_output(report: &TranslateReport, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"translate": report});
    let human = forge::docs::render_report_human(report);
    Ok(as_output(format, human, json))
}

fn cmd_release(
    db_path: &Path,
    command: &ReleaseCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ReleaseCommands::Prepare {
            target,
            version,
            dry_run,
        } => cmd_release_prepare(db_path, target, version, *dry_run, format),
        ReleaseCommands::Apply {
            target,
            version,
            confirm,
            retry,
            dry_run,
            stages,
        } => cmd_release_apply(
            db_path, target, version, *confirm, *retry, *dry_run, stages, format,
        ),
        ReleaseCommands::List { target } => cmd_release_list(db_path, target, format),
        ReleaseCommands::Inspect { release_id, target } => {
            cmd_release_inspect(db_path, target, release_id, format)
        }
    }
}

fn resolve_release_target(target: &str) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let canonical = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
        return Ok((canonical, manifest.project.id));
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
}

fn cmd_release_prepare(
    db_path: &Path,
    target: &str,
    version: &str,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = resolve_release_target(target)?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&project_dir, None)?;
    let config = release_config_from_manifest(&manifest)?;
    let semver = Semver::parse(version)?;
    let request = ReleaseRequest {
        project_id: project_id.clone(),
        version: semver,
        confirm: false,
        dry_run,
        retry: false,
        stages: config.stages.clone(),
    };
    let policy = DriftWatchConfig::from_env();
    let plan = prepare_release(&project_dir, &manifest, &config, &request, &policy)?;
    let registry = open_registry(db_path)?;
    let detail = format!(
        "release prepare `{}` v{} revision=`{}` ready={} dry_run={}",
        project_id, plan.identity.version, plan.identity.source_revision, plan.ready, dry_run
    );
    let state_label = if plan.healthy() { "done" } else { "blocked" };
    let _ = registry.record_operation("release", &project_id, state_label, &detail);
    release_plan_output(&plan, format)
}

fn release_plan_output(plan: &EnginePlanReport, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"plan": plan});
    let human = forge::release::engine::render_plan_human(plan);
    Ok(as_output(format, human, json))
}

#[allow(clippy::too_many_arguments)]
fn cmd_release_apply(
    db_path: &Path,
    target: &str,
    version: &str,
    confirm: bool,
    retry: bool,
    dry_run: bool,
    stages: &[String],
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = resolve_release_target(target)?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&project_dir, None)?;
    let mut config = release_config_from_manifest(&manifest)?;
    if !stages.is_empty() {
        config = config.with_stages(stages.to_vec());
    }
    let semver = Semver::parse(version)?;
    let request = ReleaseRequest {
        project_id: project_id.clone(),
        version: semver,
        confirm,
        dry_run,
        retry,
        stages: config.stages.clone(),
    };
    let adapters = ReleaseAdapterConfig::from_env();
    let report = match apply_release(&project_dir, &manifest, &config, &request, &adapters) {
        Ok(report) => report,
        Err(ForgeError::ReleaseCheckFailed { reason }) => {
            return Err(ForgeError::ReleaseCheckFailed { reason });
        }
        Err(err) => return Err(err),
    };
    let registry = open_registry(db_path)?;
    let detail = format!(
        "release apply `{}` v{} revision=`{}` stages={} dry_run={} retry={} healthy={}",
        report.project_id,
        report.identity.version,
        report.identity.source_revision,
        report.stage_outcomes.len(),
        report.dry_run,
        report.retry,
        report.healthy
    );
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("release", &project_id, state_label, &detail);
    let output = release_report_output(&report, format)?;
    if report.healthy() {
        Ok(output)
    } else {
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
        }
        Err(ForgeError::ReleaseCheckFailed {
            reason: report.note.clone(),
        })
    }
}

fn release_report_output(report: &ReleaseReport, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"release": report});
    let human = render_release_report_human(report);
    Ok(as_output(format, human, json))
}

fn cmd_release_list(db_path: &Path, target: &str, format: Format) -> Result<Output, ForgeError> {
    let _ = db_path;
    let (project_dir, project_id) = resolve_release_target(target)?;
    let entries = list_releases(&project_dir, &project_id)?;
    release_list_output(&entries, &project_id, format)
}

fn release_list_output(
    entries: &[ReleaseListEntry],
    project_id: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"releases": entries, "project_id": project_id});
    if entries.is_empty() {
        let human = format!("no releases for project `{project_id}`");
        return Ok(as_output(format, human, json));
    }
    let mut human = format!(
        "{:<48} {:<12} {:<14} {:<8} {}",
        "Release", "Project", "Version", "Stages", "Last Run"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<48} {:<12} {:<14} {:<8} {}",
            entry.release_id,
            truncate(&entry.project_id, 12),
            entry.version,
            entry.stage_count.to_string(),
            entry.last_run_at
        ));
    }
    Ok(as_output(format, human, json))
}

fn cmd_release_inspect(
    db_path: &Path,
    target: &str,
    release_id: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let _ = db_path;
    let (project_dir, project_id) = resolve_release_target(target)?;
    let state = match read_release(&project_dir, &project_id, release_id)? {
        Some(state) => state,
        None => {
            return Err(ForgeError::ReleaseInvalid {
                reason: format!(
                    "release `{release_id}` was not found under `.forge/release/{project_id}/`"
                ),
            });
        }
    };
    release_state_output(&state, format)
}

fn release_state_output(state: &ReleaseState, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"release": state});
    let human = render_release_report_human_for_state(state);
    Ok(as_output(format, human, json))
}

fn render_release_report_human_for_state(state: &ReleaseState) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", state.identity.project_id));
    lines.push(format!("release_id: {}", state.identity.id));
    lines.push(format!("version: {}", state.identity.version));
    lines.push(format!(
        "source_revision: {}",
        state.identity.source_revision
    ));
    if let Some(changelog) = &state.changelog {
        lines.push(format!("changelog: {}", changelog.path));
        lines.push(format!("changelog_hash: {}", changelog.content_hash));
    }
    if !state.docs_locales.is_empty() {
        lines.push(format!("docs_locales: {}", state.docs_locales.join(", ")));
    }
    lines.push(format!("stages: {}", state.stages.join(", ")));
    lines.push(format!("last_run_at: {}", state.last_run_at));
    if !state.checks.is_empty() {
        lines.push("checks:".to_string());
        for check in &state.checks {
            lines.push(format!(
                "  - {} {} applicable={} revision={}: {}",
                check.kind, check.status, check.applicable, check.source_revision, check.detail
            ));
        }
    }
    if !state.stage_outcomes.is_empty() {
        lines.push("stage_outcomes:".to_string());
        for outcome in &state.stage_outcomes {
            lines.push(format!(
                "  - {} target=`{}` {}: {}",
                outcome.stage, outcome.target, outcome.status, outcome.note
            ));
            for line in &outcome.evidence {
                lines.push(format!("      evidence: {line}"));
            }
            for line in &outcome.recovery {
                lines.push(format!("      recovery: {line}"));
            }
        }
    }
    lines.join("\n")
}

fn resolve_deploy_target(target: &str) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let canonical = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
        return Ok((canonical, manifest.project.id));
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
}

fn cmd_deploy(
    db_path: &Path,
    command: &DeployCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        DeployCommands::Plan {
            target,
            target_name,
        } => {
            let (project_dir, project_id) = resolve_deploy_target(target)?;
            let (manifest, config) = forge::deploy::engine::load_config(&project_dir)?;
            let target_name = resolve_deploy_target_name(&config, target_name.as_deref())?;
            let request = DeployRequest {
                project_id: project_id.clone(),
                target: target_name,
                confirm: false,
                dry_run: true,
            };
            let plan =
                forge::deploy::engine::prepare_deploy(&project_dir, &manifest, &config, &request)?;
            let registry = open_registry(db_path)?;
            let detail = format!(
                "deploy plan `{}` target=`{}` adapter=`{}` ready={}",
                plan.identity.id, plan.target.name, plan.target.kind, plan.ready
            );
            let state_label = if plan.healthy() { "done" } else { "blocked" };
            let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
            deploy_plan_output(&plan, format)
        }
        DeployCommands::Apply {
            target,
            target_name,
            confirm,
            dry_run,
        } => cmd_deploy_apply(
            db_path,
            target,
            target_name.as_deref(),
            *confirm,
            *dry_run,
            format,
        ),
        DeployCommands::Observe {
            target,
            target_name,
            confirm,
        } => cmd_deploy_observe(db_path, target, target_name.as_deref(), *confirm, format),
        DeployCommands::List { target } => {
            let (project_dir, project_id) = resolve_deploy_target(target)?;
            let entries = forge::deploy::engine::list_deploys(&project_dir, &project_id)?;
            deploy_list_output(&entries, &project_id, format)
        }
        DeployCommands::Inspect { deploy_id, target } => {
            let (project_dir, project_id) = resolve_deploy_target(target)?;
            let state =
                match forge::deploy::engine::read_deploy(&project_dir, &project_id, deploy_id)? {
                    Some(state) => state,
                    None => {
                        return Err(ForgeError::DeployInvalid {
                            reason: format!(
                            "deploy `{deploy_id}` was not found under `.forge/deploy/{project_id}/`"
                        ),
                        });
                    }
                };
            deploy_state_output(&state, format)
        }
    }
}

fn resolve_deploy_target_name(
    config: &forge::deploy::DeployConfig,
    requested: Option<&str>,
) -> Result<String, ForgeError> {
    if let Some(name) = requested {
        return Ok(config.target(name)?.name.clone());
    }
    Ok(config.default_target.clone())
}

fn cmd_deploy_apply(
    db_path: &Path,
    target: &str,
    target_name: Option<&str>,
    confirm: bool,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = resolve_deploy_target(target)?;
    let (manifest, config) = forge::deploy::engine::load_config(&project_dir)?;
    let target_name = resolve_deploy_target_name(&config, target_name)?;
    let request = DeployRequest {
        project_id: project_id.clone(),
        target: target_name,
        confirm,
        dry_run,
    };
    let adapters = DeployAdapterConfig::from_env();
    let report =
        forge::deploy::engine::apply_deploy(&project_dir, &manifest, &config, &request, &adapters)?;
    let registry = open_registry(db_path)?;
    let detail = forge::deploy::engine::journal_report(&report, report.healthy);
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
    let output = deploy_report_output(&report, format)?;
    if report.healthy() {
        Ok(output)
    } else {
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
        }
        Err(ForgeError::DeployHealthFailed {
            reason: report.note.clone(),
        })
    }
}

fn cmd_deploy_observe(
    db_path: &Path,
    target: &str,
    target_name: Option<&str>,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = resolve_deploy_target(target)?;
    let (manifest, config) = forge::deploy::engine::load_config(&project_dir)?;
    let target_name = resolve_deploy_target_name(&config, target_name)?;
    let request = DeployRequest {
        project_id: project_id.clone(),
        target: target_name,
        confirm,
        dry_run: false,
    };
    let adapters = DeployAdapterConfig::from_env();
    let report = forge::deploy::engine::observe_deploy(
        &project_dir,
        &manifest,
        &config,
        &request,
        &adapters,
    )?;
    let registry = open_registry(db_path)?;
    let detail = forge::deploy::engine::journal_report(&report, report.healthy);
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
    let output = deploy_report_output(&report, format)?;
    if report.healthy() {
        Ok(output)
    } else {
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
        }
        Err(ForgeError::DeployHealthFailed {
            reason: report.note.clone(),
        })
    }
}

fn deploy_plan_output(
    plan: &forge::deploy::DeployPlan,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"plan": plan});
    let human = forge::deploy::engine::render_plan_human(plan);
    Ok(as_output(format, human, json))
}

fn deploy_report_output(
    report: &forge::deploy::DeployReport,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"deploy": report});
    let human = forge::deploy::render_report_human(report);
    Ok(as_output(format, human, json))
}

fn deploy_list_output(
    entries: &[forge::deploy::DeployListEntry],
    project_id: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"deploys": entries, "project_id": project_id});
    if entries.is_empty() {
        let human = format!("no deploys for project `{project_id}`");
        return Ok(as_output(format, human, json));
    }
    let mut human = format!(
        "{:<48} {:<12} {:<10} {:<10} {}",
        "Deploy", "Project", "Target", "State", "Last Run"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<48} {:<12} {:<10} {:<10} {}",
            entry.deploy_id,
            truncate(&entry.project_id, 12),
            entry.target,
            entry.current_state,
            entry.last_run_at
        ));
    }
    Ok(as_output(format, human, json))
}

fn deploy_state_output(
    state: &forge::deploy::DeployState,
    format: Format,
) -> Result<Output, ForgeError> {
    let current_state = state.current_state();
    let mut deploy_value = serde_json::to_value(state).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    if let Some(obj) = deploy_value.as_object_mut() {
        obj.insert(
            "current_state".to_string(),
            serde_json::Value::String(current_state),
        );
    }
    let json = serde_json::json!({"deploy": deploy_value});
    let human = render_deploy_state_human(state);
    Ok(as_output(format, human, json))
}

fn render_deploy_state_human(state: &forge::deploy::DeployState) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", state.identity.project_id));
    lines.push(format!("deploy_id: {}", state.identity.id));
    lines.push(format!(
        "target: {} ({})",
        state.target.name, state.target.kind
    ));
    lines.push(format!(
        "source_revision: {}",
        state.identity.source_revision
    ));
    lines.push(format!("adapter: {}", state.adapter));
    if let Some(artifact) = &state.artifact {
        lines.push(format!("artifact: {}", artifact.path));
        lines.push(format!("artifact_hash: {}", artifact.content_hash));
    }
    if let Some(health) = &state.health {
        lines.push(format!(
            "health: {} ({})",
            health.kind,
            health_label_for_state(health)
        ));
    }
    lines.push(format!("current_state: {}", state.current_state()));
    lines.push(format!("last_run_at: {}", state.last_run_at));
    if let Some(obs) = &state.last_observation {
        lines.push(format!(
            "last_observation: {} at {}: {}",
            obs.status, obs.observed_at, obs.detail
        ));
        for line in &obs.evidence {
            lines.push(format!("      evidence: {line}"));
        }
    }
    if !state.stage_outcomes.is_empty() {
        lines.push("stage_outcomes:".to_string());
        for outcome in &state.stage_outcomes {
            lines.push(format!(
                "  - {} target=`{}` {}: {}",
                outcome.stage, outcome.target, outcome.status, outcome.note
            ));
            for line in &outcome.evidence {
                lines.push(format!("      evidence: {line}"));
            }
            for line in &outcome.recovery {
                lines.push(format!("      recovery: {line}"));
            }
        }
    }
    lines.join("\n")
}

fn health_label_for_state(health: &forge::deploy::DeployHealthSpec) -> String {
    match health.kind.as_str() {
        forge::deploy::HEALTH_DOCKER => {
            format!("service={}", health.service.as_deref().unwrap_or("?"))
        }
        forge::deploy::HEALTH_HTTP => {
            format!("url={}", health.url.as_deref().unwrap_or("?"))
        }
        forge::deploy::HEALTH_PROCESS => {
            format!("process={}", health.process.as_deref().unwrap_or("?"))
        }
        other => other.to_string(),
    }
}

fn cmd_component(
    db_path: &Path,
    command: &ComponentCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ComponentCommands::List => {
            let catalog = component_catalog();
            let entries: Vec<serde_json::Value> = catalog
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "version": c.version,
                        "quality": c.quality.label(),
                        "profiles": c.profiles,
                        "purpose": c.purpose,
                    })
                })
                .collect();
            let mut human = format!(
                "{:<28} {:<10} {:<8} {}",
                "Component", "Version", "Quality", "Profiles"
            );
            for c in &catalog {
                human.push_str(&format!(
                    "\n{:<28} {:<10} {:<8} {}",
                    c.id,
                    c.version,
                    c.quality.label(),
                    c.profiles.join(",")
                ));
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({"components": entries}),
            ))
        }
        ComponentCommands::Inspect { id } => {
            let descriptor = inspect_component(id)?;
            let json = serde_json::json!({
                "id": descriptor.id,
                "version": descriptor.version,
                "purpose": descriptor.purpose,
                "quality": descriptor.quality.label(),
                "profiles": descriptor.profiles,
                "depends_on": descriptor.depends_on,
                "install_strategy": descriptor.install_strategy,
                "validation": descriptor.validation,
                "documentation": descriptor.documentation,
                "tests": descriptor.tests,
                "contract": {
                    "inputs": descriptor.contract.inputs,
                    "outputs": descriptor.contract.outputs,
                },
                "evidence": descriptor.evidence,
            });
            let mut human = vec![
                format!("component: {}@{}", descriptor.id, descriptor.version),
                format!("quality: {}", descriptor.quality.label()),
                format!("purpose: {}", descriptor.purpose),
                format!("profiles: {}", descriptor.profiles.join(", ")),
                format!("install: {}", descriptor.install_strategy),
                format!("tests: {}", descriptor.tests),
                format!("documentation: {}", descriptor.documentation),
                "contract inputs:".to_string(),
            ];
            for port in &descriptor.contract.inputs {
                human.push(format!("  - {}: {}", port.name, port.description));
            }
            human.push("contract outputs:".to_string());
            for port in &descriptor.contract.outputs {
                human.push(format!("  - {}: {}", port.name, port.description));
            }
            human.push("evidence:".to_string());
            human.push(format!(
                "  usage={} coverage={:.2} security_review={} last_verified={}",
                descriptor.evidence.usage_count,
                descriptor.evidence.test_coverage,
                descriptor.evidence.security_review,
                descriptor.evidence.last_verified.to_rfc3339()
            ));
            Ok(as_output(format, human.join("\n"), json))
        }
        ComponentCommands::Resolve {
            profile,
            components,
        } => {
            let request = forge::component::ComponentRequest {
                profile: profile.clone(),
                component_ids: components.clone(),
            };
            let outcome = resolve_outcome(&request)?;
            let journal_state = if outcome.plan.steps.is_empty() {
                "rejected"
            } else {
                "done"
            };
            let summary = format!(
                "{} resolved={} rejected={}",
                request.profile,
                outcome.plan.steps.len(),
                outcome.plan.rejections.len()
            );
            let note = format!("{}; {}", summary, outcome.note);
            // Journal: component operations are catalog-global; the
            // synthetic `__component__` project id keeps the
            // registry contract satisfied without inventing a
            // user-visible project.
            if let Ok(registry) = open_registry(db_path) {
                let _ =
                    registry.record_operation("component", "__component__", journal_state, &note);
            }
            let json = serde_json::json!({
                "profile": outcome.profile,
                "note": note,
                "plan": outcome.plan,
                "evidence_summary": outcome.evidence_summary,
            });
            let human = render_component_outcome_human(&outcome) + "\n" + &note;
            Ok(as_output(format, human, json))
        }
        ComponentCommands::Qualify {
            id,
            to,
            reason,
            coverage,
            last_verified,
            known_issues,
            security_review,
            path,
        } => {
            let target = match to.as_str() {
                "experimental" => ComponentQuality::Experimental,
                "verified" => ComponentQuality::Verified,
                "certified" => ComponentQuality::Certified,
                "deprecated" => ComponentQuality::Deprecated,
                other => {
                    return Err(ForgeError::ComponentInvalid {
                        reason: format!(
                            "unknown quality '{other}'; expected one of experimental, \
                             verified, certified, deprecated"
                        ),
                    });
                }
            };
            let last_verified_ts = match last_verified.as_deref() {
                Some(value) => parse_rfc3339_for_qualify(value).ok_or_else(|| {
                    ForgeError::ComponentInvalid {
                        reason: format!(
                            "last_verified '{value}' is not a valid RFC 3339 timestamp"
                        ),
                    }
                })?,
                None => chrono::Utc::now(),
            };
            let evidence = ComponentQualifyEvidence {
                test_coverage: coverage.unwrap_or(0.95),
                last_verified: last_verified_ts,
                known_issues: known_issues.clone(),
                security_review: security_review.unwrap_or(target == ComponentQuality::Certified),
            };
            let request = ComponentQualifyRequest {
                component_id: id.clone(),
                target_quality: target,
                reason: reason.clone(),
            };
            // Qualify writes the receipt to `.forge/components/<id>/qualify.json`
            // inside the named project directory (default: current
            // working directory).
            let work_dir = match path.as_deref() {
                Some(p) => p.to_path_buf(),
                None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            };
            let outcome = record_qualification(&work_dir, &request, &evidence)?;
            let journal_state = if outcome.promoted { "done" } else { "blocked" };
            let detail = format!(
                "{}: {} -> {} ({}); {}",
                outcome.component_id,
                outcome.prior_quality.label(),
                outcome.target_quality.label(),
                if outcome.promoted {
                    "promoted"
                } else {
                    "refused"
                },
                outcome.note
            );
            if let Ok(registry) = open_registry(db_path) {
                let _ =
                    registry.record_operation("component", "__component__", journal_state, &detail);
            }
            let json = serde_json::json!({
                "component_id": outcome.component_id,
                "prior_quality": outcome.prior_quality.label(),
                "target_quality": outcome.target_quality.label(),
                "promoted": outcome.promoted,
                "files_written": outcome.files_written,
                "note": outcome.note,
            });
            Ok(as_output(format, render_qualify_human(&outcome), json))
        }
    }
}

fn parse_rfc3339_for_qualify(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn cmd_ui_pattern(
    db_path: &Path,
    command: &UiPatternCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        UiPatternCommands::List => {
            let catalog = ui_pattern_catalog();
            let entries: Vec<serde_json::Value> = catalog
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "version": c.version,
                        "intent": c.intent,
                        "quality": c.quality.label(),
                        "adapters": c.adapters.iter().map(|a| a.profile.clone()).collect::<Vec<_>>(),
                        "purpose": format!(
                            "{} pattern with {} state(s) and {} adapter(s)",
                            c.intent,
                            c.states.len(),
                            c.adapters.len()
                        ),
                    })
                })
                .collect();
            let mut human = format!(
                "{:<24} {:<10} {:<8} {:<12} {}",
                "Pattern", "Version", "Quality", "Intent", "Adapters"
            );
            for c in &catalog {
                let adapters = c
                    .adapters
                    .iter()
                    .map(|a| a.profile.clone())
                    .collect::<Vec<_>>()
                    .join(",");
                human.push_str(&format!(
                    "\n{:<24} {:<10} {:<8} {:<12} {}",
                    c.id,
                    c.version,
                    c.quality.label(),
                    c.intent,
                    adapters
                ));
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({"ui_patterns": entries}),
            ))
        }
        UiPatternCommands::Inspect { id } => {
            let descriptor = inspect_ui_pattern(id)?;
            let json = serde_json::json!({
                "id": descriptor.id,
                "version": descriptor.version,
                "intent": descriptor.intent,
                "documentation": descriptor.documentation,
                "quality": descriptor.quality.label(),
                "install_strategy": descriptor.install_strategy,
                "tests": descriptor.tests,
                "depends_on": descriptor.depends_on,
                "feature_deps": descriptor.feature_deps,
                "states": descriptor.states,
                "typography": descriptor.typography,
                "spacing": descriptor.spacing,
                "responsive": descriptor.responsive,
                "accessibility": descriptor.accessibility,
                "interaction": descriptor.interaction,
                "adapters": descriptor.adapters,
                "evidence": descriptor.evidence,
            });
            let mut human = vec![
                format!("ui pattern: {}@{}", descriptor.id, descriptor.version),
                format!("intent: {}", descriptor.intent),
                format!("quality: {}", descriptor.quality.label()),
                format!("documentation: {}", descriptor.documentation),
                format!("install: {}", descriptor.install_strategy),
                format!("tests: {}", descriptor.tests),
                "states:".to_string(),
            ];
            for state in &descriptor.states {
                human.push(format!("  - {}: {}", state.name, state.description));
            }
            human.push("adapters:".to_string());
            for adapter in &descriptor.adapters {
                human.push(format!(
                    "  - {} ({}): {} [tests: {}]",
                    adapter.profile, adapter.surface, adapter.artifact_path, adapter.tests
                ));
            }
            human.push("typography:".to_string());
            human.push(format!(
                "  family={} scale={} line_height={} weight={}",
                descriptor.typography.family,
                descriptor.typography.scale,
                descriptor.typography.line_height,
                descriptor.typography.weight
            ));
            human.push("accessibility:".to_string());
            human.push(format!(
                "  keyboard={} focus={} aria={} contrast={}",
                descriptor.accessibility.keyboard,
                descriptor.accessibility.focus,
                descriptor.accessibility.aria,
                descriptor.accessibility.contrast
            ));
            human.push("evidence:".to_string());
            human.push(format!(
                "  usage={} coverage={:.2} security_review={} last_verified={}",
                descriptor.evidence.usage_count,
                descriptor.evidence.test_coverage,
                descriptor.evidence.security_review,
                descriptor.evidence.last_verified.to_rfc3339()
            ));
            Ok(as_output(format, human.join("\n"), json))
        }
        UiPatternCommands::Resolve { profile, patterns } => {
            let request = UiPatternRequest {
                profile: profile.clone(),
                pattern_ids: patterns.clone(),
            };
            let outcome = resolve_ui_pattern_outcome(&request)?;
            let journal_state = if outcome.plan.steps.is_empty() {
                "rejected"
            } else {
                "done"
            };
            let summary = format!(
                "{} resolved={} rejected={}",
                request.profile,
                outcome.plan.steps.len(),
                outcome.plan.rejections.len()
            );
            let note = format!("{}; {}", summary, outcome.note);
            if let Ok(registry) = open_registry(db_path) {
                let _ =
                    registry.record_operation("ui_pattern", "__ui_pattern__", journal_state, &note);
            }
            let json = serde_json::json!({
                "profile": outcome.profile,
                "note": note,
                "plan": outcome.plan,
                "evidence_summary": outcome.evidence_summary,
            });
            let human = render_ui_pattern_outcome_human(&outcome) + "\n" + &note;
            Ok(as_output(format, human, json))
        }
        UiPatternCommands::Install {
            id,
            profile,
            reason,
            path,
        } => {
            let work_dir = match path.as_deref() {
                Some(p) => p.to_path_buf(),
                None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            };
            let request = UiPatternInstallRequest {
                pattern_id: id.clone(),
                profile: profile.clone(),
                reason: reason.clone(),
            };
            let outcome = install_pattern(&work_dir, &request)?;
            let journal_state = if outcome.installed { "done" } else { "blocked" };
            let detail = format!(
                "{}: {} -> {} ({}); {}",
                outcome.pattern_id,
                outcome.profile,
                if outcome.installed {
                    "installed"
                } else {
                    "refused"
                },
                outcome.installed,
                outcome.note
            );
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "ui_pattern",
                    "__ui_pattern__",
                    journal_state,
                    &detail,
                );
            }
            let json = serde_json::json!({
                "pattern_id": outcome.pattern_id,
                "profile": outcome.profile,
                "installed": outcome.installed,
                "files_written": outcome.files_written,
                "note": outcome.note,
            });
            Ok(as_output(format, render_install_human(&outcome), json))
        }
    }
}

fn parse_intent_action(value: &str) -> Result<IntentAction, ForgeError> {
    match value {
        "create_project" => Ok(IntentAction::CreateProject),
        "extend_project" => Ok(IntentAction::ExtendProject),
        other => Err(ForgeError::IntentInvalid {
            reason: format!(
                "unknown intent action '{other}'; accepted actions: create_project, \
                 extend_project"
            ),
        }),
    }
}

fn parse_intent_constraints(raw: &[String]) -> Result<Vec<IntentConstraint>, ForgeError> {
    let mut out = Vec::new();
    for entry in raw {
        let (key, value) = entry
            .split_once('=')
            .ok_or_else(|| ForgeError::IntentInvalid {
                reason: format!(
                    "constraint '{entry}' is not in 'key=value' form; the planner refuses a \
                 malformed constraint"
                ),
            })?;
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            return Err(ForgeError::IntentInvalid {
                reason: format!(
                    "constraint '{entry}' has an empty key or value; the planner refuses a \
                     malformed constraint"
                ),
            });
        }
        out.push(IntentConstraint {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(out)
}

fn build_intent(
    action: &str,
    profile: &str,
    required: &[String],
    forbidden: &[String],
    raw_constraints: &[String],
) -> Result<Intent, ForgeError> {
    Ok(Intent {
        action: parse_intent_action(action)?,
        profile: profile.to_string(),
        required_capabilities: required.to_vec(),
        forbidden_capabilities: forbidden.to_vec(),
        constraints: parse_intent_constraints(raw_constraints)?,
    })
}

fn cmd_intent(
    db_path: &Path,
    command: &IntentCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        IntentCommands::Validate {
            action,
            profile,
            required,
            forbidden,
            constraints,
        } => {
            let intent = build_intent(action, profile, required, forbidden, constraints)?;
            match validate_intent(&intent) {
                Ok(validated) => {
                    let outcome = IntentValidationOutcome {
                        validated: Some(validated.clone()),
                        note: validated.note.clone(),
                    };
                    let journal_state = "done";
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "planner",
                            "__planner__",
                            journal_state,
                            &format!("validate: {}", validated.note),
                        );
                    }
                    let json = serde_json::json!({
                        "contract": PLANNER_CONTRACT_VERSION,
                        "validated": validated,
                        "intent_hash": intent_hash(&validated.intent),
                    });
                    Ok(as_output(
                        format,
                        render_intent_validation_human(&outcome),
                        json,
                    ))
                }
                Err(err) => {
                    let outcome = IntentValidationOutcome {
                        validated: None,
                        note: err.to_string(),
                    };
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "planner",
                            "__planner__",
                            "rejected",
                            &outcome.note,
                        );
                    }
                    Err(err)
                }
            }
        }
        IntentCommands::Resolve {
            action,
            profile,
            required,
            forbidden,
            constraints,
            path,
        } => {
            let intent = build_intent(action, profile, required, forbidden, constraints)?;
            let validated = validate_intent(&intent)?;
            let work_dir = path
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let plan = resolve_planner_plan(&validated, None)?;
            let receipt_path = write_plan_receipt(&work_dir, &plan)?;
            let outcome = IntentResolveOutcome {
                note: plan.note.clone(),
                plan: plan.clone(),
                receipt_path: receipt_path.display().to_string(),
            };
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "planner",
                    &plan.plan_id,
                    "done",
                    &format!("resolve: {}", plan.note),
                );
            }
            let json = serde_json::json!({
                "contract": PLANNER_CONTRACT_VERSION,
                "plan": plan,
                "receipt_path": outcome.receipt_path,
            });
            Ok(as_output(format, render_plan_human(&outcome.plan), json))
        }
        IntentCommands::Apply {
            plan_id,
            confirm,
            path,
        } => {
            let work_dir = path
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let outcome = apply_planner_plan(&work_dir, plan_id, *confirm, Some(db_path))?;
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "planner",
                    &outcome.plan_id,
                    if outcome.stale { "rejected" } else { "done" },
                    &outcome.note,
                );
            }
            let json = serde_json::json!({
                "contract": PLANNER_CONTRACT_VERSION,
                "outcome": outcome.clone(),
            });
            Ok(as_output(format, render_apply_human(&outcome), json))
        }
        IntentCommands::List { path } => {
            let mut entries: Vec<serde_json::Value> = Vec::new();
            let dir = plans_dir(path);
            if dir.exists() {
                if let Ok(read) = std::fs::read_dir(&dir) {
                    for entry in read.flatten() {
                        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            let plan_id = entry.file_name().to_string_lossy().to_string();
                            entries.push(serde_json::json!({
                                "plan_id": plan_id,
                                "receipt": entry.path().join("plan.json").display().to_string(),
                            }));
                        }
                    }
                }
            }
            let mut human = format!("planner plans: {}", entries.len());
            for e in &entries {
                if let Some(id) = e.get("plan_id").and_then(|v| v.as_str()) {
                    human.push_str(&format!("\n  - {id}"));
                }
            }
            let json = serde_json::json!({"plans": entries});
            Ok(as_output(format, human, json))
        }
    }
}
