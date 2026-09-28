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
use forge::analytics::{
    aggregate_project_metrics, inspect_external_planes, load_config as load_analytics_config,
    metrics_summary_path, render_metrics_human,
    render_report_human as render_analytics_report_human, save_metrics_summary, AnalyticsConfig,
    AnalyticsInspectOptions, AnalyticsProvider, DoctorSummary, MetricsAggregateOptions,
    MetricsSummary, ANALYTICS_CONTRACT_VERSION, ANALYTICS_SYNTHETIC_PROJECT,
};
use forge::api::{
    serve as api_serve, ApiConfig, ShutdownSignal, API_CONTRACT_VERSION, API_SYNTHETIC_PROJECT,
};
use forge::checker::{self, PROTOCOL_MAX_ALERTS};
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
use forge::fleet::{
    self, health_json, inspect_entry, observe, render_entry_human, render_list_block_human,
    render_report_human as render_fleet_report_human, render_status_human, resolve_registry_path,
    DEFAULT_MAX_AGE_SECONDS, FLEET_CONTRACT_VERSION,
};
use forge::gate::{
    self, evidence_freshness, evidence_summary, load_latest_evidence, render_evidence_human,
    GateAggregate, GateConfig, GateFreshness, GateOutcome, GATE_CONTRACT_VERSION,
};
use forge::generate::{generate, normalize_explicit, parse_interactive, verify_native};
use forge::gitops::{commit_paths, push_ref, run_test, CommitOutcome, PushOutcome, TestOutcome};
use forge::governance::{
    check_project as check_governance_project, inspect as inspect_governance_project,
    list_providers as list_governance_providers, resolve_known_adapter,
    save_provider_selection_with_root, GovernanceObservation, ProviderStatus,
    GOVERNANCE_CONTRACT_VERSION, LOCAL_PROVIDER_ID, WORKSPACE_ROOT_ENV,
};
use forge::identity::{
    build_challenge, delete_challenge_file as delete_identity_challenge,
    list_sessions as list_identity_sessions, load_challenge, load_session, mint_session,
    redact_identity_evidence, render_challenge_human,
    render_outcome_human as render_identity_outcome_human,
    render_session_human as render_identity_session_human, save_challenge, save_session,
    terminate_session, validate_callback, validate_claims, validate_session, AuthCallback,
    IdentityConfig, IdentityOutcome, ProviderClaims, IDENTITY_CONTRACT_VERSION,
};
use forge::import::{adopt_import, inspect_import, render_proposal_human};
use forge::planner::{
    apply_plan as apply_planner_plan, intent_hash, plans_dir, render_apply_human,
    render_intent_validation_human, render_plan_human, resolve_plan as resolve_planner_plan,
    validate_intent, write_plan_receipt, Intent, IntentAction, IntentConstraint,
    IntentResolveOutcome, IntentValidationOutcome, PLANNER_CONTRACT_VERSION,
};
use forge::policy::{run_driftwatch, DriftWatchConfig};
use forge::portal::{
    build_dashboard_with_fleet, build_section_view_with_fleet, parse_section,
    render_dashboard_human, render_section_human as render_portal_section_human, FleetProjection,
    PortalDashboard, PORTAL_CONTRACT_VERSION, PORTAL_SYNTHETIC_PROJECT,
};
use forge::procedure::{
    inspect_procedure, procedure_catalog, render_inspect_human as render_procedure_inspect_human,
    render_list_human as render_procedure_list_human, validate_procedure, ProcedureSpec,
    PROCEDURE_CONTRACT_VERSION, PROCEDURE_SYNTHETIC_PROJECT,
};
use forge::profile::{inspect_profile, list_profiles, preflight_profile, resolve_profile};
use forge::provider::{
    inspect as inspect_provider, matrix as provider_matrix,
    parse_provider as parse_evidence_provider,
    render_descriptor_human as render_provider_descriptor_human,
    render_matrix_human as render_provider_matrix_human,
    render_row_human as render_provider_row_human, run_controlled as run_provider_controlled,
    RunOptions as ProviderRunOptions, PROVIDER_CONTRACT_VERSION, PROVIDER_SYNTHETIC_PROJECT,
};
use forge::publish::{
    jenkins::JenkinsAdapter,
    providers::{
        invoke_provider, load_config as load_publish_provider_config, select_provider,
        ProviderOperation, PublishProviderRequest,
    },
    remote_compose::RemoteComposeAdapter,
    render_report_human as render_publish_report_human, request_from as build_publish_request,
    run_publish, PublishAction, SubprocessTransport, PUBLISH_CONTRACT_VERSION,
};
use forge::readiness::{
    artifact_evidence, evaluate_gate, render_artifact_human, render_gate_human,
    render_matrix_human, run_matrix, READINESS_CONTRACT_VERSION,
};
use forge::registry::{default_registry_path, ProjectRecord, PublishPhaseEvidence, Registry};
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
use std::collections::BTreeSet;
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

    /// Explicit governance vocabulary file (overrides
    /// `$FORGE_GOVERNANCE_VOCABULARY` and the vendored copy).
    #[arg(long, global = true)]
    vocabulary: Option<PathBuf>,

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
        /// Omit the Workspace Governance `.project.json` declaration
        /// (output is then byte-identical to pre-metadata releases).
        #[arg(long)]
        no_workspace_metadata: bool,
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
    /// Emit a Driftwatchdog-compatible external-checker document on stdout (machine-pure, read-only).
    Check {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(value_name = "TARGET", default_value = ".")]
        target: String,
        /// Run the DriftWatch policy plane first and re-emit its findings as alerts
        /// (off by default to avoid a checker/policy feedback loop).
        #[arg(long)]
        include_policy: bool,
        /// Maximum number of alerts in the emitted document (1..=10000; default 64).
        #[arg(long, default_value_t = 64)]
        max_alerts: u32,
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
    /// Publish a registered project to the Jenkins/Mac infrastructure with subdomain routing.
    ///
    /// With no flags `forge publish` discovers the project from the current
    /// directory (`.project.json:id` > `forge.yaml:project.id` > directory
    /// name) and publishes it through the selected provider.
    Publish {
        #[command(subcommand)]
        command: Option<PublishCommands>,
        /// Publish one project through an enabled external provider.
        #[arg(long, conflicts_with = "folder")]
        project: Option<String>,
        /// Publish the project represented by this source folder.
        #[arg(long, conflicts_with = "project")]
        folder: Option<PathBuf>,
        /// Directory to discover the project from for bare `forge publish` (default: current directory).
        #[arg(long, value_name = "PATH")]
        cwd: Option<PathBuf>,
        /// External provider id, for example `openpanel` or `jenkins`.
        #[arg(long)]
        provider: Option<String>,
        /// Source revision sent to the provider. Defaults to HEAD when available.
        #[arg(long)]
        revision: Option<String>,
        /// Validate and print the request without invoking the provider.
        #[arg(long)]
        dry_run: bool,
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
    /// Discover, inspect and validate portable AI procedures over stable Core operations.
    Procedure {
        #[command(subcommand)]
        command: ProcedureCommands,
    },
    /// Validate, challenge, complete and terminate per-project OIDC admin sessions.
    Identity {
        #[command(subcommand)]
        command: IdentityCommands,
    },
    /// Inspect the existing content / analytics providers configured for a project and aggregate timestamped project metrics.
    Analytics {
        #[command(subcommand)]
        command: AnalyticsCommands,
    },
    /// Serve the optional HTTP transport over Core on a loopback listener.
    Api {
        #[command(subcommand)]
        command: ApiCommands,
    },
    /// Render the optional control-plane portal dashboard and per-section views.
    Portal {
        #[command(subcommand)]
        command: PortalCommands,
    },
    /// Generate the supported profile fixtures natively and evaluate release readiness.
    Readiness {
        #[command(subcommand)]
        command: ReadinessCommands,
    },
    /// Record opt-in controlled evidence for the external provider boundaries.
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },
    /// Inspect and select standalone or optional external governance providers.
    Governance {
        #[command(subcommand)]
        command: GovernanceCommands,
    },
    /// Observe the portfolio declared by an external workspace registry, read-only.
    Fleet {
        #[command(subcommand)]
        command: FleetCommands,
    },
    /// Execute the project's declared shared gate runtime and journal revision-bound evidence.
    Gate {
        /// `status` (read persisted evidence without re-running) or a
        /// project id / filesystem path (default: current directory).
        #[arg(value_name = "status | TARGET")]
        args: Vec<String>,
        /// Rehearse through the runtime's side-effect-free plan preview;
        /// never executes checks, persists evidence or journals a row.
        #[arg(long)]
        dry_run: bool,
        /// Bounded gate execution timeout in seconds (1..=86400; default 600).
        #[arg(long, value_name = "SECS")]
        timeout_secs: Option<u64>,
    },
    /// Vendor, inspect and project platform contracts into envelopes.
    Contract {
        #[command(subcommand)]
        command: ContractCommands,
    },
    /// Load, validate and report a portable project inventory
    /// (`forge-project-inventory/0.1.0`). Read-only; never
    /// invokes a provider or mutates the registry.
    Inventory {
        #[command(subcommand)]
        command: InventoryCommands,
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
enum PublishCommands {
    /// Inspect or switch standalone publish providers.
    Provider {
        #[command(subcommand)]
        command: PublishProviderCommands,
    },
    /// Sync a project source tree to the Mac via SSH/rsync (Stage 1).
    Sync {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        project: String,
        /// Read-only plan: report the would-sync command without contacting the Mac.
        #[arg(long)]
        dry_run: bool,
    },
    /// Provision shared PostgreSQL database on the Mac (Stage 2).
    Db {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        project: String,
        /// Read-only plan: report the would-db command without contacting the Mac.
        #[arg(long)]
        dry_run: bool,
    },
    /// Publish every project in an explicit inventory that has a Compose contract.
    ///
    /// When `--inventory <path>` is supplied Forge consumes the
    /// `forge-project-inventory/0.1.0` contract (local file or
    /// external adapter) and reports every entry as
    /// `compose_ready`, `compose_missing`, `invalid`, or
    /// `source_unavailable`. Only `compose_ready` entries invoke a
    /// provider; the others are reported, never silently dropped.
    /// The legacy `--fleet-registry` flag is retained as a
    /// compatibility adapter for the seven-project handoff.
    Fleet {
        /// Explicit inventory source (local JSON file or executable
        /// adapter path). Takes precedence over `--fleet-registry`.
        #[arg(long = "inventory")]
        inventory: Option<std::path::PathBuf>,
        /// Path to the legacy workspace-governance `projects.json`
        /// registry. Used only when `--inventory` is absent.
        #[arg(long = "fleet-registry")]
        fleet_registry: Option<std::path::PathBuf>,
        /// Workspace root (where project paths in the legacy registry are resolved against).
        #[arg(long)]
        workspace_root: Option<std::path::PathBuf>,
        /// Read-only plan: report what would happen without contacting the Mac.
        #[arg(long)]
        dry_run: bool,
        /// Only publish projects whose lifecycle matches (legacy
        /// `--fleet-registry` path; default `active`).
        #[arg(long, default_value = "active")]
        lifecycle: String,
        /// Stop at the first failure instead of continuing the rest of the fleet.
        #[arg(long)]
        fail_fast: bool,
        /// Use one external provider for every eligible project.
        #[arg(long)]
        provider: Option<String>,
    },
    /// Allocate ports and prepare the project environment on the Mac (Stage 2).
    Prepare {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        project: String,
        /// Read-only plan: report the would-prepare command without contacting the Mac.
        #[arg(long)]
        dry_run: bool,
    },
    /// Trigger the Jenkins deploy job on the Mac (Stage 3).
    Deploy {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        project: String,
        /// Read-only plan: report the would-deploy command without contacting Jenkins.
        #[arg(long)]
        dry_run: bool,
    },
    /// Run sync, prepare and deploy in order. Stops at the first failure.
    All {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        project: String,
        /// Read-only plan: report every stage without contacting the Mac or Jenkins.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
enum PublishProviderCommands {
    /// List configured providers and their enabled state.
    List {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Inspect one configured provider without invoking it.
    Inspect {
        id: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Enable an already configured provider.
    Enable {
        id: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Disable an already configured provider.
    Disable {
        id: String,
        #[arg(long)]
        config: Option<PathBuf>,
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
enum ProcedureCommands {
    /// List the named AI procedures in the catalog.
    List,
    /// Inspect one named AI procedure (prerequisites, ordered steps, verification).
    Inspect {
        /// Procedure id (e.g. `create-project`).
        id: String,
    },
    /// Validate a procedure spec from a JSON file (typed `procedure-invalid` / `procedure-bypass-refused` on refusal).
    Validate {
        /// Path to a JSON file containing a [`ProcedureSpec`].
        #[arg(long)]
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum IdentityCommands {
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

#[derive(Debug, Subcommand)]
enum AnalyticsCommands {
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
enum ApiCommands {
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
enum PortalCommands {
    /// Render the top-level control-plane dashboard for one project or the whole registry.
    Dashboard {
        /// Registered project id or filesystem path. Defaults to the entire registered registry.
        #[arg(default_value = "")]
        target: String,
        /// Force fleet-wide rendering even when a target is supplied.
        #[arg(long)]
        all: bool,
    },
    /// Render a single portal section (projects, features, components, policies, specs, agents, deployments, repositories, documentation, analytics, servers, settings).
    View {
        /// Section id (one of the twelve supported sections; the CLI rejects unknown ids with `error[portal-invalid]`).
        section: String,
        /// Registered project id or filesystem path. Defaults to the entire registered registry.
        #[arg(default_value = "")]
        target: String,
    },
}

#[derive(Debug, Subcommand)]
enum ReadinessCommands {
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
enum ProviderCommands {
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
enum GovernanceCommands {
    /// List the local provider and the configured optional provider.
    List {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Run the selected provider and record a bounded observation.
    Status {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Alias for status that returns the full normalized observation.
    Inspect {
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Select a provider without changing forge.yaml or registry identity.
    Use {
        /// Provider id, or `local` for the built-in standalone provider.
        provider: String,
        /// Project directory (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Adapter executable for an external provider.
        #[arg(long)]
        adapter: Option<String>,
        /// Workspace root holding a known provider's packaged adapter
        /// (default: $FORGE_WORKSPACE_ROOT). Never searched implicitly.
        #[arg(long, value_name = "PATH")]
        workspace_root: Option<PathBuf>,
        /// Disable the selected external provider without removing its config.
        #[arg(long)]
        disable: bool,
        /// Adapter timeout in milliseconds.
        #[arg(long, default_value_t = 10_000)]
        timeout_ms: u64,
    },
}

#[derive(Debug, Subcommand)]
enum FleetCommands {
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
        #[arg(long, value_name = "SECONDS", default_value_t = forge::fleet::online::DEFAULT_HTTP_TIMEOUT_SECS)]
        timeout_secs: u64,
        /// Read-only plan: render the probe plan without contacting the target or any origin.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ContractCommands {
    /// List every versioned surface and its platform mapping.
    List,
    /// Inspect one platform family and its schema.
    Inspect {
        /// Platform family (e.g. `platform.gate-result`).
        family: String,
    },
    /// Project a Core record into a platform envelope (read-only).
    Emit {
        /// Platform family (e.g. `platform.gate-result`).
        family: String,
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Validate a contract envelope against the vendored schemas.
    Validate {
        /// File to validate, or `-` for stdin.
        file: String,
    },
}

#[derive(Debug, Subcommand)]
enum InventoryCommands {
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
enum AgentCommands {
    /// Start a new managed agent session for the named project.
    Start {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Session id (kebab-case).
        #[arg(long)]
        session: String,
        /// Provider id (opencode, codex, ariadex).
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
        /// Execution supervisor. `sisyphusfy` hands the bound spec's
        /// task file to the sibling iteration loop and journals the
        /// supervisor's independent verification verdict. Omit to run
        /// through the session's bundled adapter path.
        #[arg(long)]
        provider: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let db_path = cli.registry.clone().unwrap_or_else(default_registry_path);
    // Forward the explicit vocabulary flag to every loader through the
    // env so doctor, profile parsing and any surface that consults the
    // vocabulary see exactly one source of truth for the test.
    if let Some(ref vocab) = cli.vocabulary {
        // SAFETY: this is single-threaded CLI startup; the env var is
        // never mutated again during command dispatch.
        unsafe {
            std::env::set_var(forge::vocabulary::VOCABULARY_ENV, vocab.as_os_str());
        }
    }

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

    // The gate run mirrors the sibling's blocking semantics in its exit
    // code (0 only for a passed aggregate) while still printing the
    // evidence document, so it owns its exit code like the fleet does.
    if let Commands::Gate {
        args,
        dry_run,
        timeout_secs,
    } = &cli.command
    {
        return cmd_gate(&db_path, args, *dry_run, *timeout_secs, cli.format);
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
            no_workspace_metadata,
        } => cmd_new(
            &db_path,
            path,
            profile.as_deref(),
            id.as_deref(),
            name.as_deref(),
            features,
            *verify_native,
            !no_workspace_metadata,
            cli.format,
        ),
        Commands::Doctor { path, target } => {
            cmd_doctor(&db_path, path, target.as_deref(), cli.format)
        }
        Commands::Check {
            target,
            include_policy,
            max_alerts,
        } => cmd_check(&db_path, target, *include_policy, *max_alerts),
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
        Commands::Publish {
            command,
            project,
            folder,
            cwd,
            provider,
            revision,
            dry_run,
        } => cmd_publish(
            &db_path,
            command.as_ref(),
            project.as_deref(),
            folder.as_deref(),
            cwd.as_deref(),
            provider.as_deref(),
            revision.as_deref(),
            *dry_run,
            cli.format,
        ),
        Commands::Component { command } => cmd_component(&db_path, command, cli.format),
        Commands::UiPattern { command } => cmd_ui_pattern(&db_path, command, cli.format),
        Commands::Intent { command } => cmd_intent(&db_path, command, cli.format),
        Commands::Procedure { command } => cmd_procedure(&db_path, command, cli.format),
        Commands::Identity { command } => cmd_identity(&db_path, command, cli.format),
        Commands::Analytics { command } => cmd_analytics(&db_path, command, cli.format),
        Commands::Api { command } => cmd_api(&db_path, command, cli.format),
        Commands::Portal { command } => cmd_portal(&db_path, command, cli.format),
        Commands::Readiness { command } => cmd_readiness(command, cli.format),
        Commands::Provider { command } => cmd_provider(&db_path, command, cli.format),
        Commands::Governance { command } => cmd_governance(command, cli.format),
        Commands::Fleet { command } => cmd_fleet(&db_path, command, cli.format),
        Commands::Contract { command } => cmd_contract(&db_path, command, cli.format),
        Commands::Inventory { command } => cmd_inventory(command, cli.format),
        Commands::Gate { .. } => {
            // Handled by the early `if let` above (the gate run owns its
            // exit code to mirror the sibling's blocking semantics); this
            // arm exists only to keep the match exhaustive.
            return ExitCode::from(2);
        }
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
    // Optional read-only fleet block: present only when a workspace
    // registry is configured. An unconfigured surface stays byte-
    // identical to the local list, and a configured-but-unobservable
    // registry renders the typed failure rather than masking it.
    let fleet = portal_fleet_projection(&registry);
    let human_base = if projects.is_empty() {
        "No projects registered.".to_string()
    } else {
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
        human
    };
    let json_base = if projects.is_empty() {
        serde_json::json!({"projects": []})
    } else {
        serde_json::json!({"projects": projects})
    };
    let (human, json) = match &fleet {
        None => (human_base, json_base),
        Some(Ok(report)) => {
            let mut json = json_base;
            json["fleet"] = serde_json::to_value(report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            (human_base + &render_list_block_human(report), json)
        }
        Some(Err(err)) => {
            let line = format!("\nfleet: unavailable — error[{}]: {}", err.code(), err);
            let json = serde_json::json!({
                "projects": projects,
                "fleet_error": {"code": err.code(), "message": err.to_string()},
            });
            (human_base + &line, json)
        }
    };
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
    workspace_metadata: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let mut request = if profile.is_some() {
        normalize_explicit(profile, id, name, features, path)?
    } else {
        let stdin = std::io::stdin();
        let mut reader = std::io::BufReader::new(stdin.lock());
        let mut writer = std::io::stderr();
        parse_interactive(&mut reader, &mut writer, path, profile, id, name, features)?
    };
    request.workspace_metadata = workspace_metadata;
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
    let mut human = format!(
        "created {} ({}) from {}@{}\nfiles: {}\n{}",
        generated.record.id,
        generated.record.path,
        request.profile,
        forge::generate::GENERATOR_VERSION,
        generated.files.join(", "),
        generated.native_note
    );
    for note in &generated.notes {
        human.push_str(&format!("\n{note}"));
    }
    let mut json = serde_json::json!({
        "created": generated.record,
        "profile": request.profile,
        "generator": forge::generate::GENERATOR_VERSION,
        "files": generated.files,
        "native_verified": generated.native_verified,
        "native_note": generated.native_note,
    });
    if !generated.notes.is_empty() {
        json["notes"] = serde_json::json!(generated.notes);
    }
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

/// Render the external-checker document for one registered project.
///
/// The command exists for machines (Driftwatchdog checker protocol):
/// stdout carries exactly one compact JSON document, diagnostics go to
/// stderr, findings never change the exit code, and nothing about the
/// run is journaled or persisted. Both output formats print the
/// identical document, so `--format json` cannot alter checker stdout.
fn cmd_check(
    db_path: &Path,
    target: &str,
    include_policy: bool,
    max_alerts: u32,
) -> Result<Output, ForgeError> {
    if max_alerts == 0 || max_alerts as usize > PROTOCOL_MAX_ALERTS {
        return Err(ForgeError::CheckInvalid {
            reason: format!(
                "--max-alerts {max_alerts} is outside the bounded range 1..={PROTOCOL_MAX_ALERTS}"
            ),
        });
    }
    // Resolve through the registry so an unregistered target fails with
    // the typed unknown-project error before anything is written to
    // stdout; a partial document must never be parseable by the checker.
    let registry = open_registry(db_path)?;
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: record.path.clone(),
        });
    }
    let observation = Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at),
    });
    // The DriftWatch policy plane stays off by default: a checker run
    // must not drive DriftWatch, which may drive Forge back through its
    // checker registration. Only an explicit --include-policy invokes
    // the adapter; without it the policy plane contributes no findings.
    let policy_outcome = if include_policy {
        Some(run_driftwatch(&dir, &DriftWatchConfig::from_env()))
    } else {
        None
    };
    let report = run_doctor(&dir, None, observation.as_ref(), policy_outcome.as_ref())?;
    // The governance plane is evaluated read-only (no observation is
    // persisted); a plane that cannot be evaluated projects into a
    // warning naming the gap instead of a hard failure.
    let governance = inspect_governance_project(&dir);
    let document = checker::build_document(
        &dir,
        &report,
        &governance,
        &checker::now_rfc3339(),
        max_alerts as usize,
    );
    Ok(Output::Human(checker::render_document(&document)?))
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
        "ariadex" => Ok(AgentProvider::Ariadex),
        "sisyphusfy" => Err(ForgeError::AgentUnavailable {
            reason: "sisyphusfy is a spec-execution supervisor, not a session provider; use `forge agent run-spec --provider sisyphusfy`".to_string(),
        }),
        other => Err(ForgeError::AgentUnavailable {
            reason: format!(
                "unknown agent provider `{other}`; expected one of: opencode, codex, ariadex"
            ),
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
            let backing_note = outcome
                .session
                .backing
                .as_ref()
                .map(|b| {
                    format!(
                        " backing `{}`/`{}`",
                        b.runtime,
                        if b.handle.is_empty() {
                            "(none)"
                        } else {
                            &b.handle
                        }
                    )
                })
                .unwrap_or_default();
            let detail = format!(
                "agent `{}` session `{}` provider `{}` spec `{}` -> `{}`{backing_note}",
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
            let mut json =
                serde_json::to_value(&session_rec).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?;
            let mut human = render_agent_session_human(&session_rec);
            if let Some(live) = forge::agent::live_runtime_status(&session_rec) {
                json["live"] = serde_json::to_value(&live).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?;
                human.push_str(&format!(
                    "\nlive: {} (daemon={} mode={} handle={} note={})",
                    live.state,
                    if live.daemon.is_empty() {
                        "(none)"
                    } else {
                        &live.daemon
                    },
                    if live.mode.is_empty() {
                        "(none)"
                    } else {
                        &live.mode
                    },
                    if live.handle.is_empty() {
                        "(none)"
                    } else {
                        &live.handle
                    },
                    live.note
                ));
            }
            Ok(as_output(format, human, json))
        }
        AgentCommands::List { target } => {
            let (path, _) = resolve_agent_project(target)?;
            let entries = list_sessions(&path)?;
            agent_list_output(&entries, format)
        }
        AgentCommands::RunSpec {
            target,
            session,
            provider,
        } => {
            let (path, project_id) = resolve_agent_project(target)?;
            let supervisor = match provider.as_deref() {
                None => None,
                Some(value) if value == forge::agent::SISYPHUSFY_SUPERVISOR => Some(value),
                Some(other) => {
                    return Err(ForgeError::AgentUnavailable {
                        reason: format!(
                            "unknown run-spec provider `{other}`; spec execution supports the supervisor: {}",
                            forge::agent::SISYPHUSFY_SUPERVISOR
                        ),
                    })
                }
            };
            let now = chrono::Utc::now();
            let outcome = run_session_spec(&path, session, supervisor, now)?;
            let files = forge::agent::write_session(&path, &outcome.session)?;
            let registry = open_registry(db_path)?;
            let verdict = outcome
                .verdict
                .clone()
                .unwrap_or_else(|| "done".to_string());
            let detail = format!(
                "agent `{}` run_spec session `{}` -> `{}` (verdict `{verdict}`)",
                project_id,
                session,
                outcome.state.label()
            );
            let _ = registry.record_operation("agent", &project_id, &verdict, &detail);
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
    let mut json = serde_json::json!({
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
    if let Some(verdict) = &outcome.verdict {
        json["transition"]["verdict"] = serde_json::Value::String(verdict.clone());
    }
    let mut human = format!(
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
    if let Some(verdict) = &outcome.verdict {
        human.push_str(&format!("\nverdict: {verdict}"));
    }
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

fn render_agent_session_human(session: &forge::agent::AgentSession) -> String {
    let mut lines = vec![
        format!("session: {}", session.session_id),
        format!("project: {}", session.project_id),
        format!("provider: {}", session.provider.label()),
    ];
    if let Some(backing) = &session.backing {
        lines.push(format!(
            "backing: runtime={} handle={} adapter_version={}",
            backing.runtime,
            if backing.handle.is_empty() {
                "(none)"
            } else {
                &backing.handle
            },
            backing.adapter_version
        ));
    }
    lines.extend(vec![
        format!("spec: {}", session.spec_id),
        format!("state: {}", session.state.label()),
        format!("started_at: {}", session.started_at.to_rfc3339()),
        format!(
            "last_transition_at: {}",
            session.last_transition_at.to_rfc3339()
        ),
    ]);
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
        DeployCommands::Status {
            project,
            queue,
            limit,
            watch,
            interval_secs,
            deadline_secs,
        } => cmd_deploy_status(
            db_path,
            project.as_deref(),
            queue.as_deref(),
            *limit,
            *watch,
            *interval_secs,
            *deadline_secs,
            format,
        ),
    }
}

fn cmd_deploy_status(
    db_path: &Path,
    project: Option<&str>,
    queue: Option<&str>,
    limit: usize,
    watch: bool,
    interval_secs: u64,
    deadline_secs: u64,
    format: Format,
) -> Result<Output, ForgeError> {
    if watch && queue.is_none() {
        return Err(ForgeError::PublishInvalid {
            reason: "deploy status --watch requires --queue <id>".to_string(),
        });
    }
    if interval_secs == 0 || interval_secs > 60 {
        return Err(ForgeError::PublishInvalid {
            reason: "deploy status --interval-secs must be in 1..=60".to_string(),
        });
    }
    if deadline_secs == 0 || deadline_secs > 86400 {
        return Err(ForgeError::PublishInvalid {
            reason: "deploy status --deadline-secs must be in 1..=86400".to_string(),
        });
    }
    let registry = open_registry(db_path)?;
    let limit = limit.clamp(1, 500);
    let (entries, read_only_label) = match (project, queue) {
        (Some(_), Some(_)) => {
            return Err(ForgeError::PublishInvalid {
                reason: "deploy status accepts --project or --queue, not both".to_string(),
            });
        }
        (Some(project_id), None) => {
            let rows = registry.operations_for_project(project_id, limit)?;
            (filter_publish_deploy(rows), format!("project={project_id}"))
        }
        (None, Some(queue_id)) => {
            forge::publish::providers::validate_queue_id(queue_id).map_err(|_| {
                ForgeError::PublishInvalid {
                    reason: format!("queue id `{queue_id}` is not 1..=128 ASCII"),
                }
            })?;
            let rows = registry.operations_for_queue(queue_id, limit)?;
            (filter_publish_deploy(rows), format!("queue={queue_id}"))
        }
        (None, None) => {
            let rows = registry.recent_operations(limit)?;
            (filter_publish_deploy(rows), "scope=recent".to_string())
        }
    };
    let latest = entries.first();
    let aggregate_state = latest.map(|entry| entry.state.as_str()).unwrap_or("empty");
    let value = serde_json::json!({
        "contract": "forge-deploy-status/0.2.0",
        "project": project,
        "queue": queue,
        "scope": read_only_label,
        "state": aggregate_state,
        "entries": entries,
        "read_only": true,
    });
    let human = render_deploy_status_human(&entries, project, queue, &read_only_label);
    if watch {
        return cmd_deploy_status_watch(
            db_path,
            queue.unwrap(),
            interval_secs,
            deadline_secs,
            format,
        );
    }
    Ok(as_output(format, human, value))
}

fn filter_publish_deploy(
    entries: Vec<forge::registry::OperationEntry>,
) -> Vec<forge::registry::OperationEntry> {
    // Includes every variant of the publish surface — the manual
    // `forge publish` path (`publish`), the legacy stage publish
    // (`deploy`), and the GitHub-push reserve id (`publish.github`)
    // — so the additive `revision` / `build_status` / `run_status` /
    // `container_identity` evidence stays visible through one
    // `forge deploy status` query regardless of which path produced
    // the row.
    entries
        .into_iter()
        .filter(|entry| {
            entry.kind == "publish" || entry.kind == "deploy" || entry.kind == "publish.github"
        })
        .collect()
}

fn render_deploy_status_human(
    entries: &[forge::registry::OperationEntry],
    project: Option<&str>,
    queue: Option<&str>,
    scope: &str,
) -> String {
    if entries.is_empty() {
        return match (project, queue) {
            (Some(project_id), _) => {
                format!("deploy status: no publish/deploy history for {project_id}")
            }
            (None, Some(queue_id)) => {
                format!("deploy status: no history for queue `{queue_id}`")
            }
            (None, None) => "deploy status: no publish/deploy history".to_string(),
        };
    }
    let mut lines = vec![format!("deploy status: {scope}")];
    for entry in entries {
        let queue_label = entry
            .queue_id
            .as_deref()
            .map(|q| format!(" queue={q}"))
            .unwrap_or_default();
        let revision_label = entry
            .revision
            .as_deref()
            .map(|r| format!(" revision={r}"))
            .unwrap_or_default();
        let build_label = entry
            .build_status
            .as_deref()
            .map(|b| format!(" build={b}"))
            .unwrap_or_default();
        let run_label = entry
            .run_status
            .as_deref()
            .map(|r| format!(" run={r}"))
            .unwrap_or_default();
        let identity_label = entry
            .container_identity
            .as_deref()
            .map(|c| format!(" container={c}"))
            .unwrap_or_default();
        lines.push(format!(
            "  #{} {} {} {}{}{}{}{}{}",
            entry.op_id,
            entry.project_id,
            entry.kind,
            entry.state,
            queue_label,
            revision_label,
            build_label,
            run_label,
            identity_label,
        ));
    }
    lines.join("\n")
}

fn cmd_deploy_status_watch(
    db_path: &Path,
    queue_id: &str,
    interval_secs: u64,
    deadline_secs: u64,
    format: Format,
) -> Result<Output, ForgeError> {
    let started = std::time::Instant::now();
    let deadline = std::time::Duration::from_secs(deadline_secs);
    let interval = std::time::Duration::from_secs(interval_secs);
    loop {
        let registry = open_registry(db_path)?;
        let entries = registry.operations_for_queue(queue_id, 1024)?;
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|entry| entry.kind == "publish" || entry.kind == "deploy")
            .collect();
        let still_active = entries
            .iter()
            .any(|entry| entry.state == "pending" || entry.state == "running");
        let rendered_human = render_deploy_status_human(
            &entries,
            None,
            Some(queue_id),
            &format!("queue={queue_id}"),
        );
        let rendered_json = serde_json::json!({
            "contract": "forge-deploy-status/0.2.0",
            "queue": queue_id,
            "scope": format!("queue={queue_id}"),
            "state": if still_active { "active" } else { "terminal" },
            "entries": entries,
            "read_only": true,
        });
        render_output(as_output(format, rendered_human, rendered_json));
        if !still_active {
            let all_succeeded = entries
                .iter()
                .all(|entry| entry.state == "done" || entry.state == "succeeded");
            return Ok(if all_succeeded {
                as_output(format, String::new(), serde_json::json!({"watch":"done"}))
            } else {
                return Err(ForgeError::PublishDeployFailed {
                    reason: format!(
                        "watched queue `{queue_id}` ended with non-success terminal states"
                    ),
                });
            });
        }
        if started.elapsed() >= deadline {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "watch deadline {deadline_secs}s reached while queue `{queue_id}` was still active"
                ),
            });
        }
        std::thread::sleep(interval);
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

fn discover_cwd_publish_target(cwd: Option<&Path>) -> Result<(PathBuf, String), ForgeError> {
    let raw = if let Some(path) = cwd {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_err(|error| ForgeError::PublishInvalid {
            reason: format!("cannot determine current directory: {error}"),
        })?
    };
    let dir = std::fs::canonicalize(&raw).map_err(|error| ForgeError::PublishInvalid {
        reason: format!("cannot resolve publish cwd {}: {error}", raw.display()),
    })?;
    let project_json = dir.join(".project.json");
    if project_json.is_file() {
        let content =
            std::fs::read_to_string(&project_json).map_err(|error| ForgeError::PublishInvalid {
                reason: format!("cannot read {}: {error}", project_json.display()),
            })?;
        let value: serde_json::Value =
            serde_json::from_str(&content).map_err(|error| ForgeError::PublishInvalid {
                reason: format!("malformed {}: {error}", project_json.display()),
            })?;
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");
        if id.is_empty() {
            return Err(ForgeError::PublishInvalid {
                reason: format!("{} has no usable `id`", project_json.display()),
            });
        }
        validate_cwd_project_id(id)?;
        return Ok((dir, id.to_string()));
    }
    let forge_yaml = dir.join("forge.yaml");
    if forge_yaml.is_file() {
        let content =
            std::fs::read_to_string(&forge_yaml).map_err(|error| ForgeError::PublishInvalid {
                reason: format!("cannot read {}: {error}", forge_yaml.display()),
            })?;
        let value: serde_yaml::Value =
            serde_yaml::from_str(&content).map_err(|error| ForgeError::PublishInvalid {
                reason: format!("malformed {}: {error}", forge_yaml.display()),
            })?;
        if let Some(id) = value
            .get("project")
            .and_then(|p| p.get("id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            validate_cwd_project_id(id)?;
            return Ok((dir, id.to_string()));
        }
    }
    let basename = dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ForgeError::PublishInvalid {
            reason: format!(
                "publish cwd `{}` has no usable project id; use --folder or add .project.json",
                dir.display()
            ),
        })?
        .to_string();
    validate_cwd_project_id(&basename).map_err(|_| ForgeError::PublishInvalid {
        reason: format!(
            "publish cwd `{}` has no usable project id `{basename}`; use --folder or add .project.json with a valid id",
            dir.display()
        ),
    })?;
    Ok((dir, basename))
}

fn validate_cwd_project_id(id: &str) -> Result<(), ForgeError> {
    if id.is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: "project id must not be empty".to_string(),
        });
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "project id `{id}` must be kebab/snake-case (letters, digits, dash, underscore)"
            ),
        });
    }
    Ok(())
}

fn cmd_publish(
    db_path: &Path,
    command: Option<&PublishCommands>,
    project: Option<&str>,
    folder: Option<&Path>,
    cwd: Option<&Path>,
    provider: Option<&str>,
    revision: Option<&str>,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    if project.is_some() || folder.is_some() {
        return cmd_publish_provider(
            db_path, project, folder, provider, revision, dry_run, format,
        );
    }
    if let Some(cmd) = command {
        match cmd {
            PublishCommands::Provider { command } => {
                return cmd_publish_provider_lifecycle(command, format)
            }
            PublishCommands::Fleet {
                fleet_registry,
                workspace_root,
                inventory,
                dry_run,
                lifecycle,
                fail_fast,
                provider,
            } => {
                return cmd_publish_fleet(
                    db_path,
                    inventory.clone(),
                    fleet_registry.clone(),
                    workspace_root.clone(),
                    *dry_run,
                    lifecycle.clone(),
                    *fail_fast,
                    provider.clone(),
                    format,
                )
            }
            _ => return cmd_publish_single(db_path, cmd, format),
        }
    }
    let (project_dir, project_id) = discover_cwd_publish_target(cwd)?;
    return publish_via_provider_dir(
        db_path,
        &project_dir,
        &project_id,
        provider,
        revision,
        dry_run,
        format,
    );
}

fn publish_via_provider_dir(
    db_path: &Path,
    project_dir: &Path,
    project_id: &str,
    provider: Option<&str>,
    revision: Option<&str>,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let provider_id = provider
        .map(str::to_string)
        .or_else(|| std::env::var("FORGE_PUBLISH_PROVIDER").ok())
        .ok_or_else(|| ForgeError::PublishInvalid {
            reason: "publish requires --provider or FORGE_PUBLISH_PROVIDER".to_string(),
        })?;
    let config_path = std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"));
    let config = load_publish_provider_config(&config_path)?;
    let entry = select_provider(&config, &provider_id)?;
    let revision = revision
        .map(str::to_string)
        .or_else(|| git_revision(project_dir))
        .unwrap_or_else(|| "unknown".to_string());
    forge::publish::providers::validate_revision(&revision).map_err(|error| {
        ForgeError::PublishInvalid {
            reason: format!(
                "publish requires a 40-character hex revision; got `{revision}` ({error})"
            ),
        }
    })?;
    let operation_id = format!(
        "publish-{project_id}-{}",
        &revision[..revision.len().min(12)]
    );
    let request = PublishProviderRequest {
        contract: forge::publish::providers::PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: ProviderOperation::Publish,
        provider: provider_id.clone(),
        project_id: project_id.to_string(),
        revision: revision.clone(),
        operation_id,
        folder: Some(project_dir.display().to_string()),
        dry_run,
        queue_id: None,
    };
    let response = if dry_run {
        serde_json::Value::from(serde_json::to_value(&request).map_err(|error| {
            ForgeError::PublishInvalid {
                reason: format!("cannot encode publish request: {error}"),
            }
        })?)
    } else {
        let response = invoke_provider(&entry, &request, project_dir)?;
        let registry = open_registry(db_path)?;
        let phase_revision = response
            .revision
            .clone()
            .unwrap_or_else(|| revision.clone());
        let container_identity = response.container_identity.clone().unwrap_or_else(|| {
            forge::publish::providers::compose_project_name(project_id, &phase_revision)
        });
        registry.record_publish_phase(
            project_id,
            &response.status,
            PublishPhaseEvidence::new()
                .revision(&phase_revision)
                .container_identity(&container_identity)
                .build_status_opt(response.build_status.as_deref())
                .run_status_opt(response.run_status.as_deref()),
            Some(&format!(
                "provider={} health={}",
                response.provider, response.health
            )),
        )?;
        serde_json::to_value(response).map_err(|error| ForgeError::PublishInvalid {
            reason: format!("cannot encode publish response: {error}"),
        })?
    };
    let human = if dry_run {
        format!("publish dry-run: provider={provider_id} project={project_id}")
    } else {
        format!("publish: provider={provider_id} project={project_id}")
    };
    Ok(as_output(format, human, response))
}

fn cmd_publish_provider_lifecycle(
    command: &PublishProviderCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let config_path = match command {
        PublishProviderCommands::List { config }
        | PublishProviderCommands::Inspect { config, .. }
        | PublishProviderCommands::Enable { config, .. }
        | PublishProviderCommands::Disable { config, .. } => config
            .clone()
            .unwrap_or_else(|| PathBuf::from(".forge/providers.yaml")),
    };
    let mut config = load_publish_provider_config(&config_path)?;
    match command {
        PublishProviderCommands::List { .. } => {
            let value =
                serde_json::to_value(&config).map_err(|error| ForgeError::PublishInvalid {
                    reason: format!("cannot encode provider list: {error}"),
                })?;
            let human = if config.providers.is_empty() {
                "No publish providers configured.".to_string()
            } else {
                config
                    .providers
                    .iter()
                    .map(|entry| {
                        format!("{}\t{}", entry.id, if entry.enabled { "on" } else { "off" })
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(as_output(format, human, value))
        }
        PublishProviderCommands::Inspect { id, .. } => {
            let entry = config
                .providers
                .iter()
                .find(|entry| entry.id == *id)
                .ok_or_else(|| ForgeError::PublishInvalid {
                    reason: format!("publish provider `{id}` is not configured"),
                })?;
            let value =
                serde_json::to_value(entry).map_err(|error| ForgeError::PublishInvalid {
                    reason: format!("cannot encode provider inspection: {error}"),
                })?;
            Ok(as_output(
                format,
                format!(
                    "publish provider `{id}`: {} ({})",
                    if entry.enabled { "enabled" } else { "disabled" },
                    entry.command.display()
                ),
                value,
            ))
        }
        PublishProviderCommands::Enable { id, .. }
        | PublishProviderCommands::Disable { id, .. } => {
            let enabled = matches!(command, PublishProviderCommands::Enable { .. });
            let entry = config
                .providers
                .iter_mut()
                .find(|entry| entry.id == *id)
                .ok_or_else(|| ForgeError::PublishInvalid {
                    reason: format!("publish provider `{id}` is not configured"),
                })?;
            entry.enabled = enabled;
            let bytes =
                serde_yaml::to_string(&config).map_err(|error| ForgeError::PublishInvalid {
                    reason: format!("cannot encode provider config: {error}"),
                })?;
            std::fs::write(&config_path, bytes).map_err(|error| ForgeError::PublishInvalid {
                reason: format!(
                    "cannot write provider config {}: {error}",
                    config_path.display()
                ),
            })?;
            let value = serde_json::json!({"provider": id, "enabled": enabled});
            Ok(as_output(
                format,
                format!(
                    "publish provider `{id}` {}",
                    if enabled { "enabled" } else { "disabled" }
                ),
                value,
            ))
        }
    }
}

fn cmd_publish_provider(
    db_path: &Path,
    project: Option<&str>,
    folder: Option<&Path>,
    provider: Option<&str>,
    revision: Option<&str>,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = if let Some(project) = project {
        resolve_publish_target(project)?
    } else if let Some(folder) = folder {
        let dir = std::fs::canonicalize(folder).map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "cannot resolve publish folder {}: {error}",
                folder.display()
            ),
        })?;
        let id = dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ForgeError::PublishInvalid {
                reason: format!(
                    "publish folder `{}` has no usable project id",
                    dir.display()
                ),
            })?
            .to_string();
        (dir, id)
    } else {
        return Err(ForgeError::PublishInvalid {
            reason: "publish requires --project or --folder".to_string(),
        });
    };
    let provider_id = provider
        .map(str::to_string)
        .or_else(|| std::env::var("FORGE_PUBLISH_PROVIDER").ok())
        .ok_or_else(|| ForgeError::PublishInvalid {
            reason: "publish requires --provider or FORGE_PUBLISH_PROVIDER".to_string(),
        })?;
    let config_path = std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"));
    let config = load_publish_provider_config(&config_path)?;
    let entry = select_provider(&config, &provider_id)?;
    let revision = revision
        .map(str::to_string)
        .or_else(|| git_revision(&project_dir))
        .unwrap_or_else(|| "unknown".to_string());
    forge::publish::providers::validate_revision(&revision).map_err(|error| {
        ForgeError::PublishInvalid {
            reason: format!(
                "publish requires a 40-character hex revision; got `{revision}` ({error})"
            ),
        }
    })?;
    let operation_id = format!(
        "publish-{project_id}-{}",
        &revision[..revision.len().min(12)]
    );
    let request = PublishProviderRequest {
        contract: forge::publish::providers::PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: ProviderOperation::Publish,
        provider: provider_id.clone(),
        project_id: project_id.clone(),
        revision: revision.clone(),
        operation_id,
        folder: Some(project_dir.display().to_string()),
        dry_run,
        queue_id: None,
    };
    let response = if dry_run {
        serde_json::Value::from(serde_json::to_value(&request).map_err(|error| {
            ForgeError::PublishInvalid {
                reason: format!("cannot encode publish request: {error}"),
            }
        })?)
    } else {
        let response = invoke_provider(&entry, &request, &project_dir)?;
        let registry = open_registry(db_path)?;
        let phase_revision = response
            .revision
            .clone()
            .unwrap_or_else(|| revision.clone());
        let container_identity = response.container_identity.clone().unwrap_or_else(|| {
            forge::publish::providers::compose_project_name(&project_id, &phase_revision)
        });
        registry.record_publish_phase(
            &project_id,
            &response.status,
            PublishPhaseEvidence::new()
                .revision(&phase_revision)
                .container_identity(&container_identity)
                .build_status_opt(response.build_status.as_deref())
                .run_status_opt(response.run_status.as_deref()),
            Some(&format!(
                "provider={} health={}",
                response.provider, response.health
            )),
        )?;
        serde_json::to_value(response).map_err(|error| ForgeError::PublishInvalid {
            reason: format!("cannot encode publish response: {error}"),
        })?
    };
    let human = if dry_run {
        format!("publish dry-run: provider={provider_id} project={project_id}")
    } else {
        format!("publish: provider={provider_id} project={project_id}")
    };
    Ok(as_output(format, human, response))
}

fn git_revision(project_dir: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|revision| !revision.is_empty())
}

fn cmd_publish_single(
    db_path: &Path,
    command: &PublishCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id, action, dry_run) = match command {
        PublishCommands::Provider { .. } => unreachable!("Provider handled by cmd_publish"),
        PublishCommands::Sync { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::Sync, *dry_run)
        }
        PublishCommands::Db { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::Db, *dry_run)
        }
        PublishCommands::Prepare { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::Prepare, *dry_run)
        }
        PublishCommands::Deploy { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::Deploy, *dry_run)
        }
        PublishCommands::All { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::All, *dry_run)
        }
        PublishCommands::Fleet { .. } => unreachable!("Fleet handled by cmd_publish_fleet"),
    };

    let request = build_publish_request(project_id.clone(), project_dir, action, dry_run);
    // Decoupled default with one-cycle legacy rollback (`decoupled-remote-publish`
    // task 2.6/3.5): `FORGE_PUBLISH_ADAPTER=jenkins` restores the Mac-script
    // lane; every other value (including unset) selects `remote-compose`.
    // No new CLI flags; both adapters stay compiled.
    let jenkins_adapter = JenkinsAdapter::from_env();
    let remote_adapter = RemoteComposeAdapter::from_env();
    let adapter: &dyn forge::publish::PublishAdapter = if use_legacy_publish_adapter() {
        &jenkins_adapter
    } else {
        &remote_adapter
    };
    let registry = open_registry(db_path)?;
    let transport = SubprocessTransport::default();
    let report = run_publish(&request, adapter, &transport, Some(&registry))?;

    let human = render_publish_report_human(&report);
    let mut value = match serde_json::to_value(&report) {
        Ok(v) => v,
        Err(err) => {
            return Err(ForgeError::PublishInvalid {
                reason: format!("cannot encode publish report: {err}"),
            })
        }
    };
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "contract".to_string(),
            serde_json::json!(PUBLISH_CONTRACT_VERSION),
        );
    }
    if !report.healthy {
        // Surface the report so the operator sees which stage failed
        // even when the aggregate is unhealthy.
        render_output(as_output(format, human, value));
        return Err(ForgeError::PublishDeployFailed {
            reason: report.note.clone(),
        });
    }
    Ok(as_output(format, human, value))
}

/// Publish every declared project in the resolved inventory.
///
/// Inventory resolution order:
/// 1. `--inventory <path>` — local JSON file or external adapter
///    executable (consumes the
///    `forge-project-inventory/0.1.0` contract).
/// 2. `--fleet-registry <path>` or `$FORGE_WORKSPACE_REGISTRY` —
///    legacy workspace-governance `projects.json` (compatibility
///    adapter that synthesizes an inventory snapshot).
///
/// Every declared entry is reported as `compose_ready`,
/// `compose_missing`, `invalid`, or `source_unavailable`. Only
/// `compose_ready` entries invoke a provider; the others are
/// surfaced to the operator instead of silently omitted.
fn cmd_publish_fleet(
    db_path: &Path,
    inventory: Option<std::path::PathBuf>,
    registry_path: Option<std::path::PathBuf>,
    workspace_root: Option<std::path::PathBuf>,
    dry_run: bool,
    lifecycle: String,
    fail_fast: bool,
    provider: Option<String>,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::publish::fleet::{default_registry_path, default_workspace_root};
    use forge::publish::inventory::{
        classify, resolve_source, InventoryClassification, DEFAULT_DOMAIN,
    };

    // Resolve the inventory: explicit `--inventory` first, then the
    // legacy registry path (compatibility adapter).
    let (snapshot, source_label) = if let Some(path) = inventory
        .as_ref()
        .map(|p| p.to_path_buf())
        .or_else(|| resolve_source(None))
        .or(registry_path.clone())
    {
        let snapshot = load_inventory_snapshot(&path)?;
        let label = format!("inventory:{}", path.display());
        (snapshot, label)
    } else {
        // No explicit source — synthesise an inventory from the
        // legacy workspace-governance registry so the seven-project
        // handoff keeps working without configuration.
        let workspace_root = workspace_root.unwrap_or_else(default_workspace_root);
        let registry_path =
            registry_path.unwrap_or_else(|| default_registry_path(Some(&workspace_root)));
        let snapshot = legacy_inventory_snapshot(&registry_path, &workspace_root, &lifecycle)?;
        (snapshot, format!("registry:{}", registry_path.display()))
    };

    let fleet_report = classify(&snapshot, DEFAULT_DOMAIN);
    let eligible: Vec<forge::publish::inventory::InventoryFleetEntry> = fleet_report
        .entries
        .iter()
        .filter(|entry| entry.classification == InventoryClassification::ComposeReady)
        .cloned()
        .collect();
    let skipped: Vec<&forge::publish::inventory::InventoryFleetEntry> = fleet_report
        .entries
        .iter()
        .filter(|entry| entry.classification != InventoryClassification::ComposeReady)
        .collect();

    if eligible.is_empty() {
        let declared = snapshot.declared_count();
        let summary = serde_json::json!({
            "contract": forge::publish::inventory::INVENTORY_CONTRACT_VERSION,
            "inventory_source": source_label,
            "provider": snapshot.provider,
            "generated_at": snapshot.generated_at,
            "declared": declared,
            "compose_ready": 0,
            "skipped": skipped.iter().map(|entry| {
                serde_json::json!({
                    "id": entry.id,
                    "classification": entry.classification.as_str(),
                    "reason": entry.reason,
                })
            }).collect::<Vec<_>>(),
        });
        let human = format!(
            "fleet publish: 0/{declared} compose_ready in {source_label}; nothing to publish"
        );
        render_output(as_output(format, human, summary));
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "no compose_ready entries in inventory source `{source_label}`; \
                 check that every project declares a Compose file at a resolvable source path"
            ),
        });
    }

    let jenkins_adapter = JenkinsAdapter::from_env();
    let remote_adapter = RemoteComposeAdapter::from_env();
    let adapter: &dyn forge::publish::PublishAdapter = if use_legacy_publish_adapter() {
        &jenkins_adapter
    } else {
        &remote_adapter
    };
    let core_registry = open_registry(db_path)?;
    let transport = SubprocessTransport::default();

    // One fleet run, one stable queue id. The id is part of every
    // queued/running/terminal journal row this loop writes so
    // `forge deploy status --queue <id> --watch` can poll the
    // single fleet invocation end to end. ASCII alphanumeric plus
    // `-`/`_` characters only — `validate_queue_id` re-checks before
    // the loop starts so a clock skew never escapes into the
    // journal.
    let queue_id = format!(
        "fleet-{}-{:08x}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| (d.as_nanos() as u64) & 0xffff_ffff)
            .unwrap_or(0)
    );
    forge::publish::providers::validate_queue_id(&queue_id).map_err(|_| {
        ForgeError::PublishInvalid {
            reason: format!("generated queue id `{queue_id}` failed shape validation"),
        }
    })?;
    let mut queue_op_ids: Vec<(String, i64)> = Vec::with_capacity(eligible.len());

    let mut summaries: Vec<serde_json::Value> = Vec::new();
    let mut first_failure: Option<ForgeError> = None;
    let mut success_count = 0usize;
    let mut failure_count = 0usize;

    for entry in &eligible {
        let project_id = entry.id.clone();
        let source_path = entry
            .source_path
            .as_deref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        // The provider transport needs an on-disk folder to compose.
        // Entries declared as remote-only git sources fail closed
        // here rather than fabricating a synthetic workdir; the
        // classification step above already surfaced the gap so the
        // operator can remediate.
        if !source_path.is_dir() {
            failure_count += 1;
            let detail = format!(
                "fleet publish: project `{project_id}` source `{}` is not a directory",
                source_path.display()
            );
            summaries.push(serde_json::json!({
                "project": project_id,
                "healthy": false,
                "error": "source_unavailable_at_invoke",
            }));
            if let Ok(op_id) = core_registry.record_queue_operation(
                "publish",
                &project_id,
                &queue_id,
                "failed",
                Some(&detail),
            ) {
                queue_op_ids.push((project_id.clone(), op_id));
            }
            if fail_fast && first_failure.is_none() {
                first_failure = Some(ForgeError::PublishInvalid { reason: detail });
                break;
            }
            continue;
        }

        // Optional external `--provider` branch. Honours the same
        // phase-evidence contract as the manual path so the journal
        // rows are byte-equivalent regardless of provider selection.
        if let Some(provider_id) = provider.as_deref() {
            let outcome = cmd_publish_provider(
                db_path,
                None,
                Some(&source_path),
                Some(provider_id),
                None,
                dry_run,
                Format::Json,
            );
            match outcome {
                Ok(Output::Json(value)) => {
                    let healthy = value
                        .get("health")
                        .and_then(|v| v.as_str())
                        .map(|v| v == "healthy")
                        .unwrap_or(dry_run);
                    render_output(as_output(
                        format,
                        format!("publish {project_id}: provider={provider_id}"),
                        value.clone(),
                    ));
                    summaries.push(serde_json::json!({
                        "project": project_id,
                        "provider": provider_id,
                        "healthy": healthy,
                        "status": value.get("status"),
                        "build_status": value.get("build_status"),
                        "run_status": value.get("run_status"),
                        "container_identity": value.get("container_identity"),
                    }));
                    let terminal_state = if healthy { "done" } else { "failed" };
                    let phase_revision = value.get("revision").and_then(|v| v.as_str());
                    let build_status = value.get("build_status").and_then(|v| v.as_str());
                    let run_status = value.get("run_status").and_then(|v| v.as_str());
                    let container_identity = value
                        .get("container_identity")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            phase_revision.map(|rev| {
                                forge::publish::providers::compose_project_name(&project_id, rev)
                            })
                        });
                    let mut phase = PublishPhaseEvidence::new();
                    if let Some(rev) = phase_revision {
                        phase = phase.revision(rev);
                    }
                    if let Some(b) = build_status {
                        phase = phase.build_status(b);
                    }
                    if let Some(r) = run_status {
                        phase = phase.run_status(r);
                    }
                    if let Some(c) = container_identity.as_deref() {
                        phase = phase.container_identity(c);
                    }
                    if let Ok(op_id) = core_registry.record_queue_publish_phase(
                        &project_id,
                        &queue_id,
                        terminal_state,
                        phase,
                        Some(&format!("fleet provider={provider_id}")),
                    ) {
                        queue_op_ids.push((project_id.clone(), op_id));
                    }
                    if healthy {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                        if fail_fast && first_failure.is_none() {
                            first_failure = Some(ForgeError::PublishDeployFailed {
                                reason: format!("fleet provider publish failed at `{project_id}`"),
                            });
                            break;
                        }
                    }
                }
                Ok(Output::Human(text)) => {
                    println!("{text}");
                    success_count += 1;
                    if let Ok(op_id) = core_registry.record_queue_operation(
                        "publish",
                        &project_id,
                        &queue_id,
                        "done",
                        Some(&format!("fleet provider={provider_id}")),
                    ) {
                        queue_op_ids.push((project_id.clone(), op_id));
                    }
                }
                Err(error) => {
                    failure_count += 1;
                    let detail = format!("fleet provider={provider_id} failed: {error}");
                    render_output(as_output(
                        format,
                        format!("publish {project_id} failed: {error}"),
                        serde_json::json!({"project": project_id, "error": error.to_string()}),
                    ));
                    if let Ok(op_id) = core_registry.record_queue_operation(
                        "publish",
                        &project_id,
                        &queue_id,
                        "failed",
                        Some(&detail),
                    ) {
                        queue_op_ids.push((project_id.clone(), op_id));
                    }
                    if fail_fast && first_failure.is_none() {
                        first_failure = Some(error);
                        break;
                    }
                }
            }
            continue;
        }

        let request =
            build_publish_request(project_id.clone(), source_path, PublishAction::All, dry_run);
        let outcome = run_publish(&request, adapter, &transport, Some(&core_registry));
        match outcome {
            Ok(report) => {
                let human = render_publish_report_human(&report);
                let mut value =
                    serde_json::to_value(&report).map_err(|err| ForgeError::PublishInvalid {
                        reason: format!("cannot encode report for {project_id}: {err}"),
                    })?;
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(
                        "contract".to_string(),
                        serde_json::json!(PUBLISH_CONTRACT_VERSION),
                    );
                    if let Some(subdomain) = entry.subdomain.as_deref() {
                        obj.insert(
                            "inventory_subdomain".to_string(),
                            serde_json::json!(subdomain),
                        );
                    }
                }
                render_output(as_output(format, human.clone(), value.clone()));
                let healthy = report.healthy;
                summaries.push(serde_json::json!({
                    "project": project_id,
                    "healthy": healthy,
                    "subdomain": report.subdomain,
                    "stages": report.stages.len(),
                }));
                let terminal_state = if healthy { "done" } else { "failed" };
                if let Ok(op_id) = core_registry.record_queue_operation(
                    "publish",
                    &project_id,
                    &queue_id,
                    terminal_state,
                    Some(&format!(
                        "fleet stages={} healthy={}",
                        report.stages.len(),
                        healthy
                    )),
                ) {
                    queue_op_ids.push((project_id.clone(), op_id));
                }
                if healthy {
                    success_count += 1;
                } else {
                    failure_count += 1;
                    if fail_fast && first_failure.is_none() {
                        first_failure = Some(ForgeError::PublishDeployFailed {
                            reason: format!("fleet publish failed at `{project_id}`"),
                        });
                        break;
                    }
                }
            }
            Err(err) => {
                failure_count += 1;
                let detail = format!("fleet publish failed: {err}");
                let summary = serde_json::json!({
                    "project": project_id,
                    "healthy": false,
                    "error": err.to_string(),
                });
                summaries.push(summary);
                if let Ok(op_id) = core_registry.record_queue_operation(
                    "publish",
                    &project_id,
                    &queue_id,
                    "failed",
                    Some(&detail),
                ) {
                    queue_op_ids.push((project_id.clone(), op_id));
                }
                if fail_fast && first_failure.is_none() {
                    first_failure = Some(err);
                    break;
                }
            }
        }
    }

    let skipped_summary: Vec<serde_json::Value> = skipped
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.id,
                "runtime": entry.runtime.as_str(),
                "classification": entry.classification.as_str(),
                "reason": entry.reason,
            })
        })
        .collect();
    let summary = serde_json::json!({
        "contract": forge::publish::inventory::INVENTORY_CONTRACT_VERSION,
        "inventory_source": source_label,
        "provider": snapshot.provider,
        "generated_at": snapshot.generated_at,
        "fleet": {
            "queue_id": queue_id,
            "declared": snapshot.declared_count(),
            "compose_ready": eligible.len(),
            "skipped": skipped.len(),
            "success": success_count,
            "failed": failure_count,
            "projects": summaries,
            "skipped_entries": skipped_summary,
        },
    });
    let human = format!(
        "fleet publish: {}/{} compose_ready succeeded, {} skipped ({} in {source_label})",
        success_count,
        eligible.len(),
        skipped.len(),
        snapshot.declared_count(),
    );

    render_output(as_output(format, human, summary.clone()));
    if let Some(err) = first_failure {
        Err(err)
    } else if failure_count > 0 {
        Err(ForgeError::PublishDeployFailed {
            reason: format!(
                "fleet publish finished with {failure_count} failure(s); see per-project reports"
            ),
        })
    } else {
        Ok(Output::Human(String::new()))
    }
}

/// Load an inventory snapshot from a local JSON file or an
/// external adapter executable. The path is checked for `is_file`
/// so an external adapter is distinguishable from a regular file.
fn load_inventory_snapshot(
    path: &std::path::Path,
) -> Result<forge::publish::inventory::InventorySnapshot, ForgeError> {
    use forge::publish::inventory::invoke_external;
    if path.is_file() {
        // Distinguish the JSON contract from an external adapter by
        // checking the suffix: `.json` always means local; anything
        // else is an adapter executable.
        let is_json = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("json"))
            .unwrap_or(false);
        if is_json {
            return forge::publish::inventory::load_local(path);
        }
        return invoke_external(path, std::path::Path::new("."));
    }
    Err(ForgeError::PublishInvalid {
        reason: format!(
            "inventory source `{}` is not a local file or adapter executable",
            path.display()
        ),
    })
}

/// Compatibility adapter that synthesizes an inventory snapshot
/// from the legacy workspace-governance `projects.json` registry.
/// Used when no `--inventory` is supplied so the seven-project
/// handoff keeps working during migration.
fn legacy_inventory_snapshot(
    registry_path: &std::path::Path,
    workspace_root: &std::path::Path,
    lifecycle: &str,
) -> Result<forge::publish::inventory::InventorySnapshot, ForgeError> {
    use forge::publish::fleet::{filter_eligible, load_registry};
    use forge::publish::inventory::{
        InventoryEntry, InventoryMalformedEntry, InventorySnapshot, RuntimeClass,
        INVENTORY_CONTRACT_VERSION,
    };

    let registry = load_registry(registry_path).map_err(|err| match err {
        ForgeError::PublishInvalid { reason } => ForgeError::PublishInvalid {
            reason: format!(
                "legacy fleet registry `{registry_path}`: {reason}",
                registry_path = registry_path.display()
            ),
        },
        other => other,
    })?;
    let eligible = filter_eligible(&registry, workspace_root, lifecycle);
    let now = forge::publish::inventory::now_rfc3339();
    let mut projects = Vec::with_capacity(eligible.len());
    let mut malformed = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // Surface every declared entry — even those the legacy filter
    // excluded — so the operator sees the complete fleet.
    for entry in &registry.projects {
        if !seen.insert(entry.id.clone()) {
            malformed.push(InventoryMalformedEntry {
                name: entry.id.clone(),
                reason: "duplicate fleet registry id".to_string(),
            });
            continue;
        }
        let declared_lifecycle = entry.lifecycle.as_deref().unwrap_or("active");
        let compose = if eligible.iter().any(|e| e.id == entry.id) {
            "docker-compose.yml"
        } else if declared_lifecycle != lifecycle {
            continue; // non-matching lifecycle — skip without claiming invalid
        } else {
            malformed.push(InventoryMalformedEntry {
                name: entry.id.clone(),
                reason: format!(
                    "legacy registry entry has no Compose file at `{}`",
                    entry.path.as_deref().unwrap_or("<unspecified>")
                ),
            });
            continue;
        };
        // Revision is unknown from the legacy registry; a future
        // git-aware adapter replaces this. Synthesize an explicit
        // sentinel so the contract field stays populated.
        let revision = "0".repeat(40);
        projects.push(InventoryEntry {
            id: entry.id.clone(),
            repository: format!("legacy-registry:{}", registry_path.display()),
            revision,
            profile: entry.profile.clone().unwrap_or_default(),
            runtime: RuntimeClass::Web,
            compose_file: Some(compose.to_string()),
            source_path: entry
                .path
                .as_deref()
                .map(|path| workspace_root.join(path).display().to_string()),
            public_http: false,
            public_port: None,
        });
    }
    Ok(InventorySnapshot {
        contract: INVENTORY_CONTRACT_VERSION.to_string(),
        provider: "legacy-fleet-registry".to_string(),
        generated_at: now,
        source: Some(registry_path.display().to_string()),
        projects,
        malformed,
    })
}

/// Select the publish adapter without a new CLI flag (`decoupled-remote-publish`
/// task 2.6). `FORGE_PUBLISH_ADAPTER=jenkins` (or `legacy`/`mac-scripts`)
/// restores the Mac-script lane for one-cycle rollback; every other value, including
/// unset, selects `remote-compose`. Both adapters stay compiled.
fn use_legacy_publish_adapter() -> bool {
    matches!(
        std::env::var(forge::publish::remote_compose::ADAPTER_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "jenkins" | "legacy" | "mac-scripts"
    )
}

fn resolve_publish_target(target: &str) -> Result<(std::path::PathBuf, String), ForgeError> {
    let candidate = std::path::Path::new(target);
    if candidate.is_dir() {
        let canonical = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        // Manifest is optional for publish: the Jenkins adapter only
        // needs the project id (derived from the directory name) and
        // the docker-compose file inside. Skip the manifest lookup so
        // legacy projects without forge.yaml can still be published.
        let id = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        return Ok((canonical, id));
    }
    // Fall back to the registry so callers can pass a project id.
    let db_path = default_registry_path();
    if let Ok(registry) = Registry::open(&db_path) {
        if let Ok(record) = registry.inspect(target) {
            let path = std::path::PathBuf::from(&record.path);
            if path.is_dir() {
                return Ok((path, record.id));
            }
        }
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
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

fn cmd_procedure(
    db_path: &Path,
    command: &ProcedureCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ProcedureCommands::List => {
            let entries = procedure_catalog();
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "procedure",
                    PROCEDURE_SYNTHETIC_PROJECT,
                    "done",
                    &format!("list: {} procedure(s) returned", entries.len()),
                );
            }
            let human = render_procedure_list_human(&entries);
            let json = serde_json::json!({
                "contract": PROCEDURE_CONTRACT_VERSION,
                "procedures": entries,
            });
            Ok(as_output(format, human, json))
        }
        ProcedureCommands::Inspect { id } => match inspect_procedure(id) {
            Ok(spec) => {
                if let Ok(registry) = open_registry(db_path) {
                    let _ = registry.record_operation(
                        "procedure",
                        &spec.id,
                        "done",
                        &format!("inspect: {} step(s)", spec.steps.len()),
                    );
                }
                let human = render_procedure_inspect_human(&spec);
                let json = serde_json::json!({
                    "contract": PROCEDURE_CONTRACT_VERSION,
                    "procedure": spec,
                });
                Ok(as_output(format, human, json))
            }
            Err(err) => {
                if let Ok(registry) = open_registry(db_path) {
                    let _ =
                        registry.record_operation("procedure", id, "rejected", &err.to_string());
                }
                Err(err)
            }
        },
        ProcedureCommands::Validate { path } => {
            let bytes = std::fs::read(path).map_err(|err| ForgeError::ProcedureInvalid {
                reason: format!("could not read procedure spec at {}: {err}", path.display()),
            })?;
            let spec: ProcedureSpec =
                serde_json::from_slice(&bytes).map_err(|err| ForgeError::ProcedureInvalid {
                    reason: format!(
                        "procedure spec at {} is not valid JSON ProcedureSpec: {err}",
                        path.display()
                    ),
                })?;
            let validated = validate_procedure(&spec)?;
            let note = format!(
                "procedure '{}' validated: {} step(s) and ends with report_findings",
                validated.id,
                validated.steps.len()
            );
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation("procedure", &validated.id, "done", &note);
            }
            let json = serde_json::json!({
                "contract": PROCEDURE_CONTRACT_VERSION,
                "procedure": validated,
                "note": note,
            });
            Ok(as_output(
                format,
                render_procedure_inspect_human(&validated),
                json,
            ))
        }
    }
}

fn cmd_identity(
    db_path: &Path,
    command: &IdentityCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        IdentityCommands::ValidateConfig { target } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
            match IdentityConfig::from_manifest_opt(&project_id, &manifest)? {
                Some(cfg) => {
                    let detail = format!(
                        "validate-config: project={} provider={} issuer={} client_id={} \
                         audience={} redirect_uri={} scopes={} admin_claim={} admin_values={} \
                         state_ttl={} session_ttl={}",
                        project_id,
                        cfg.provider,
                        redact_identity_evidence(&cfg.issuer),
                        cfg.client_id,
                        cfg.audience,
                        cfg.redirect_uri,
                        cfg.scopes.len(),
                        cfg.admin_claim,
                        cfg.admin_values.len(),
                        cfg.state_ttl_seconds,
                        cfg.session_ttl_seconds,
                    );
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation("identity", &project_id, "done", &detail);
                    }
                    let json = serde_json::json!({
                        "contract": IDENTITY_CONTRACT_VERSION,
                        "project_id": project_id,
                        "config": cfg,
                    });
                    Ok(as_output(
                        format,
                        format!(
                            "identity config validated for project `{}`: provider={} \
                             issuer={} client_id={} scopes={} admin_claim={} \
                             state_ttl={}s session_ttl={}s",
                            project_id,
                            cfg.provider,
                            redact_identity_evidence(&cfg.issuer),
                            cfg.client_id,
                            cfg.scopes.join(","),
                            cfg.admin_claim,
                            cfg.state_ttl_seconds,
                            cfg.session_ttl_seconds,
                        ),
                        json,
                    ))
                }
                None => {
                    let detail = "validate-config: project has no identity block".to_string();
                    if let Ok(registry) = open_registry(db_path) {
                        let _ =
                            registry.record_operation("identity", &project_id, "rejected", &detail);
                    }
                    Err(ForgeError::IdentityInvalid {
                        reason: format!(
                            "project `{project_id}` has no `identity:` block; declare one in \
                             forge.yaml to enable OIDC admin federation"
                        ),
                    })
                }
            }
        }
        IdentityCommands::BuildChallenge { target } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
            let cfg =
                IdentityConfig::from_manifest_opt(&project_id, &manifest)?.ok_or_else(|| {
                    ForgeError::IdentityInvalid {
                        reason: format!(
                            "project `{project_id}` has no `identity:` block; declare one before \
                         building an auth challenge"
                        ),
                    }
                })?;
            let now = chrono::Utc::now();
            let challenge = build_challenge(&project_id, &cfg, now)?;
            save_challenge(&dir, &project_id, &challenge)?;
            let detail = format!(
                "build-challenge: project={} state={} nonce={} code_challenge={}",
                project_id, challenge.state, challenge.nonce, challenge.code_challenge
            );
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation("identity", &project_id, "done", &detail);
            }
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "challenge": challenge,
            });
            Ok(as_output(format, render_challenge_human(&challenge), json))
        }
        IdentityCommands::CompleteAuth {
            target,
            state,
            code,
            error,
            error_description,
            subject,
            issuer,
            audience,
            nonce,
            issued_at,
            expires_at,
            scope,
            admin_claim_value,
        } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
            let cfg =
                IdentityConfig::from_manifest_opt(&project_id, &manifest)?.ok_or_else(|| {
                    ForgeError::IdentityInvalid {
                        reason: format!(
                            "project `{project_id}` has no `identity:` block; declare one before \
                         completing the OIDC round trip"
                        ),
                    }
                })?;
            let callback = AuthCallback {
                project_id: project_id.clone(),
                state: state.clone(),
                code: code.clone(),
                error: error.clone(),
                error_description: error_description.clone(),
            };
            let now = chrono::Utc::now();
            let challenge = load_challenge(&dir, &project_id, state)?.ok_or_else(|| {
                let mut reason = format!(
                    "no pending OIDC challenge found for project `{project_id}` with state \
                         `{state}`; call `forge identity build-challenge` first and complete the \
                         round trip before the challenge expires"
                );
                if let Some(err) = error.as_deref() {
                    reason.push_str(&format!(
                        "; provider error: `{}`",
                        redact_identity_evidence(err)
                    ));
                }
                if let Some(desc) = error_description.as_deref() {
                    reason.push_str(&format!(
                        "; description: `{}`",
                        redact_identity_evidence(desc)
                    ));
                }
                ForgeError::IdentityInvalid { reason }
            })?;
            validate_callback(&callback, &challenge, now)?;
            let iat = match issued_at.as_deref() {
                Some(raw) => Some(parse_rfc3339(raw, "issued_at")?),
                None => None,
            };
            let exp = match expires_at.as_deref() {
                Some(raw) => Some(parse_rfc3339(raw, "expires_at")?),
                None => None,
            };
            let claims_iat = iat.unwrap_or(now);
            let claims_exp = exp.unwrap_or(claims_iat + chrono::Duration::seconds(60));
            let scopes = scope
                .as_deref()
                .map(|s| s.split_whitespace().map(str::to_string).collect::<Vec<_>>())
                .unwrap_or_else(|| cfg.scopes.clone());
            let mut claim_map = std::collections::BTreeMap::new();
            claim_map.insert(cfg.admin_claim.clone(), admin_claim_value.clone());
            let claims = ProviderClaims {
                issuer: issuer.clone().unwrap_or_else(|| cfg.issuer.clone()),
                audience: audience.clone().unwrap_or_else(|| cfg.audience.clone()),
                subject: subject.clone(),
                issued_at: claims_iat,
                expires_at: claims_exp,
                nonce: nonce.clone(),
                scopes,
                claims: claim_map,
            };
            validate_claims(&claims, &challenge, &cfg, now)?;
            let session = match mint_session(&cfg, &claims, &challenge, now) {
                Ok(session) => session,
                Err(err) => {
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "identity",
                            &project_id,
                            "rejected",
                            &format!("complete-auth: {}", err),
                        );
                    }
                    return Err(err);
                }
            };
            save_session(&dir, &project_id, &session)?;
            let _ = delete_identity_challenge(&dir, &project_id, state);
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "identity",
                    &project_id,
                    "done",
                    &format!(
                        "complete-auth: session {} minted for subject {}",
                        session.session_id, session.subject
                    ),
                );
            }
            let outcome = IdentityOutcome::Session(session);
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "outcome": outcome,
            });
            Ok(as_output(
                format,
                render_identity_outcome_human(&outcome),
                json,
            ))
        }
        IdentityCommands::SessionList { target } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let sessions = list_identity_sessions(&dir, &project_id)?;
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "identity",
                    &project_id,
                    "done",
                    &format!("session-list: {} session(s)", sessions.len()),
                );
            }
            let human = if sessions.is_empty() {
                format!("no admin sessions for project `{project_id}`")
            } else {
                let mut text = format!("admin sessions for project `{project_id}`:");
                for s in &sessions {
                    text.push_str(&format!("\n  - {}", render_identity_session_human(s)));
                }
                text
            };
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "sessions": sessions,
            });
            Ok(as_output(format, human, json))
        }
        IdentityCommands::SessionInspect { target, session } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let loaded = match load_session(&dir, &project_id, session)? {
                Some(session) => session,
                None => {
                    let owner = forge::identity::lookup_session_in_sibling_projects(&dir, session)?;
                    match owner {
                        Some((_, owner_id, _)) => {
                            return Err(ForgeError::IdentitySessionCrossProject {
                                reason: format!(
                                    "session `{session}` was minted for project `{owner_id}`; \
                                     inspecting it through project `{project_id}` is refused"
                                ),
                            });
                        }
                        None => {
                            return Err(ForgeError::IdentitySessionNotFound {
                                reason: format!(
                                    "session `{session}` was not found under \
                                     `.forge/identity/{project_id}/` and no other project owns it"
                                ),
                            });
                        }
                    }
                }
            };
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "identity",
                    &project_id,
                    "done",
                    &format!("session-inspect: session {session}"),
                );
            }
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "session": loaded,
            });
            Ok(as_output(
                format,
                render_identity_session_human(&loaded),
                json,
            ))
        }
        IdentityCommands::SessionValidate {
            target,
            session,
            permission,
        } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let loaded = match load_session(&dir, &project_id, session)? {
                Some(session) => session,
                None => {
                    let registry = open_registry(db_path).ok();
                    let mut owner: Option<(String, std::path::PathBuf)> = None;
                    if let Some(registry) = registry {
                        let all = registry.list().unwrap_or_default();
                        let projects: Vec<(String, std::path::PathBuf)> = all
                            .into_iter()
                            .map(|p| (p.id, std::path::PathBuf::from(p.path)))
                            .collect();
                        if let Some((_, owner_id, owner_dir)) =
                            forge::identity::lookup_session_across_projects(session, projects)?
                        {
                            owner = Some((owner_id, owner_dir));
                        }
                    }
                    if owner.is_none() {
                        if let Some((_, owner_id, owner_dir)) =
                            forge::identity::lookup_session_in_sibling_projects(&dir, session)?
                        {
                            owner = Some((owner_id, owner_dir));
                        }
                    }
                    match owner {
                        Some((owner_id, _)) => {
                            return Err(ForgeError::IdentitySessionCrossProject {
                                reason: format!(
                                    "session `{session}` was minted for project `{owner_id}`; \
                                     presenting it to project `{project_id}` is refused; \
                                     sessions are project-scoped and may not be shared across \
                                     unrelated applications"
                                ),
                            });
                        }
                        None => {
                            return Err(ForgeError::IdentitySessionNotFound {
                                reason: format!(
                                    "session `{session}` was not found under \
                                     `.forge/identity/{project_id}/` and no other registered \
                                     project owns it"
                                ),
                            });
                        }
                    }
                }
            };
            let now = chrono::Utc::now();
            match validate_session(&loaded, &project_id, permission, now) {
                Ok(()) => {
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "identity",
                            &project_id,
                            "done",
                            &format!("session-validate: session {session} granted {permission}"),
                        );
                    }
                    let json = serde_json::json!({
                        "contract": IDENTITY_CONTRACT_VERSION,
                        "project_id": project_id,
                        "session": loaded,
                        "granted": true,
                        "permission": permission,
                    });
                    Ok(as_output(
                        format,
                        format!(
                            "session `{}` granted `{permission}` for project `{project_id}`",
                            session
                        ),
                        json,
                    ))
                }
                Err(err) => {
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "identity",
                            &project_id,
                            "rejected",
                            &format!("session-validate: {} ({})", err, err.code()),
                        );
                    }
                    Err(err)
                }
            }
        }
        IdentityCommands::SessionTerminate { target, session } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let mut loaded = match load_session(&dir, &project_id, session)? {
                Some(session) => session,
                None => {
                    let owner = forge::identity::lookup_session_in_sibling_projects(&dir, session)?;
                    match owner {
                        Some((_, owner_id, _)) => {
                            return Err(ForgeError::IdentitySessionCrossProject {
                                reason: format!(
                                    "session `{session}` was minted for project `{owner_id}`; \
                                     terminating it through project `{project_id}` is refused"
                                ),
                            });
                        }
                        None => {
                            return Err(ForgeError::IdentitySessionNotFound {
                                reason: format!(
                                    "session `{session}` was not found under \
                                     `.forge/identity/{project_id}/` and no other project owns it"
                                ),
                            });
                        }
                    }
                }
            };
            terminate_session(&mut loaded);
            save_session(&dir, &project_id, &loaded)?;
            delete_session_file_at(&dir, &project_id, session)?;
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "identity",
                    &project_id,
                    "done",
                    &format!("session-terminate: session {session}"),
                );
            }
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "session": loaded,
                "terminated": true,
            });
            Ok(as_output(
                format,
                format!(
                    "session `{session}` terminated for project `{project_id}`; state=`revoked`"
                ),
                json,
            ))
        }
    }
}

fn delete_session_file_at(
    dir: &Path,
    project_id: &str,
    session_id: &str,
) -> Result<(), ForgeError> {
    let path = forge::identity::session_path_for(dir, project_id, session_id)?;
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_file(&path).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot delete session file {}: {err}", path.display()),
    })?;
    Ok(())
}

fn parse_rfc3339(raw: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>, ForgeError> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|err| ForgeError::IdentityInvalid {
            reason: format!("identity {field} `{raw}` is not RFC 3339: {err}"),
        })
}

fn cmd_analytics(
    db_path: &Path,
    command: &AnalyticsCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        AnalyticsCommands::Inspect { target, dry_run } => {
            cmd_analytics_inspect(db_path, target, *dry_run, format)
        }
        AnalyticsCommands::Metrics {
            target,
            window_days,
            all,
        } => cmd_analytics_metrics(db_path, target, *window_days, *all, format),
    }
}

fn cmd_api(db_path: &Path, command: &ApiCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        ApiCommands::Serve {
            bind,
            port,
            max_body_bytes,
        } => cmd_api_serve(db_path, *bind, *port, *max_body_bytes, format),
    }
}

fn cmd_portal(
    db_path: &Path,
    command: &PortalCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortalCommands::Dashboard { target, all } => {
            cmd_portal_dashboard(db_path, target, *all, format)
        }
        PortalCommands::View { section, target } => {
            cmd_portal_view(db_path, section, target, format)
        }
    }
}

fn cmd_portal_dashboard(
    db_path: &Path,
    target: &str,
    all: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let resolved_target = if all { "" } else { target };
    let query: Option<&str> = if resolved_target.trim().is_empty() {
        None
    } else {
        Some(resolved_target.trim())
    };
    let fleet_source = portal_fleet_projection(&registry);
    let projection = match fleet_source.as_ref() {
        None => None,
        Some(Ok(report)) => Some(FleetProjection::Configured(report)),
        Some(Err(err)) => Some(FleetProjection::Failed {
            code: err.code(),
            reason: err.to_string(),
        }),
    };
    let dashboard = build_dashboard_with_fleet(&registry, query, projection)?;
    let detail = portal_journal_detail(&dashboard);
    journal_portal_operation(&registry, dashboard.project_id.as_deref(), "done", &detail);
    let json = serde_json::json!({
        "contract": PORTAL_CONTRACT_VERSION,
        "dashboard": dashboard,
    });
    let human = render_dashboard_human(&dashboard);
    Ok(as_output(format, human, json))
}

fn cmd_portal_view(
    db_path: &Path,
    section: &str,
    target: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let parsed = parse_section(section)?;
    let registry = open_registry(db_path)?;
    let query: Option<&str> = if target.trim().is_empty() {
        None
    } else {
        Some(target.trim())
    };
    let fleet_source = portal_fleet_projection(&registry);
    let projection = match fleet_source.as_ref() {
        None => None,
        Some(Ok(report)) => Some(FleetProjection::Configured(report)),
        Some(Err(err)) => Some(FleetProjection::Failed {
            code: err.code(),
            reason: err.to_string(),
        }),
    };
    let view = build_section_view_with_fleet(&registry, query, parsed, projection)?;
    let detail = format!(
        "view: section={id} status={status} entries={n}",
        id = view.section_id,
        status = view.status.id(),
        n = view.entries.len()
    );
    journal_portal_operation(&registry, view.project_id.as_deref(), "done", &detail);
    let json = serde_json::json!({
        "contract": PORTAL_CONTRACT_VERSION,
        "view": view,
    });
    let human = render_portal_section_human(&view);
    Ok(as_output(format, human, json))
}

fn portal_journal_detail(dashboard: &PortalDashboard) -> String {
    format!(
        "dashboard: scope={scope} sections={n} rollup={rollup}",
        scope = dashboard.scope.id(),
        n = dashboard.section_count,
        rollup = dashboard.rollup().id()
    )
}

fn journal_portal_operation(
    registry: &Registry,
    project_id: Option<&str>,
    state: &str,
    detail: &str,
) {
    let journal_project = project_id.unwrap_or(PORTAL_SYNTHETIC_PROJECT);
    let _ = registry.record_operation("portal", journal_project, state, detail);
}

fn cmd_readiness(command: &ReadinessCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        ReadinessCommands::Matrix { profiles } => cmd_readiness_matrix(profiles, format),
        ReadinessCommands::Artifact => cmd_readiness_artifact(format),
        ReadinessCommands::Check { profiles } => cmd_readiness_check(profiles, format),
    }
}

fn cmd_readiness_matrix(profiles: &[String], format: Format) -> Result<Output, ForgeError> {
    // The matrix touches no registry and invents no project: every fixture
    // lives in a disposable directory and the journal stays independent of
    // the readiness surface.
    let report = run_matrix(profiles)?;
    let json = serde_json::json!({
        "contract": READINESS_CONTRACT_VERSION,
        "matrix": report,
    });
    let human = render_matrix_human(&report);
    Ok(as_output(format, human, json))
}

fn cmd_readiness_artifact(format: Format) -> Result<Output, ForgeError> {
    let evidence = artifact_evidence()?;
    let json = serde_json::json!({
        "contract": READINESS_CONTRACT_VERSION,
        "artifact": evidence,
    });
    let human = render_artifact_human(&evidence);
    Ok(as_output(format, human, json))
}

fn cmd_readiness_check(profiles: &[String], format: Format) -> Result<Output, ForgeError> {
    // The gate owns its exit code: the report always renders on stdout so a
    // blocked gate stays observable, while the typed error renders on stderr.
    let (report, gate) = evaluate_gate(profiles);
    let json = serde_json::json!({
        "contract": READINESS_CONTRACT_VERSION,
        "gate": report,
    });
    let human = render_gate_human(&report);
    match gate {
        Ok(()) => Ok(as_output(format, human, json)),
        Err(err) => {
            println!(
                "{}",
                match format {
                    Format::Human => human,
                    Format::Json => serde_json::to_string_pretty(&json).unwrap(),
                }
            );
            Err(err)
        }
    }
}

fn cmd_provider(
    db_path: &Path,
    command: &ProviderCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ProviderCommands::Matrix { live } => cmd_provider_matrix(db_path, *live, format),
        ProviderCommands::Run {
            provider,
            target,
            live,
            fixture,
            project_ref,
        } => cmd_provider_run(
            db_path,
            provider,
            target.as_deref(),
            *live,
            fixture.as_deref(),
            project_ref,
            format,
        ),
        ProviderCommands::Inspect { provider } => cmd_provider_inspect(db_path, provider, format),
    }
}

fn cmd_governance(command: &GovernanceCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        GovernanceCommands::List { path } => {
            let providers = list_governance_providers(path)?;
            let human = providers
                .iter()
                .map(|provider| {
                    format!(
                        "{}: {}{} (protocol {})",
                        provider.provider,
                        if provider.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        },
                        provider
                            .adapter
                            .as_deref()
                            .map(|adapter| format!(", adapter={adapter}"))
                            .unwrap_or_default(),
                        provider.protocol_version
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": GOVERNANCE_CONTRACT_VERSION,
                    "providers": providers,
                }),
            ))
        }
        GovernanceCommands::Status { path } | GovernanceCommands::Inspect { path } => {
            let observation = check_governance_project(path)?;
            governance_output(format, observation)
        }
        GovernanceCommands::Use {
            provider,
            path,
            adapter,
            workspace_root,
            disable,
            timeout_ms,
        } => {
            // An explicit operator path always wins; a preset never
            // overrides the operator's choice. A `--workspace-root`
            // supplied alongside it is still stored so the adapter keeps
            // receiving the same WORKSPACE_ROOT context on later checks.
            let env_root = std::env::var(WORKSPACE_ROOT_ENV).ok();
            let (adapter, used_root, preset_resolved) = if provider == LOCAL_PROVIDER_ID {
                (None, None, false)
            } else if let Some(adapter) = adapter.as_deref() {
                (
                    Some(adapter.to_string()),
                    workspace_root.as_deref().map(Path::to_path_buf),
                    false,
                )
            } else {
                match resolve_known_adapter(
                    provider,
                    workspace_root.as_deref(),
                    env_root.as_deref(),
                )
                .transpose()?
                {
                    Some(resolved) => {
                        let adapter = resolved.to_str().map(str::to_string).ok_or_else(|| {
                            ForgeError::GovernanceInvalid {
                                reason: format!(
                                    "resolved adapter path {} is not valid UTF-8 and cannot be stored",
                                    resolved.display()
                                ),
                            }
                        })?;
                        let root = match workspace_root.as_deref() {
                            Some(root) => root.to_path_buf(),
                            None => PathBuf::from(env_root.as_deref().unwrap_or("").trim()),
                        };
                        (Some(adapter), Some(root), true)
                    }
                    None => {
                        if workspace_root.is_some() {
                            return Err(ForgeError::GovernanceInvalid {
                                reason: format!(
                                    "provider `{provider}` has no packaged adapter: pass --adapter explicitly",
                                ),
                            });
                        }
                        (None, None, false)
                    }
                }
            };
            let stored_root = used_root
                .as_deref()
                .map(|root| {
                    let canonical =
                        root.canonicalize()
                            .map_err(|err| ForgeError::GovernanceInvalid {
                                reason: format!(
                                "workspace root {} cannot be resolved to an absolute path ({err})",
                                root.display()
                            ),
                            })?;
                    canonical.to_str().map(str::to_string).ok_or_else(|| {
                        ForgeError::GovernanceInvalid {
                            reason: "resolved workspace root is not valid UTF-8".to_string(),
                        }
                    })
                })
                .transpose()?;
            save_provider_selection_with_root(
                path,
                provider,
                adapter.as_deref(),
                stored_root.as_deref(),
                !disable,
                *timeout_ms,
            )?;
            let preset_note = if preset_resolved {
                adapter
                    .as_deref()
                    .map(|resolved| format!(" (adapter={resolved})"))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let detail = if provider == LOCAL_PROVIDER_ID {
                "selected built-in local provider".to_string()
            } else if *disable {
                format!("selected external provider `{provider}` (disabled){preset_note}")
            } else {
                format!("selected external provider `{provider}`{preset_note}")
            };
            Ok(as_output(
                format,
                detail,
                serde_json::json!({
                    "contract": GOVERNANCE_CONTRACT_VERSION,
                    "provider": provider,
                    "enabled": !disable,
                    "adapter": adapter,
                    "workspace_root": stored_root,
                }),
            ))
        }
    }
}

fn governance_output(
    format: Format,
    observation: GovernanceObservation,
) -> Result<Output, ForgeError> {
    let status = match observation.status {
        ProviderStatus::Pass => "pass",
        ProviderStatus::Fail => "fail",
        ProviderStatus::Blocked => "blocked",
        ProviderStatus::Unknown => "unknown",
        ProviderStatus::Unavailable => "unavailable",
        ProviderStatus::Stale => "stale",
        ProviderStatus::Disabled => "disabled",
        ProviderStatus::Incompatible => "incompatible",
    };
    let human = format!(
        "governance: {}\nprovider: {}\nproject: {}\nobserved_at: {}\nevidence: {}{}",
        status,
        observation.provider,
        observation.project_id,
        observation.observed_at,
        if observation.evidence.is_empty() {
            "(none)".to_string()
        } else {
            observation.evidence.join(" | ")
        },
        observation
            .detail
            .as_deref()
            .map(|detail| format!("\ndetail: {detail}"))
            .unwrap_or_default()
    );
    Ok(as_output(
        format,
        human,
        serde_json::json!({
            "contract": GOVERNANCE_CONTRACT_VERSION,
            "observation": observation,
        }),
    ))
}

/// Ids currently present in the local registry, used only as the
/// managed/unmanaged join set for a fleet observation. Read-only.
fn fleet_local_ids(registry: &Registry) -> Result<BTreeSet<String>, ForgeError> {
    Ok(registry.list()?.into_iter().map(|r| r.id).collect())
}

/// Observe the configured workspace registry (if any) for portal and
/// `forge list` read surfaces. A configured-but-unobservable registry
/// projects as `Some(Err(..))` so the surfaces render `unavailable`
/// rather than masking the gap; an unconfigured registry projects as
/// `None` and leaves every local surface byte-identical.
fn portal_fleet_projection(
    registry: &Registry,
) -> Option<Result<forge::fleet::FleetReport, ForgeError>> {
    let path = resolve_registry_path(None)?;
    let local_ids = match fleet_local_ids(registry) {
        Ok(ids) => ids,
        Err(err) => return Some(Err(err)),
    };
    Some(observe(Some(&path), DEFAULT_MAX_AGE_SECONDS, &local_ids))
}

fn cmd_fleet(
    db_path: &Path,
    command: &FleetCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let (workspace_registry, max_age) = match command {
        FleetCommands::List {
            workspace_registry,
            max_age,
        }
        | FleetCommands::Status {
            workspace_registry,
            max_age,
        }
        | FleetCommands::Inspect {
            workspace_registry,
            max_age,
            ..
        } => (workspace_registry.as_deref(), *max_age),
        FleetCommands::Online { .. } => (None, DEFAULT_MAX_AGE_SECONDS),
    };
    fleet::validate_max_age(max_age)?;
    // The local registry is opened only as the read side of the
    // managed/unmanaged join; fleet mirroring never registers, imports
    // or mutates anything and never journals.
    let registry = open_registry(db_path)?;
    let local_ids = fleet_local_ids(&registry)?;
    let path = resolve_registry_path(workspace_registry);
    let report = observe(path.as_deref(), max_age, &local_ids)?;
    match command {
        FleetCommands::List { .. } => {
            let human = render_fleet_report_human(&report);
            let json = serde_json::json!({
                "contract": FLEET_CONTRACT_VERSION,
                "fleet": report,
            });
            Ok(as_output(format, human, json))
        }
        FleetCommands::Status { .. } => {
            let human = render_status_human(&report);
            let json = serde_json::json!({
                "contract": FLEET_CONTRACT_VERSION,
                "health": health_json(&report),
            });
            Ok(as_output(format, human, json))
        }
        FleetCommands::Inspect { entry, .. } => {
            let found = inspect_entry(&report, entry)?;
            let human = render_entry_human(&report, found);
            let json = serde_json::json!({
                "contract": FLEET_CONTRACT_VERSION,
                "entry": found,
                "source": report.source,
                "freshness": report.freshness,
                "observed_at": report.observed_at,
            });
            Ok(as_output(format, human, json))
        }
        FleetCommands::Online {
            inventory,
            fleet_registry,
            workspace_root,
            domain,
            timeout_secs,
            dry_run,
        } => cmd_fleet_online(
            inventory.as_deref(),
            fleet_registry.as_deref(),
            workspace_root.as_deref(),
            domain.clone(),
            *timeout_secs,
            *dry_run,
            format,
        ),
    }
}

fn cmd_provider_matrix(db_path: &Path, live: bool, format: Format) -> Result<Output, ForgeError> {
    // The matrix is project-agnostic: it invents no project and the
    // journal row carries the synthetic id.
    let report = provider_matrix(live);
    let registry = open_registry(db_path)?;
    let _ = registry.record_operation(
        "provider",
        PROVIDER_SYNTHETIC_PROJECT,
        "done",
        &format!(
            "provider matrix: supported={} unavailable={} not-run={} disabled={} live={}",
            report.supported, report.unavailable, report.not_run, report.disabled, report.live
        ),
    );
    let json = serde_json::json!({
        "contract": PROVIDER_CONTRACT_VERSION,
        "matrix": report,
    });
    let human = render_provider_matrix_human(&report);
    Ok(as_output(format, human, json))
}

fn cmd_provider_run(
    db_path: &Path,
    provider: &str,
    target: Option<&str>,
    live: bool,
    fixture: Option<&Path>,
    project_ref: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let id = parse_evidence_provider(provider)?;
    let options = ProviderRunOptions {
        live,
        fixture: fixture.map(Path::to_path_buf),
        allow_unflagged_live: false,
    };
    // Resolve the attribution: a named target contributes its real
    // project id and directory; otherwise the probe runs in a
    // disposable dir under the synthetic project boundary.
    let (project_id, workdir, temp) = match target {
        Some(name) => {
            let (dir, pid) = resolve_analytics_target(db_path, name)?;
            (pid, dir, false)
        }
        None => {
            let mut dir = std::env::temp_dir();
            dir.push(format!(
                "forge-provider-run-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ));
            std::fs::create_dir_all(&dir).map_err(|err| ForgeError::ProviderInvalid {
                reason: format!("cannot create probe dir: {err}"),
            })?;
            ("evidence-probe".to_string(), dir, true)
        }
    };
    let row = run_provider_controlled(&id, &options, &project_id, &workdir, project_ref)?;
    if temp {
        let _ = std::fs::remove_dir_all(&workdir);
    }
    let journal_project = if target.is_some() {
        project_id.clone()
    } else {
        PROVIDER_SYNTHETIC_PROJECT.to_string()
    };
    let registry = open_registry(db_path)?;
    let _ = registry.record_operation(
        "provider",
        &journal_project,
        if row.status == "supported" {
            "done"
        } else {
            "rejected"
        },
        &format!(
            "provider run {id}: {} (sandbox {})",
            row.status,
            options_sandbox_label(&options)
        ),
    );
    let json = serde_json::json!({
        "contract": PROVIDER_CONTRACT_VERSION,
        "run": row,
    });
    let human = render_provider_row_human(&row);
    Ok(as_output(format, human, json))
}

fn options_sandbox_label(options: &ProviderRunOptions) -> &'static str {
    if options.fixture.is_some() {
        "fixture"
    } else if options.live {
        "live"
    } else {
        "none"
    }
}

fn cmd_provider_inspect(
    db_path: &Path,
    provider: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let descriptor = inspect_provider(provider)?;
    let registry = open_registry(db_path)?;
    let _ = registry.record_operation(
        "provider",
        PROVIDER_SYNTHETIC_PROJECT,
        "done",
        &format!("provider inspect {}", descriptor.provider),
    );
    let json = serde_json::json!({
        "contract": PROVIDER_CONTRACT_VERSION,
        "descriptor": descriptor,
    });
    let human = render_provider_descriptor_human(&descriptor);
    Ok(as_output(format, human, json))
}

fn cmd_api_serve(
    db_path: &Path,
    bind: Option<std::net::IpAddr>,
    port: Option<u16>,
    max_body_bytes: Option<usize>,
    format: Format,
) -> Result<Output, ForgeError> {
    let mut config = ApiConfig::from_env();
    if let Some(value) = bind {
        config.bind = value;
    }
    if let Some(value) = port {
        config.port = value;
    }
    if let Some(value) = max_body_bytes {
        config.max_body_bytes = value;
    }
    let shutdown = ShutdownSignal::new();
    let human = format!(
        "forge api serving on http://{addr} (contract {version}, synthetic project id `{synthetic}`); press Ctrl-C to stop",
        addr = config.socket_addr(),
        version = API_CONTRACT_VERSION,
        synthetic = API_SYNTHETIC_PROJECT,
    );
    let json = serde_json::json!({
        "contract": API_CONTRACT_VERSION,
        "bind": config.bind.to_string(),
        "port": config.port,
        "max_body_bytes": config.max_body_bytes,
        "synthetic_project": API_SYNTHETIC_PROJECT,
        "registry": db_path.display().to_string(),
    });
    if matches!(format, Format::Json) {
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("{human}");
    }
    let accepted = api_serve(&config, db_path, shutdown)?;
    let summary = format!("forge api stopped after {accepted} accepted connection(s)");
    let summary_json = serde_json::json!({
        "stopped": true,
        "accepted": accepted,
    });
    match format {
        Format::Human => Ok(Output::Human(summary)),
        Format::Json => Ok(Output::Json(summary_json)),
    }
}

fn resolve_analytics_target(db_path: &Path, target: &str) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let dir = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
        return Ok((dir, manifest.project.id));
    }
    let registry = open_registry(db_path)?;
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable { path: record.path });
    }
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
    Ok((dir, manifest.project.id))
}

fn cmd_analytics_inspect(
    db_path: &Path,
    target: &str,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (dir, project_id) = resolve_analytics_target(db_path, target)?;
    let _ = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
    let config = match load_analytics_config(&dir)? {
        Some(cfg) => cfg,
        None => {
            let detail = "inspect: project has no analytics block".to_string();
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation("analytics", &project_id, "rejected", &detail);
            }
            return Err(ForgeError::AnalyticsInvalid {
                reason: format!(
                    "project `{project_id}` has no `analytics:` block; declare one in \
                     forge.yaml to enable the external content / analytics plane"
                ),
            });
        }
    };
    let report =
        inspect_external_planes(&project_id, &config, &AnalyticsInspectOptions { dry_run })?;
    let detail = format!(
        "inspect: project={} enabled={} observations={} healthy={} dry_run={}",
        project_id,
        report.enabled,
        report.observations.len(),
        report.healthy(),
        dry_run,
    );
    if let Ok(registry) = open_registry(db_path) {
        let state = if report.healthy() { "done" } else { "rejected" };
        let _ = registry.record_operation("analytics", &project_id, state, &detail);
    }
    let json = serde_json::json!({
        "contract": ANALYTICS_CONTRACT_VERSION,
        "project_id": project_id,
        "report": report,
    });
    let human = render_analytics_report_human(&report);
    Ok(as_output(format, human, json))
}

fn cmd_analytics_metrics(
    db_path: &Path,
    target: &str,
    window_days: Option<u32>,
    all: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let project_id: String;
    let external_observations: Vec<forge::analytics::HealthObservation>;
    let default_window: u32;
    let summary_path_dir: Option<PathBuf>;
    if all {
        project_id = ANALYTICS_SYNTHETIC_PROJECT.to_string();
        external_observations = collect_external_observations_for_all(&registry, db_path)?;
        default_window = window_days.unwrap_or(forge::analytics::DEFAULT_WINDOW_DAYS);
        summary_path_dir = None;
    } else {
        let (dir, pid) = resolve_analytics_target(db_path, target)?;
        project_id = pid.clone();
        let config = load_analytics_config(&dir)?;
        let cfg = config.unwrap_or_else(|| AnalyticsConfig {
            enabled: false,
            default_window_days: forge::analytics::DEFAULT_WINDOW_DAYS,
            content: Vec::new(),
            repository: Vec::new(),
        });
        default_window = window_days.unwrap_or(cfg.default_window_days);
        if cfg.enabled && !cfg.all_providers().is_empty() {
            let report = inspect_external_planes(
                &project_id,
                &cfg,
                &AnalyticsInspectOptions { dry_run: false },
            )?;
            external_observations = report.observations;
        } else {
            external_observations = Vec::new();
        }
        summary_path_dir = Some(dir);
    }
    let doctor = DoctorSummary::default();
    let report = aggregate_project_metrics(
        &registry,
        Some(doctor),
        &MetricsAggregateOptions {
            default_window_days: default_window,
            external_observations,
        },
    )?;
    let summary = MetricsSummary {
        contract: report.contract.clone(),
        project_id: project_id.clone(),
        generated_at: report.generated_at.clone(),
        aggregates: report.aggregates.clone(),
    };
    if let Some(dir) = summary_path_dir {
        let path = metrics_summary_path(&dir, &project_id)?;
        let _ = save_metrics_summary(&path, &summary);
    }
    let detail = format!(
        "metrics: scope={} default_window={}d complete={} aggregates={}",
        if all { "all" } else { &project_id },
        default_window,
        report.complete,
        report.aggregates.len()
    );
    if let Ok(reg) = open_registry(db_path) {
        let journal_project = if all {
            ANALYTICS_SYNTHETIC_PROJECT
        } else {
            project_id.as_str()
        };
        let state = if report.complete { "done" } else { "partial" };
        let _ = reg.record_operation("analytics", journal_project, state, &detail);
    }
    let json = serde_json::json!({
        "contract": ANALYTICS_CONTRACT_VERSION,
        "metrics": report,
    });
    let human = render_metrics_human(&report);
    Ok(as_output(format, human, json))
}

fn collect_external_observations_for_all(
    registry: &Registry,
    db_path: &Path,
) -> Result<Vec<forge::analytics::HealthObservation>, ForgeError> {
    let mut out: Vec<forge::analytics::HealthObservation> = Vec::new();
    let projects = registry.list()?;
    for record in projects {
        let dir = PathBuf::from(&record.path);
        if !dir.is_dir() {
            continue;
        }
        let config = match load_analytics_config(&dir) {
            Ok(Some(cfg)) => cfg,
            _ => continue,
        };
        if !config.enabled {
            continue;
        }
        let report = inspect_external_planes(
            &record.id,
            &config,
            &AnalyticsInspectOptions { dry_run: false },
        )?;
        out.extend(report.observations);
    }
    let _ = db_path;
    Ok(out)
}

fn _ensure_analytics_symbols_used(provider: AnalyticsProvider) -> &'static str {
    forge::analytics::provider_label(provider)
}

/// Resolve a target string (registered id or filesystem path) to a
/// project directory and the manifest project id. Mirrors the upgrade
/// resolution contract so the CLI accepts both spellings.
fn resolve_identity_target(db_path: &Path, target: &str) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let dir = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
        return Ok((dir, manifest.project.id));
    }
    let registry = open_registry(db_path)?;
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable { path: record.path });
    }
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
    Ok((dir, manifest.project.id))
}

/// Resolve a gate target to a project directory and identity. A
/// registered project (by id or registered path) always wins so the
/// journal row names the real registry identity; an unregistered
/// directory falls back to the manifest id, the Workspace Governance
/// declaration id, or the directory name — gate is always
/// project-scoped, never a fleet operation.
fn resolve_gate_target(db_path: &Path, target: &str) -> Result<(PathBuf, String), ForgeError> {
    if let Ok(registry) = open_registry(db_path) {
        if let Ok(record) = registry.inspect(target) {
            let dir = PathBuf::from(&record.path);
            if !dir.is_dir() {
                return Err(ForgeError::PathUnavailable { path: record.path });
            }
            return Ok((dir, record.id));
        }
    }
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let dir = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        if let Ok((manifest, _)) = forge::core::manifest::Manifest::load_from_dir(&dir, None) {
            return Ok((dir, manifest.project.id));
        }
        if let Some(id) = gate::declared_project_id(&dir) {
            if forge::core::validate_project_id(&id).is_ok() {
                return Ok((dir, id));
            }
        }
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        return Ok((dir, name));
    }
    let registry = open_registry(db_path)?;
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable { path: record.path });
    }
    Ok((dir, record.id))
}

fn render_output(output: Output) {
    match output {
        Output::Human(text) => println!("{text}"),
        Output::Json(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
    }
}

fn gate_fail(err: &ForgeError, format: Format) -> ExitCode {
    render_error(err, format);
    ExitCode::from(1)
}

/// `forge gate [status] [TARGET] [--dry-run] [--timeout-secs N]`.
///
/// Exit codes mirror the sibling's blocking semantics: 0 only for a
/// fresh passing aggregate, 1 for blocked, failed, unknown, stale or
/// absent evidence, and an unavailable runtime. Evidence documents are
/// printed to stdout even when the verdict is non-zero (the fleet
/// precedent); only Forge-side failures use the typed stderr path.
fn cmd_gate(
    db_path: &Path,
    args: &[String],
    dry_run: bool,
    timeout_secs: Option<u64>,
    format: Format,
) -> ExitCode {
    // Parse the `status | evidence [status] | TARGET` positional forms.
    // Three mutually-exclusive actions:
    //   - `status [TARGET]` → read gate verdict evidence
    //   - `evidence [TARGET]` → run export and persist release evidence
    //   - `evidence status [TARGET]` → read persisted release evidence
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum GateCommand {
        Run,
        Status,
        EvidenceRun,
        EvidenceStatus,
    }
    let (command, target) = match args.split_first() {
        None => (GateCommand::Run, ".".to_string()),
        Some((first, rest)) if first == "status" => {
            if rest.len() > 1 {
                return gate_fail(
                    &ForgeError::GateInvalid {
                        reason: format!(
                            "`forge gate status` takes at most one TARGET; got {} extra arguments",
                            rest.len()
                        ),
                    },
                    format,
                );
            }
            (
                GateCommand::Status,
                rest.first().cloned().unwrap_or_else(|| ".".to_string()),
            )
        }
        Some((first, rest)) if first == "evidence" => {
            // `evidence [TARGET]` → run export and persist.
            // `evidence status [TARGET]` → read persisted release evidence.
            match rest.split_first() {
                Some((second, rest2)) if second == "status" => {
                    // `evidence status [TARGET]` — read without running.
                    if rest2.len() > 1 {
                        return gate_fail(
                            &ForgeError::GateInvalid {
                                reason: format!(
                                    "`forge gate evidence status` takes at most one TARGET; got {} extra arguments",
                                    rest2.len()
                                ),
                            },
                            format,
                        );
                    }
                    (
                        GateCommand::EvidenceStatus,
                        rest2.first().cloned().unwrap_or_else(|| ".".to_string()),
                    )
                }
                _ => {
                    // `evidence [TARGET]` — run export and persist.
                    if rest.len() > 1 {
                        return gate_fail(
                            &ForgeError::GateInvalid {
                                reason: format!(
                                    "`forge gate evidence [TARGET]` takes at most one TARGET; got {} extra arguments",
                                    rest.len()
                                ),
                            },
                            format,
                        );
                    }
                    (
                        GateCommand::EvidenceRun,
                        rest.first().cloned().unwrap_or_else(|| ".".to_string()),
                    )
                }
            }
        }
        Some((first, rest)) => {
            if !rest.is_empty() {
                return gate_fail(
                    &ForgeError::GateInvalid {
                        reason: format!(
                            "unexpected arguments {:?}; usage: `forge gate [TARGET]`, `forge gate status [TARGET]`, `forge gate evidence [TARGET]`, or `forge gate evidence status [TARGET]`",
                            rest
                        ),
                    },
                    format,
                );
            }
            (GateCommand::Run, first.clone())
        }
    };
    if matches!(command, GateCommand::Status) && dry_run {
        return gate_fail(
            &ForgeError::GateInvalid {
                reason:
                    "--dry-run does not apply to `forge gate status` (it never runs the runtime)"
                        .to_string(),
            },
            format,
        );
    }
    if matches!(command, GateCommand::Status) && timeout_secs.is_some() {
        return gate_fail(
            &ForgeError::GateInvalid {
                reason: "--timeout-secs does not apply to `forge gate status` (it never runs the runtime)".to_string(),
            },
            format,
        );
    }
    if matches!(command, GateCommand::EvidenceRun) && (dry_run || timeout_secs.is_some()) {
        return gate_fail(
            &ForgeError::GateInvalid {
                reason: "--dry-run and --timeout-secs do not apply to `forge gate evidence` (it consumes the sibling's export verb)"
                    .to_string(),
            },
            format,
        );
    }
    let config = match timeout_secs {
        Some(raw) => {
            let parsed = match gate::parse_timeout_secs(raw) {
                Ok(timeout) => timeout,
                Err(err) => return gate_fail(&err, format),
            };
            let mut config = GateConfig::from_env();
            config.timeout = parsed;
            config
        }
        None => GateConfig::from_env(),
    };
    let (dir, project_id) = match resolve_gate_target(db_path, &target) {
        Ok(resolved) => resolved,
        Err(err) => return gate_fail(&err, format),
    };
    match command {
        GateCommand::Status => cmd_gate_status(&dir, &project_id, format),
        GateCommand::EvidenceRun => cmd_gate_evidence(db_path, &dir, &project_id, &config, format),
        GateCommand::EvidenceStatus => cmd_gate_evidence_status(&dir, &project_id, format),
        GateCommand::Run => cmd_gate_run(db_path, &dir, &project_id, &config, dry_run, format),
    }
}

fn cmd_gate_status(dir: &Path, project_id: &str, format: Format) -> ExitCode {
    let evidence = match load_latest_evidence(dir) {
        Ok(Some(evidence)) => evidence,
        Ok(None) => {
            let json = serde_json::json!({
                "contract": GATE_CONTRACT_VERSION,
                "gate": {
                    "project": project_id,
                    "freshness": GateFreshness::Absent.label(),
                    "aggregate": "unverified",
                    "checks": [],
                },
            });
            let human = format!(
                "project: {project_id}\nfreshness: {}\naggregate: unverified\ndetail: the gate has never run; unverified is not a pass",
                GateFreshness::Absent.label()
            );
            render_output(as_output(format, human, json));
            return ExitCode::from(1);
        }
        Err(err) => return gate_fail(&err, format),
    };
    let current = gate::capture_revision(dir);
    let freshness = evidence_freshness(&evidence, current.as_deref());
    let passed =
        matches!(evidence.aggregate, GateAggregate::Passed) && freshness == GateFreshness::Fresh;
    // The present-evidence envelope flattens the record fields onto
    // `gate` (plus freshness) so every read surface reports the same
    // aggregate, revision, runtime name and timestamp bytes.
    let mut value = match serde_json::to_value(&evidence) {
        Ok(value) => value,
        Err(err) => {
            return gate_fail(
                &ForgeError::GateInvalid {
                    reason: format!("cannot encode gate evidence: {err}"),
                },
                format,
            )
        }
    };
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "freshness".to_string(),
            serde_json::json!(freshness.label()),
        );
    }
    let json = serde_json::json!({
        "contract": GATE_CONTRACT_VERSION,
        "gate": value,
    });
    let human = render_evidence_human(&evidence, Some(freshness), None);
    render_output(as_output(format, human, json));
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// `forge gate evidence [TARGET]` — run the sibling's export verb, validate,
/// and persist the attributed release-evidence record.
fn cmd_gate_evidence(
    db_path: &Path,
    dir: &Path,
    project_id: &str,
    config: &GateConfig,
    format: Format,
) -> ExitCode {
    let registry = match open_registry(db_path) {
        Ok(registry) => registry,
        Err(err) => return gate_fail(&err, format),
    };
    let outcome = gate::evidence::consume_export(dir, project_id, config);
    match outcome {
        gate::evidence::EvidenceOutcome::Record(record) => {
            let relative = match gate::evidence::save_release_evidence(dir, &record) {
                Ok(rel) => rel,
                Err(err) => return gate_fail(&err, format),
            };
            // Journal the consumption attempt.
            let verdict = if record.freshness == gate::evidence::EvidenceFreshness::Fresh {
                "done"
            } else {
                "done"
            };
            let _ = registry.record_operation(
                "gate",
                project_id,
                verdict,
                &format!(
                    "gate evidence consumed: freshness={} verified={} configured={} declared={} unverified={}",
                    record.freshness.label(),
                    record.counts.verified,
                    record.counts.configured,
                    record.counts.declared,
                    record.counts.unverified
                ),
            );
            let json = serde_json::json!({
                "contract": "release-evidence/0.1.0",
                "release_evidence": &record,
                "persisted": relative.display().to_string(),
            });
            let human = gate::evidence::render_release_evidence_human(
                &record,
                Some(relative.display().to_string().as_str()),
            );
            render_output(as_output(format, human, json));
            ExitCode::SUCCESS
        }
        gate::evidence::EvidenceOutcome::Absent { reason } => {
            let bound = gate::bound_note(&reason);
            let _ = registry.record_operation(
                "gate",
                project_id,
                "failed",
                &format!("gate evidence export unavailable: {bound}"),
            );
            gate_fail(
                &ForgeError::GateEvidenceUnavailable { reason: bound },
                format,
            )
        }
        gate::evidence::EvidenceOutcome::Refused {
            reason,
            refusal_reasons,
        } => {
            let bound = gate::bound_note(&reason);
            let _ = registry.record_operation(
                "gate",
                project_id,
                "failed",
                &format!("gate evidence export refused: {bound}"),
            );
            gate_fail(
                &ForgeError::GateEvidenceUnavailable {
                    reason: format!(
                        "{} ({} refusal reason(s): {})",
                        bound,
                        refusal_reasons.len(),
                        refusal_reasons.join("; ")
                    ),
                },
                format,
            )
        }
        gate::evidence::EvidenceOutcome::Unavailable { reason } => {
            let bound = gate::bound_note(&reason);
            let _ = registry.record_operation(
                "gate",
                project_id,
                "failed",
                &format!("gate evidence export unavailable: {bound}"),
            );
            gate_fail(
                &ForgeError::GateEvidenceUnavailable { reason: bound },
                format,
            )
        }
    }
}

/// `forge gate evidence status [TARGET]` — read persisted release-evidence
/// without running the export.
fn cmd_gate_evidence_status(dir: &Path, project_id: &str, format: Format) -> ExitCode {
    match gate::evidence::load_latest_release_evidence(dir) {
        Ok(Some(record)) => {
            let json = serde_json::json!({
                "contract": "release-evidence/0.1.0",
                "release_evidence": &record,
            });
            let human = gate::evidence::render_release_evidence_human(&record, None);
            render_output(as_output(format, human, json));
            ExitCode::SUCCESS
        }
        Ok(None) => {
            let reason =
                "no release-evidence record persisted; run `forge gate evidence` first".to_string();
            let json = serde_json::json!({
                "contract": "release-evidence/0.1.0",
                "release_evidence": null,
                "freshness": "absent",
                "detail": reason,
            });
            let human = gate::evidence::render_absent_human(project_id, &reason);
            render_output(as_output(format, human, json));
            ExitCode::from(1)
        }
        Err(err) => gate_fail(&err, format),
    }
}

fn cmd_gate_run(
    db_path: &Path,
    dir: &Path,
    project_id: &str,
    config: &GateConfig,
    dry_run: bool,
    format: Format,
) -> ExitCode {
    // The registry is required for journaling every attempted real run;
    // it is opened before the runtime is contacted so an unwritable
    // registry never leaves an unrecorded gate execution.
    let registry = match open_registry(db_path) {
        Ok(registry) => registry,
        Err(err) => return gate_fail(&err, format),
    };
    let outcome = gate::run_gate(dir, project_id, config, dry_run);
    let (output, verdict) = match outcome {
        GateOutcome::PlanPreview {
            runtime,
            runtime_version,
            plan,
        } => {
            let json = serde_json::json!({
                "contract": GATE_CONTRACT_VERSION,
                "dry_run": true,
                "gate": {
                    "project": project_id,
                    "runtime": runtime,
                    "runtime_version": runtime_version,
                    "plan": plan,
                    "persisted": false,
                    "journaled": false,
                },
            });
            let mut human = format!(
                "dry-run: gate plan preview for {project_id} (nothing was executed, persisted or journaled)\nruntime: {}",
                runtime_version
                    .as_deref()
                    .map(|version| format!("{runtime} ({version})"))
                    .unwrap_or_else(|| runtime.clone())
            );
            for line in &plan {
                human.push_str(&format!("\n  {line}"));
            }
            (as_output(format, human, json), ExitCode::SUCCESS)
        }
        GateOutcome::Evidence(evidence) => {
            let current = gate::capture_revision(dir);
            let freshness = evidence_freshness(&evidence, current.as_deref());
            let mut value = match serde_json::to_value(&evidence) {
                Ok(value) => value,
                Err(err) => {
                    return gate_fail(
                        &ForgeError::GateInvalid {
                            reason: format!("cannot encode gate evidence: {err}"),
                        },
                        format,
                    )
                }
            };
            let persisted: Option<String> = if evidence.dry_run {
                None
            } else {
                match gate::save_evidence(dir, &evidence) {
                    Ok(relative) => Some(relative.display().to_string()),
                    Err(err) => return gate_fail(&err, format),
                }
            };
            let verdict = gate::journal_verdict(&GateOutcome::Evidence(evidence.clone()));
            if !evidence.dry_run {
                let _ = registry.record_operation(
                    "gate",
                    project_id,
                    verdict,
                    &format!("gate {verdict}: {}", evidence_summary(&evidence)),
                );
            }
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "freshness".to_string(),
                    serde_json::json!(freshness.label()),
                );
            }
            let json = serde_json::json!({
                "contract": GATE_CONTRACT_VERSION,
                "dry_run": evidence.dry_run,
                "gate": value,
                "persisted": persisted,
                "journaled": !evidence.dry_run,
            });
            let human = if evidence.dry_run {
                format!(
                    "{}\ndry-run: the rehearsal document was not persisted and not journaled",
                    render_evidence_human(&evidence, Some(freshness), None)
                )
            } else {
                render_evidence_human(&evidence, Some(freshness), persisted.as_deref())
            };
            let code = if matches!(evidence.aggregate, GateAggregate::Passed) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            };
            (as_output(format, human, json), code)
        }
        GateOutcome::Unavailable { reason } => {
            let bound = gate::bound_note(&reason);
            // A refused rehearsal leaves no shadow: dry-runs never
            // journal (the deploy-rehearsal precedent). Real attempted
            // runs journal `failed` so the history survives the gap.
            if !dry_run {
                let _ = registry.record_operation(
                    "gate",
                    project_id,
                    "failed",
                    &format!("gate failed: runtime unavailable; {bound}"),
                );
            }
            return gate_fail(
                &ForgeError::GateRuntimeUnavailable { reason: bound },
                format,
            );
        }
    };
    render_output(output);
    verdict
}

fn cmd_contract(
    db_path: &Path,
    command: &ContractCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ContractCommands::List => {
            let manifest = forge::contract::load_manifest()?;
            let mut human = format!(
                "{:<18} {:<28} {:<10} {}",
                "Module", "Constant", "Version", "Family"
            );
            for spec in forge::contract::CONTRACTS {
                human.push_str(&format!(
                    "\n{:<18} {:<28} {:<10} {}",
                    spec.module,
                    spec.constant,
                    spec.version,
                    spec.platform_family.unwrap_or("-")
                ));
            }
            human.push_str(&format!(
                "\nsource: {}@{}",
                manifest.source,
                &manifest.revision[..12.min(manifest.revision.len())]
            ));
            let json = serde_json::json!({
                "contracts": forge::contract::CONTRACTS.iter().map(|c| serde_json::json!({
                    "module": c.module, "constant": c.constant, "discriminator": c.discriminator,
                    "version": c.version, "platform_family": c.platform_family, "doc": c.doc
                })).collect::<Vec<_>>(),
                "manifest": manifest,
            });
            Ok(as_output(format, human, json))
        }
        ContractCommands::Inspect { family } => {
            let manifest = forge::contract::load_manifest()?;
            let entry = manifest.files.iter().find(|f| {
                let fam_file = family.strip_prefix("platform.").unwrap_or(family);
                f.path.contains(fam_file)
            });
            let schema_path = forge::contract::family_schema_path(family).ok_or_else(|| {
                ForgeError::ContractInvalid {
                    reason: format!(
                        "unknown family '{family}'; supported: {}",
                        forge::contract::supported_families().join(", ")
                    ),
                }
            })?;
            let schema_text =
                std::fs::read_to_string(forge::contract::contracts_dir().join(schema_path))
                    .map_err(|e| ForgeError::ContractInvalid {
                        reason: format!("cannot read schema {schema_path}: {e}"),
                    })?;
            let schema: serde_json::Value =
                serde_json::from_str(&schema_text).map_err(|e| ForgeError::ContractInvalid {
                    reason: format!("invalid schema: {e}"),
                })?;
            let required = schema
                .get("required")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let human = format!(
                "family: {family}\ncontract: {}/0.1.0\nschema: {schema_path}\nrequired: {}\n{}",
                family,
                required.join(", "),
                schema_text.lines().take(20).collect::<Vec<_>>().join("\n")
            );
            let _ = entry;
            let json =
                serde_json::json!({"family": family, "schema": schema, "manifest": manifest});
            Ok(as_output(format, human, json))
        }
        ContractCommands::Emit { family, target } => {
            let doc = match family.as_str() {
                "platform.gate-result" => {
                    let (dir, pid) = resolve_gate_target(db_path, target)?;
                    forge::contract::emit_gate_result(&dir, &pid)?
                }
                "platform.readiness" => {
                    let docs = forge::contract::emit_readiness(Path::new(target), None)?;
                    if docs.is_empty() {
                        return Err(ForgeError::ContractInvalid {
                            reason: "no readiness data".to_string(),
                        });
                    }
                    docs.into_iter().next().unwrap()
                }
                "platform.release-evidence" => {
                    let dir = Path::new(target);
                    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(dir, None)
                        .map_err(|e| ForgeError::ContractInvalid {
                        reason: e.to_string(),
                    })?;
                    let pid = manifest.project.id.clone();
                    let releases =
                        forge::release::engine::list_releases(dir, &pid).map_err(|e| {
                            ForgeError::ContractInvalid {
                                reason: e.to_string(),
                            }
                        })?;
                    let latest = releases
                        .first()
                        .ok_or_else(|| ForgeError::ContractInvalid {
                            reason: "no release state for this project".to_string(),
                        })?;
                    forge::contract::emit_release_evidence(dir, &pid, &latest.release_id)?
                }
                "platform.capability" => {
                    let dir = Path::new(target);
                    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(dir, None)
                        .map_err(|e| ForgeError::ContractInvalid {
                        reason: e.to_string(),
                    })?;
                    let pid = manifest.project.id.clone();
                    forge::contract::emit_capability(dir, &pid)?.ok_or_else(|| {
                        ForgeError::ContractInvalid {
                            reason: "no capabilities block in .project.json".to_string(),
                        }
                    })?
                }
                "platform.audit-event" => {
                    let docs = forge::contract::emit_audit_events(db_path, 64)?;
                    if docs.is_empty() {
                        return Err(ForgeError::ContractInvalid {
                            reason: "no audit events to emit".to_string(),
                        });
                    }
                    docs.into_iter().next().unwrap()
                }
                "platform.job-outcome" => {
                    return Err(ForgeError::ContractInvalid {
                        reason: "platform.job-outcome has no source record in this release"
                            .to_string(),
                    });
                }
                _ => {
                    return Err(ForgeError::ContractInvalid {
                        reason: format!(
                            "unknown family '{family}'; supported: {}",
                            forge::contract::supported_families().join(", ")
                        ),
                    })
                }
            };
            forge::contract::validate_envelope(&doc)
                .map_err(|e| ForgeError::ContractInvalid { reason: e })?;
            let human = serde_json::to_string_pretty(&doc).unwrap_or_else(|_| doc.to_string());
            Ok(as_output(format, human, doc))
        }
        ContractCommands::Validate { file } => {
            let text = if file == "-" {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf).map_err(|e| {
                    ForgeError::ContractInvalid {
                        reason: e.to_string(),
                    }
                })?;
                buf
            } else {
                std::fs::read_to_string(file).map_err(|e| ForgeError::ContractInvalid {
                    reason: format!("cannot read {file}: {e}"),
                })?
            };
            let doc: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| ForgeError::ContractInvalid {
                    reason: format!("invalid JSON: {e}"),
                })?;
            forge::contract::validate_envelope(&doc)
                .map_err(|e| ForgeError::ContractInvalid { reason: e })?;
            let human = format!(
                "valid: {}",
                doc.get("contract")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            );
            let json = serde_json::json!({"valid": true, "contract": doc.get("contract")});
            Ok(as_output(format, human, json))
        }
    }
}

fn cmd_inventory(command: &InventoryCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        InventoryCommands::Show { source, domain } => {
            use forge::publish::inventory::{classify, resolve_source};
            let path = source
                .clone()
                .or_else(|| resolve_source(None))
                .ok_or_else(|| ForgeError::PublishInvalid {
                    reason: "inventory source is not configured: pass a positional path or set \
                         $FORGE_INVENTORY_SOURCE"
                        .to_string(),
                })?;
            let snapshot = load_inventory_snapshot(&path)?;
            let report = classify(&snapshot, domain);
            let mut human = format!(
                "inventory {contract}: {provider} at {source}, declared={declared}, \
                 compose_ready={ready}, compose_missing={missing}, \
                 source_unavailable={unavailable}, invalid={invalid}\n",
                contract = report.contract,
                provider = report.provider,
                source = source_label(&snapshot),
                declared = report.entries.len(),
                ready = report
                    .entries
                    .iter()
                    .filter(|e| matches!(
                        e.classification,
                        forge::publish::inventory::InventoryClassification::ComposeReady
                    ))
                    .count(),
                missing = report
                    .entries
                    .iter()
                    .filter(|e| matches!(
                        e.classification,
                        forge::publish::inventory::InventoryClassification::ComposeMissing
                    ))
                    .count(),
                unavailable = report
                    .entries
                    .iter()
                    .filter(|e| matches!(
                        e.classification,
                        forge::publish::inventory::InventoryClassification::SourceUnavailable
                    ))
                    .count(),
                invalid = report
                    .entries
                    .iter()
                    .filter(|e| matches!(
                        e.classification,
                        forge::publish::inventory::InventoryClassification::Invalid
                    ))
                    .count(),
            );
            for entry in &report.entries {
                human.push_str(&format!(
                    "  {id:<20} {runtime:<8} {classification:<18} subdomain={subdomain}\n",
                    id = entry.id,
                    runtime = entry.runtime.as_str(),
                    classification = entry.classification.as_str(),
                    subdomain = entry.subdomain.as_deref().unwrap_or("-"),
                ));
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::PublishInvalid {
                reason: format!("cannot encode inventory report: {err}"),
            })?;
            Ok(as_output(format, human, json))
        }
    }
}

fn source_label(snapshot: &forge::publish::inventory::InventorySnapshot) -> String {
    snapshot
        .source
        .clone()
        .unwrap_or_else(|| "<unspecified>".to_string())
}

/// Render the `forge fleet online` plan. Every value the operator
/// would see at runtime is rendered as plain text, including the
/// SSH argv and the curl argv. `--dry-run` is the rehearsal: nothing
/// is contacted and nothing is journaled; the plan preview is the
/// entire output. The per-host list deliberately comes from the
/// served Caddyfile, which is only readable at runtime — a dry-run
/// that printed a synthetic host list would mislead the operator
/// when the served file disagrees with the inventory.
fn render_fleet_online_plan(
    inventory_source: &str,
    target: &str,
    domain: &str,
    timeout_secs: u64,
    _entries: &[forge::fleet::online::LivenessEntry],
    caddyfile_path: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "fleet online: dry-run, inventory={inventory_source}, target={target}, domain={domain}, \
         timeout={timeout_secs}s\n"
    ));
    out.push_str(&format!(
        "  probe plan: 1) ssh {target} cat {caddyfile_path}\n"
    ));
    out.push_str(&format!(
        "  probe plan: 2) ssh {target} env PATH=<docker-desktop> /usr/local/bin/docker ps --no-trunc --format ...\n"
    ));
    out.push_str(&format!(
        "  probe plan: 3) per compose-ready entry with a served route, one bounded HTTPS GET (curl -s -m {timeout_secs} --max-filesize 65536 https://<host>); per-host list is resolved at runtime from the served Caddyfile, not printed here\n"
    ));
    out
}

/// Probe the served router rules and target containers to verdict
/// whether every routed host is online. Mirrors `cmd_publish_fleet`'s
/// inventory resolution exactly: explicit `--inventory` first, then
/// the legacy workspace-governance registry compatibility adapter,
/// then the inventory classification. Only `compose_ready` entries
/// are probed; the others are reported with their classification,
/// never silently omitted. The whole surface is read-only: no
/// journal rows, no registry writes, no target writes, and every
/// captured string passes through `policy::redact_credentials`.
#[allow(clippy::too_many_arguments)]
fn cmd_fleet_online(
    inventory: Option<&std::path::Path>,
    fleet_registry: Option<&std::path::Path>,
    workspace_root: Option<&std::path::Path>,
    domain: String,
    timeout_secs: u64,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::fleet::online::{
        caddyfile_path_from_env, cat_caddyfile_command, classify, docker_ps_command,
        match_container, parse_caddyfile_hosts, probe_http, route_for_project, ssh_target_from_env,
        ProbeFacts, DEFAULT_HTTP_TIMEOUT_SECS, LIVENESS_CONTRACT_VERSION, MAX_HTTP_TIMEOUT_SECS,
        SSH_TARGET_ENV,
    };
    use forge::publish::fleet::{default_registry_path, default_workspace_root};
    use forge::publish::inventory::{
        classify as classify_inventory, resolve_source, DEFAULT_DOMAIN,
    };

    if !(1..=MAX_HTTP_TIMEOUT_SECS).contains(&timeout_secs) {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "timeout {timeout_secs}s is out of bounds; expected 1..={MAX_HTTP_TIMEOUT_SECS}"
            ),
        });
    }
    let _ = DEFAULT_HTTP_TIMEOUT_SECS;
    let _ = SSH_TARGET_ENV;

    // Resolve the inventory exactly the way `cmd_publish_fleet` does:
    // explicit `--inventory` first, then the legacy registry
    // compatibility adapter. The seven-project handoff keeps working
    // without a sibling checkout during migration.
    let lifecycle = "active".to_string();
    let (snapshot, source_label) = if let Some(path) = inventory
        .map(|p| p.to_path_buf())
        .or_else(|| resolve_source(None))
        .or_else(|| fleet_registry.map(|p| p.to_path_buf()))
    {
        let snapshot = load_inventory_snapshot(&path)?;
        (snapshot, format!("inventory:{}", path.display()))
    } else {
        let workspace_root = workspace_root
            .map(|p| p.to_path_buf())
            .unwrap_or_else(default_workspace_root);
        let registry_path = fleet_registry
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| default_registry_path(Some(&workspace_root)));
        let snapshot = legacy_inventory_snapshot(&registry_path, &workspace_root, &lifecycle)?;
        (snapshot, format!("registry:{}", registry_path.display()))
    };

    let fleet_report = classify_inventory(&snapshot, DEFAULT_DOMAIN);
    let target = ssh_target_from_env();
    let caddyfile_path = caddyfile_path_from_env();

    // Dry-run: render the plan, return success, journal nothing.
    if dry_run {
        let entries: Vec<forge::fleet::online::LivenessEntry> = fleet_report
            .entries
            .iter()
            .map(|entry| forge::fleet::online::LivenessEntry {
                id: entry.id.clone(),
                classification: entry.classification.as_str().to_string(),
                container: None,
                route: None,
                http_status: None,
                verdict: forge::fleet::online::Verdict::Unavailable,
                detail: None,
            })
            .collect();
        let human = render_fleet_online_plan(
            &source_label,
            &target,
            &domain,
            timeout_secs,
            &entries,
            &caddyfile_path,
        );
        let json = serde_json::json!({
            "contract": LIVENESS_CONTRACT_VERSION,
            "dry_run": true,
            "inventory_source": source_label,
            "target": target,
            "domain": domain,
            "timeout_secs": timeout_secs,
            "caddyfile_path": caddyfile_path,
        });
        return Ok(as_output(format, human, json));
    }

    // Real probe: read the served Caddyfile once via SSH (or a
    // local test stub named in `FORGE_PUBLISH_SSH_TARGET`).
    let caddyfile_argv = cat_caddyfile_command(&target, &caddyfile_path);
    let caddyfile_argv_str: Vec<String> = caddyfile_argv
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let mut ssh_command = std::process::Command::new(&caddyfile_argv[0]);
    for arg in caddyfile_argv.iter().skip(1) {
        ssh_command.arg(arg);
    }
    ssh_command.env("LC_ALL", "C");
    let caddyfile_output = match ssh_command.output() {
        Ok(out) => out,
        Err(err) => {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "cannot spawn `{}` to read served Caddyfile: {err}; \
                     the binary must be present on the controller",
                    caddyfile_argv_str
                        .first()
                        .map(|s| s.as_str())
                        .unwrap_or("<ssh>")
                ),
            });
        }
    };
    let caddyfile_text = String::from_utf8_lossy(&caddyfile_output.stdout).into_owned();
    let caddyfile_stderr = String::from_utf8_lossy(&caddyfile_output.stderr).into_owned();
    if !caddyfile_output.status.success() && caddyfile_text.trim().is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "probe `{}` failed: {}",
                caddyfile_argv_str.join(" "),
                forge::policy::redact_credentials(caddyfile_stderr.trim())
            ),
        });
    }
    let nav_host = std::env::var("FORGE_PUBLISH_NAV_HOST")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "apps".to_string());
    let routes = parse_caddyfile_hosts(&caddyfile_text, &domain, &nav_host).map_err(|err| {
        ForgeError::PublishInvalid {
            reason: format!("served Caddyfile `{caddyfile_path}` is malformed: {err}"),
        }
    })?;

    // Read container state from the target.
    let docker_argv = docker_ps_command(&target);
    let docker_argv_str: Vec<String> = docker_argv
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let mut docker_command = std::process::Command::new(&docker_argv[0]);
    for arg in docker_argv.iter().skip(1) {
        docker_command.arg(arg);
    }
    docker_command.env("LC_ALL", "C");
    let docker_output = match docker_command.output() {
        Ok(out) => out,
        Err(err) => {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "cannot spawn `{}` to read target `docker ps`: {err}; \
                     the binary must be present on the controller",
                    docker_argv_str
                        .first()
                        .map(|s| s.as_str())
                        .unwrap_or("<ssh>")
                ),
            });
        }
    };
    let docker_stdout = String::from_utf8_lossy(&docker_output.stdout).into_owned();
    let docker_stderr = String::from_utf8_lossy(&docker_output.stderr).into_owned();
    if !docker_output.status.success() && docker_stdout.trim().is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "probe `{}` failed: {}",
                docker_argv_str.join(" "),
                forge::policy::redact_credentials(docker_stderr.trim())
            ),
        });
    }

    let mut assembled: Vec<(String, String, ProbeFacts, Option<String>)> =
        Vec::with_capacity(fleet_report.entries.len());
    for entry in &fleet_report.entries {
        let id = entry.id.clone();
        let classification = entry.classification.as_str().to_string();
        if classification != "compose_ready" {
            // Non-ready entries are reported with their classification
            // and never probed. No container / route / http probe.
            assembled.push((
                id,
                classification,
                ProbeFacts {
                    container: None,
                    route: None,
                    http_status: None,
                    body: None,
                    probe_error: None,
                },
                None,
            ));
            continue;
        }
        // Compose-ready: build the public URL the served router
        // claims, then verify the container is up.
        let route = route_for_project(&routes, &id, &domain);
        let container = match_container(&docker_stdout, &id);
        match route {
            None => assembled.push((
                id,
                classification,
                ProbeFacts {
                    container,
                    route: None,
                    http_status: None,
                    body: None,
                    probe_error: None,
                },
                None,
            )),
            Some(host) => {
                let url = format!("https://{host}");
                let probe = probe_http(&url, timeout_secs);
                match probe {
                    Ok((status, body)) => assembled.push((
                        id,
                        classification,
                        ProbeFacts {
                            container,
                            route: Some(host.clone()),
                            http_status: Some(status),
                            body: Some(body),
                            probe_error: None,
                        },
                        Some(url),
                    )),
                    Err(err) => assembled.push((
                        id,
                        classification,
                        ProbeFacts {
                            container,
                            route: Some(host.clone()),
                            http_status: None,
                            body: None,
                            probe_error: Some(err.to_string()),
                        },
                        Some(url),
                    )),
                }
            }
        }
    }

    // Run every entry through the classifier (one place so the
    // verdict matrix has exactly one source of truth).
    let entries: Vec<(String, String, ProbeFacts)> = assembled
        .into_iter()
        .map(|(id, classification, facts, _url)| (id, classification, facts))
        .collect();
    let probe_results: Vec<(String, String, ProbeFacts)> = entries.clone();
    let mut entries_for_report: Vec<(String, String, ProbeFacts)> =
        Vec::with_capacity(entries.len());
    let mut probe_summary: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (id, classification, facts) in probe_results {
        let (verdict, _status, _detail) = classify(facts.clone());
        *probe_summary.entry(verdict.as_str()).or_insert(0) += 1;
        entries_for_report.push((id, classification, facts));
    }
    let _ = probe_summary;

    let now = forge::publish::inventory::now_rfc3339();
    let report = forge::fleet::online::build_report(
        entries_for_report,
        source_label.clone(),
        target.clone(),
        domain.clone(),
        now.clone(),
    );

    let mut human = String::new();
    human.push_str(&format!(
        "fleet online ({contract}): {source}, target={target}, domain={domain}\n",
        contract = report.contract,
        source = source_label,
    ));
    human.push_str(&format!(
        "  summary: online={online} down={down} no_route={no_route} \
         not_deployed={not_deployed} unavailable={unavailable} skipped={skipped}\n",
        online = report.summary.online,
        down = report.summary.down,
        no_route = report.summary.no_route,
        not_deployed = report.summary.not_deployed,
        unavailable = report.summary.unavailable,
        skipped = report.summary.skipped,
    ));
    for entry in &report.entries {
        let container = entry.container.as_deref().unwrap_or("-");
        let route = entry.route.as_deref().unwrap_or("-");
        let status = entry
            .http_status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "-".to_string());
        let detail = entry.detail.as_deref().unwrap_or("");
        human.push_str(&format!(
            "  {id:<24} {verdict:<13} classification={classification:<14} \
             container={container:<24} route={route:<32} http={status:<4} {detail}\n",
            id = entry.id,
            verdict = entry.verdict.as_str(),
            classification = entry.classification,
            container = container,
            route = route,
            status = status,
            detail = if detail.is_empty() { "" } else { detail },
        ));
    }
    let json = serde_json::to_value(&report).map_err(|err| ForgeError::PublishInvalid {
        reason: format!("cannot encode fleet liveness report: {err}"),
    })?;
    let output = as_output(format, human, json);

    if !report.summary.all_probed_online() {
        // Echo the report so operators get the same content in
        // success and failure paths.
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
        }
        return Err(ForgeError::PublishDeployFailed {
            reason: format!(
                "fleet online: {} of {} probed host(s) are not ONLINE ({} down, {} no_route, {} not_deployed, {} unavailable)",
                report.summary.down
                    + report.summary.no_route
                    + report.summary.not_deployed
                    + report.summary.unavailable,
                report.summary.online
                    + report.summary.down
                    + report.summary.no_route
                    + report.summary.not_deployed
                    + report.summary.unavailable,
                report.summary.down,
                report.summary.no_route,
                report.summary.not_deployed,
                report.summary.unavailable,
            ),
        });
    }
    Ok(output)
}
