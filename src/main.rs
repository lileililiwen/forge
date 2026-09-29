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
use forge::catalog::{
    self, CatalogQuery, CatalogRequest, CatalogSourceSelection, CATALOG_CONTRACT_VERSION,
    DEFAULT_LIMIT as CATALOG_DEFAULT_LIMIT,
    DEFAULT_MAX_AGE_SECONDS as CATALOG_DEFAULT_MAX_AGE_SECONDS,
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
use forge::graduation::{
    adopt_graduation, build_proposal, render_preview_human, GraduationImport, GraduationPreview,
    GraduationRefusal, GRADUATION_CONTRACT_VERSION,
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
use forge::remediation::{self, RemediationPlan};
use forge::semantic::{self, ProposalEvidence, ProposalKind, SuggestRequest};
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
use std::process::{Command, ExitCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Human-facing text.
    Human,
    /// The same human-facing layout, named explicitly by the project
    /// catalog so `forge project list --format table` reads as a layout
    /// choice rather than a machine contract.
    Table,
    /// The versioned JSON document.
    Json,
    /// Newline-delimited JSON: one catalog record per line on the
    /// project query surface, one document line elsewhere.
    Ndjson,
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
    /// Validate a local `platform.idea-graduation` artifact and, on
    /// explicit confirmation, create a project from its brief.
    Graduation {
        #[command(subcommand)]
        command: GraduationCommands,
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
        /// Render a versioned standard-pack snapshot into `.standard/`
        /// (e.g. `baseline-service@1.1.0`). Without this flag the output
        /// is byte-identical to pre-standard releases: no pack is
        /// selected implicitly.
        #[arg(long, value_name = "PACK@VERSION")]
        standard_pack: Option<String>,
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
    /// Manage user-owned portfolio metadata and import source-owned evidence snapshots.
    Portfolio {
        #[command(subcommand)]
        command: PortfolioCommands,
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
    /// Query the normalized read-only project catalog
    /// (`forge-project-catalog/0.1.0`).
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
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
    /// List, inspect, check, diff and upgrade versioned standard-pack snapshots.
    Standard {
        #[command(subcommand)]
        command: StandardCommands,
    },
    /// Plan and apply ownership-safe local remediation.
    Remediate {
        #[command(subcommand)]
        command: RemediateCommands,
    },
    /// Suggest, list, show, approve and reject semantic project
    /// descriptions (`forge-semantic-proposal/0.1.0`).
    Describe {
        #[command(subcommand)]
        command: DescribeCommands,
    },
    /// Suggest, list, show, approve and reject semantic project
    /// classifications (domain, portfolio tags, profile, lifecycle).
    Classify {
        #[command(subcommand)]
        command: ClassifyCommands,
    },
    /// Coordinate the staged delivery workflow (preflight → stage →
    /// production) and the optional post-deploy Hermora onboarding.
    Delivery {
        #[command(subcommand)]
        command: DeliveryCommands,
    },
}

#[derive(Debug, Subcommand)]
enum DeliveryCommands {
    /// Read the project's current delivery phase, revision, and
    /// most-recent per-verb journal evidence.
    Status {
        /// Registered project id.
        project: String,
    },
    /// Invoke the publish provider's `preflight` operation and
    /// record the terminal evidence.
    Preflight {
        /// Registered project id.
        project: String,
    },
    /// Invoke the publish provider's `publish` operation for the
    /// stage environment. Requires `--confirm-operation-id` from a
    /// healthy preflight row.
    Stage {
        /// Registered project id.
        project: String,
        /// Operation id from a terminal `delivery.preflight` row
        /// for the same revision.
        #[arg(long = "confirm-operation-id")]
        confirm_operation_id: Option<i64>,
    },
    /// Invoke the publish provider's `publish` operation for the
    /// production environment. Requires `--confirm-revision` matching
    /// the project's registered source revision.
    Promote {
        /// Registered project id.
        project: String,
        /// 40-character source revision SHA the operator is
        /// confirming for production promotion.
        #[arg(long = "confirm-revision")]
        confirm_revision: Option<String>,
    },
    /// Retry the optional Hermora site onboarding for a healthy
    /// deployment. Independent child operation; never republishes.
    HermoraRetry {
        /// Registered project id.
        project: String,
        /// Public deployment URL to enroll (http/https).
        #[arg(long = "deployment-url")]
        deployment_url: Option<String>,
        /// Environment-variable reference (e.g. `env:HERMORA_TOKEN_*`).
        /// Only the variable name crosses Forge; the value never does.
        #[arg(long = "secret-ref")]
        secret_ref: Option<String>,
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
        /// Maximum concurrent per-project publishes (1..=32; default
        /// 4). Workers run publishes concurrently; journal rows and
        /// output render serially on the main thread. `--jobs 1`
        /// preserves sequential behavior. The external `--provider`
        /// branch always stays sequential (`fleet-live-rollout`).
        #[arg(long, default_value_t = 4)]
        jobs: usize,
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
enum PortfolioCommands {
    /// Manage user-owned tags.
    Tag {
        #[command(subcommand)]
        command: PortfolioTagCommands,
    },
    /// Manage user-owned relationships between projects.
    Relation {
        #[command(subcommand)]
        command: PortfolioRelationCommands,
    },
    /// Record a review decision and the current classification.
    Review {
        #[command(subcommand)]
        command: PortfolioReviewCommands,
    },
    /// Manage user-owned portfolio goals.
    Goal {
        #[command(subcommand)]
        command: PortfolioGoalCommands,
    },
    /// Import source-owned observations as append-only snapshots.
    Evidence {
        #[command(subcommand)]
        command: PortfolioEvidenceCommands,
    },
    /// Show one project's whole portfolio projection.
    Show {
        /// Registered project id.
        project: String,
    },
    /// Manage the explicit allowlist of projects that may be published.
    Share {
        #[command(subcommand)]
        command: PortfolioShareCommands,
    },
    /// Import and compare aggregate, privacy-safe interest evidence.
    Interest {
        #[command(subcommand)]
        command: PortfolioInterestCommands,
    },
    /// Read-only readiness verdict on whether aggregate evidence
    /// justifies the product-owned activation follow-up. No billing.
    Activation {
        #[command(subcommand)]
        command: PortfolioActivationCommands,
    },
}

#[derive(Debug, Subcommand)]
enum GraduationCommands {
    /// Validate and show an artifact without choosing a destination.
    /// Read-only: prints the mapped brief, provenance and evidence.
    Preview {
        /// Local artifact path, or `-` for stdin.
        artifact: String,
    },
    /// Validate an artifact, resolve the project identity and, with
    /// `--confirm`, create the project. Without `--confirm` it prints
    /// the preview and writes nothing.
    Import {
        /// Local artifact path, or `-` for stdin.
        artifact: String,
        /// Destination project directory.
        #[arg(long)]
        path: PathBuf,
        /// Explicit profile id for the new project.
        #[arg(long)]
        profile: String,
        /// Explicit project id (default: kebab-cased brief title).
        #[arg(long)]
        id: Option<String>,
        /// Who is importing. Recorded in the receipt, bounded and scrubbed.
        #[arg(long, default_value = "local-admin")]
        actor: String,
        /// Perform the write; without it the command is a dry run.
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Debug, Subcommand)]
enum PortfolioInterestCommands {
    /// Import a versioned batch of aggregate snapshots from a JSON
    /// file, or from stdin when the path is `-`. Every record is
    /// reported separately; a refused record never blocks the others.
    Import {
        /// Path to the import document, or `-` for stdin.
        file: String,
        /// Who is importing. Recorded verbatim as provenance.
        #[arg(long, default_value = "local-admin")]
        actor: String,
    },
    /// List one project's stored snapshots, or every project's.
    List {
        /// Registered project id (omit for every project).
        #[arg(default_value = "")]
        project: String,
        /// Maximum rows to read.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Days after which a window reads as `stale`.
        #[arg(long, default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS)]
        stale_after_days: i64,
    },
    /// Show one project's whole interest projection with its refusals.
    Show {
        /// Registered project id.
        project: String,
        /// Days after which a window reads as `stale`.
        #[arg(long, default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS)]
        stale_after_days: i64,
    },
    /// Compare projects on the allowlisted metrics. Forge never totals
    /// or ranks across windows and labels every row with its source and
    /// freshness.
    Compare {
        /// Registered project ids to compare.
        #[arg(required = true)]
        projects: Vec<String>,
        /// Metric to compare; repeat for several (default: all).
        #[arg(long = "metric")]
        metrics: Vec<String>,
        /// Narrow the comparison to one analytics source.
        #[arg(long)]
        source: Option<String>,
        /// Days after which a window reads as `stale`.
        #[arg(long, default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS)]
        stale_after_days: i64,
    },
    /// One metric's windowed history for one project.
    Trend {
        /// Registered project id.
        project: String,
        /// Metric to plot.
        #[arg(long)]
        metric: String,
        /// Maximum windows to return.
        #[arg(long, default_value_t = 12)]
        limit: usize,
        /// Days after which a window reads as `stale`.
        #[arg(long, default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS)]
        stale_after_days: i64,
    },
    /// The refusals this store recorded, newest first.
    Audit {
        /// Maximum rows to read.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
enum PortfolioActivationCommands {
    /// Read-only verdict on whether a project's aggregate interest
    /// evidence justifies the product-owned activation follow-up.
    /// Prints the report and exits non-zero when any evaluated
    /// project is `not-ready`. Adds no billing of any kind.
    Readiness {
        /// Registered project id (omit for every registered project).
        #[arg(default_value = "")]
        project: String,
        /// Aggregate metric the verdict rests on.
        #[arg(long = "metric")]
        metric: String,
        /// Minimum value the operator declares; absent is the
        /// `threshold-not-declared` verdict, not an error.
        #[arg(long = "min-value")]
        min_value: Option<u64>,
        /// Narrow the verdict to one analytics source.
        #[arg(long)]
        source: Option<String>,
        /// Narrow the verdict to one window `<START>..<END>`.
        #[arg(long)]
        window: Option<String>,
        /// Days after which a window reads as `stale`.
        #[arg(long, default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS)]
        stale_after_days: i64,
    },
}

#[derive(Debug, Subcommand)]
enum PortfolioTagCommands {
    /// Attach a tag to a project, creating the tag on first use.
    Add {
        /// Registered project id.
        project: String,
        /// Tag name (lowercase, digits and single dashes).
        #[arg(long)]
        name: String,
        /// Optional `#rrggbb` colour or lowercase word.
        #[arg(long)]
        color: Option<String>,
    },
    /// Detach a tag from a project. The tag itself is kept.
    Remove {
        /// Registered project id.
        project: String,
        /// Tag name to detach.
        #[arg(long)]
        name: String,
    },
    /// List every tag, or the tags on one project.
    List {
        /// Registered project id (omit for every tag).
        #[arg(default_value = "")]
        project: String,
    },
}

#[derive(Debug, Subcommand)]
enum PortfolioRelationCommands {
    /// Link two projects. The same link twice stays one row.
    Add {
        /// Source project id.
        from: String,
        /// Target project id.
        #[arg(long)]
        to: String,
        /// Relation type.
        #[arg(long)]
        r#type: String,
        /// Optional note.
        #[arg(long)]
        note: Option<String>,
    },
    /// Remove one declared relation.
    Remove {
        /// Source project id.
        from: String,
        /// Target project id.
        #[arg(long)]
        to: String,
        /// Relation type.
        #[arg(long)]
        r#type: String,
    },
    /// List relations touching one project, or every relation.
    List {
        /// Registered project id (omit for every relation).
        #[arg(default_value = "")]
        project: String,
    },
}

#[derive(Debug, Subcommand)]
enum PortfolioReviewCommands {
    /// Record a review decision and optionally the current lifecycle,
    /// next action and blocker for one project.
    Set {
        /// Registered project id.
        project: String,
        /// Review confidence.
        #[arg(long)]
        confidence: String,
        /// Review note.
        #[arg(long)]
        note: Option<String>,
        /// Current lifecycle classification.
        #[arg(long)]
        lifecycle: Option<String>,
        /// Next action the operator intends to take.
        #[arg(long)]
        next_action: Option<String>,
        /// Current blocker.
        #[arg(long)]
        blocker: Option<String>,
    },
    /// List the review history for one project.
    List {
        /// Registered project id.
        project: String,
    },
}

#[derive(Debug, Subcommand)]
enum PortfolioGoalCommands {
    /// Create a goal. Re-running with the same title updates it.
    Add {
        /// Goal title (unique).
        title: String,
        /// Goal status.
        #[arg(long, default_value = "planned")]
        status: String,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Attach a project to a goal, creating the goal when new.
    Link {
        /// Goal title.
        goal: String,
        /// Registered project id.
        #[arg(long)]
        project: String,
    },
    /// List every goal with its projects.
    List,
}

#[derive(Debug, Subcommand)]
enum PortfolioEvidenceCommands {
    /// Append one source-owned observation as a snapshot. The payload is
    /// redacted before it is stored and is never rewritten afterwards.
    Import {
        /// Registered project id.
        #[arg(long)]
        project: String,
        /// Source system that produced the observation.
        #[arg(long)]
        source: String,
        /// Revision the source observed.
        #[arg(long)]
        revision: String,
        /// Status: observed, stale, unavailable, invalid or not-run.
        #[arg(long)]
        status: String,
        /// RFC 3339 observation time (default: now).
        #[arg(long)]
        observed_at: Option<String>,
        /// RFC 3339 freshness bound; past this the read model shows stale.
        #[arg(long)]
        stale_after: Option<String>,
        /// Evidence document as JSON, or `@path` to read it from a file.
        #[arg(long, default_value = "{}")]
        evidence: String,
    },
    /// List every snapshot for one project, newest first.
    List {
        /// Registered project id.
        project: String,
    },
}

#[derive(Debug, Subcommand)]
enum PortfolioShareCommands {
    /// Define or replace one project's public share record. Nothing is
    /// public until the manifest is approved and published.
    Set {
        /// Registered project id.
        project: String,
        /// Public title.
        #[arg(long)]
        title: String,
        /// Public one-paragraph summary.
        #[arg(long)]
        summary: String,
        /// Public category label.
        #[arg(long)]
        category: String,
        /// Public HTTPS source URL.
        #[arg(long)]
        source_url: String,
        /// Optional public HTTPS demo URL.
        #[arg(long)]
        demo_url: Option<String>,
        /// Visibility: public or unlisted.
        #[arg(long, default_value = "public")]
        visibility: String,
        /// Mark the record as featured.
        #[arg(long)]
        featured: bool,
        /// Showcase status: planned, demo, beta, stable, archived or unknown.
        #[arg(long, default_value = "unknown")]
        status: String,
        /// Optional status-evidence JSON object, or `@path` to read it from a file.
        #[arg(long)]
        evidence: Option<String>,
        /// Allowlisted public surface as `label=https://…`; repeat for more.
        #[arg(long = "surface", value_name = "LABEL=URL")]
        surfaces: Vec<String>,
    },
    /// Withdraw a project's share record; it leaves the public catalog.
    Remove {
        /// Registered project id.
        project: String,
    },
    /// Show one project's share record and its allowlisted surfaces.
    Show {
        /// Registered project id.
        project: String,
    },
    /// List every share record, newest state first.
    List {
        /// Registered project id (omit for every record).
        #[arg(default_value = "")]
        project: String,
    },
    /// Preview the candidate manifest: exact canonical bytes, hash and findings.
    Preview {
        /// Also print the rendered public document.
        #[arg(long)]
        document: bool,
    },
    /// Approve one exact manifest hash.
    Approve {
        /// The `manifest_sha256` a preview reported.
        #[arg(long)]
        hash: String,
        /// Who is approving; recorded in the publication audit trail.
        #[arg(long, default_value = "local-admin")]
        actor: String,
    },
    /// Publish the approved manifest through the default-safe local export
    /// or an optional credential-injected adapter.
    Publish {
        /// Publication target: an artifact path for the local publisher, or a
        /// deployment target name for an adapter.
        #[arg(long)]
        target: String,
        /// Operation key; a retry under the same key never publishes twice.
        #[arg(long, default_value = "share-publish-1")]
        operation_key: String,
        /// Who is publishing; recorded in the publication audit trail.
        #[arg(long, default_value = "local-admin")]
        actor: String,
        /// Optional publication adapter executable receiving the manifest on stdin.
        #[arg(long, value_name = "PATH")]
        adapter: Option<PathBuf>,
    },
    /// Resolve a partial publication reported as `unknown`.
    Reconcile {
        /// Publication attempt id from the audit trail.
        #[arg(long)]
        publication: i64,
        /// The observed outcome: published or failed.
        #[arg(long)]
        result: String,
        /// Who reconciled it; recorded in the publication audit trail.
        #[arg(long, default_value = "local-admin")]
        actor: String,
    },
    /// Show the approval and publication audit trail.
    Audit {
        /// Maximum rows per section (1-500).
        #[arg(long, default_value_t = 50)]
        limit: usize,
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
enum ProjectCommands {
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

/// Subcommands of `forge project github`.
#[derive(Debug, Subcommand)]
enum GithubCommands {
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

/// Shared source selection and filter flags for the catalog commands.
#[derive(Debug, Clone, clap::Args)]
struct CatalogFilterArgs {
    /// Source to read (repeatable): local, git, workspace-registry,
    /// inventory or github. Defaults to `local`; only declared sources are
    /// read and no parent directory is ever scanned.
    #[arg(long = "source", value_name = "KIND")]
    sources: Vec<String>,
    /// Tag predicate (repeatable; values are OR within the predicate).
    #[arg(long = "tag", value_name = "TAG")]
    tags: Vec<String>,
    /// Language predicate (repeatable).
    #[arg(long = "language", value_name = "LANGUAGE")]
    languages: Vec<String>,
    /// Profile predicate (repeatable).
    #[arg(long = "profile", value_name = "PROFILE")]
    profiles: Vec<String>,
    /// Lifecycle predicate (repeatable).
    #[arg(long = "lifecycle", value_name = "LIFECYCLE")]
    lifecycles: Vec<String>,
    /// Repository substring predicate (repeatable).
    #[arg(long = "repository", value_name = "REPO")]
    repositories: Vec<String>,
    /// CI-state predicate (repeatable).
    #[arg(long = "ci", value_name = "STATE")]
    ci: Vec<String>,
    /// Compose-state predicate (repeatable).
    #[arg(long = "compose", value_name = "STATE")]
    compose: Vec<String>,
    /// Evidence-state predicate (repeatable).
    #[arg(long = "evidence", value_name = "STATE")]
    evidence: Vec<String>,
    /// Generic `key=value` filter; an unknown key is a `catalog-invalid`
    /// refusal before any source is contacted.
    #[arg(long = "filter", value_name = "KEY=VALUE")]
    filters: Vec<String>,
    /// Explicit Git working tree for `--source git` (repeatable).
    #[arg(long = "git-repository", value_name = "PATH")]
    git_repositories: Vec<PathBuf>,
    /// Workspace registry document for `--source workspace-registry`
    /// (defaults to $FORGE_WORKSPACE_REGISTRY).
    #[arg(long = "workspace-registry", value_name = "PATH")]
    workspace_registry: Option<PathBuf>,
    /// Portable inventory source for `--source inventory`
    /// (defaults to $FORGE_INVENTORY_SOURCE).
    #[arg(long = "inventory", value_name = "PATH")]
    inventory: Option<PathBuf>,
    /// Explicit GitHub repository for `--source github` (repeatable;
    /// `owner/repo`). When no value is given, the GitHub source walks
    /// the local registry for projects whose `git_remote` points to
    /// github.com.
    #[arg(long = "github-repository", value_name = "OWNER/REPO")]
    github_repositories: Vec<String>,
    /// Read-time staleness window in seconds (1..=31536000; default 86400).
    #[arg(long = "max-age", value_name = "SECONDS", default_value_t = CATALOG_DEFAULT_MAX_AGE_SECONDS)]
    max_age: i64,
    /// Maximum records per page (1..=1000; default 50).
    #[arg(long = "limit", value_name = "N", default_value_t = CATALOG_DEFAULT_LIMIT)]
    limit: usize,
    /// Opaque cursor returned by a previous page.
    #[arg(long = "cursor", value_name = "CURSOR")]
    cursor: Option<String>,
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
enum StandardCommands {
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
enum RemediateCommands {
    /// Inspect automatic findings and produce a read-only plan.
    Scan(RemediationArgs),
    /// Produce a versioned read-only remediation plan.
    Plan(RemediationArgs),
    /// Show the files a remediation plan would change.
    Diff(RemediationArgs),
    /// Apply a plan after explicit confirmation.
    Apply(RemediationArgs),
}

#[derive(Debug, Subcommand)]
enum DescribeCommands {
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
enum ClassifyCommands {
    /// Suggest a bounded classification for the named project.
    Suggest(SemanticSuggestArgs),
    /// List every recorded classification proposal for the named project.
    List {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Show the full proposal manifest for one proposal id.
    Show {
        /// Proposal id (`<kind>-<hash>`, e.g. `domain-deadbeef`).
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

#[derive(Debug, clap::Args)]
struct SemanticSuggestArgs {
    /// Registered project id or filesystem path (default: current directory).
    #[arg(default_value = ".")]
    target: String,
    /// Bounded, control-free suggested value (e.g. `tools for inspecting catalogs`).
    #[arg(long)]
    suggested_value: String,
    /// Optional current value (the value the project has today).
    #[arg(long = "current-value")]
    current_value: Option<String>,
    /// Confidence label: `low`, `medium`, or `high` (default: `medium`).
    #[arg(long, default_value = "medium")]
    confidence: String,
    /// Provider identity: `operator` (default) or `local`.
    #[arg(long, default_value = "operator")]
    provider: String,
    /// Path to the evidence source (repeatable; one per source).
    #[arg(long = "evidence-path", value_name = "PATH")]
    evidence_paths: Vec<String>,
    /// Revision the evidence was observed at. A single revision
    /// applies to every `--evidence-path`; an evidence path
    /// without a paired revision is refused.
    #[arg(long = "evidence-revision", value_name = "REV")]
    evidence_revisions: Vec<String>,
    /// Optional bounded excerpt for each `--evidence-path`.
    #[arg(long = "evidence-excerpt", value_name = "EXCERPT")]
    evidence_excerpts: Vec<String>,
    /// Optional human note recorded with the proposal.
    #[arg(long)]
    note: Option<String>,
}

#[derive(Debug, clap::Args)]
struct RemediationArgs {
    /// Project directory (default: current directory).
    #[arg(long, default_value = ".")]
    target: PathBuf,
    /// Doctor finding id, for example gaps.ci.demo.ci.
    #[arg(long)]
    finding: Option<String>,
    /// Standard pack selector, for example baseline-service@1.1.0.
    #[arg(long)]
    pack: Option<String>,
    /// Read a previously saved JSON plan instead of assembling one.
    #[arg(long)]
    plan: Option<PathBuf>,
    /// Required for apply; prevents accidental writes.
    #[arg(long)]
    confirm: bool,
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
        Commands::Graduation { command } => cmd_graduation(&db_path, command, cli.format),
        Commands::Profile { command } => cmd_profile(command, cli.format),
        Commands::New {
            path,
            profile,
            id,
            name,
            features,
            verify_native,
            no_workspace_metadata,
            standard_pack,
        } => cmd_new(
            &db_path,
            path,
            profile.as_deref(),
            id.as_deref(),
            name.as_deref(),
            features,
            *verify_native,
            !no_workspace_metadata,
            standard_pack.as_deref(),
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
        Commands::Portfolio { command } => cmd_portfolio(&db_path, command, cli.format),
        Commands::Readiness { command } => cmd_readiness(command, cli.format),
        Commands::Provider { command } => cmd_provider(&db_path, command, cli.format),
        Commands::Governance { command } => cmd_governance(command, cli.format),
        Commands::Fleet { command } => cmd_fleet(&db_path, command, cli.format),
        Commands::Project { command } => cmd_project(&db_path, command, cli.format),
        Commands::Contract { command } => cmd_contract(&db_path, command, cli.format),
        Commands::Inventory { command } => cmd_inventory(command, cli.format),
        Commands::Standard { command } => cmd_standard(command, cli.format),
        Commands::Remediate { command } => cmd_remediate(&db_path, command, cli.format),
        Commands::Describe { command } => cmd_describe(command, cli.format),
        Commands::Classify { command } => cmd_classify(command, cli.format),
        Commands::Delivery { command } => cmd_delivery(&db_path, command, cli.format),
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
            // NDJSON is one document per line; every other JSON format
            // is the indented document.
            if cli.format == Format::Ndjson {
                println!("{}", serde_json::to_string(&value).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&value).unwrap());
            }
            ExitCode::SUCCESS
        }
        Ok(Output::Raw(text)) => {
            print!("{text}");
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
    /// Already-serialized machine output (NDJSON): printed verbatim.
    Raw(String),
}

fn open_registry(db_path: &Path) -> Result<Registry, ForgeError> {
    Registry::open(db_path)
}

fn as_output(format: Format, human: String, json: serde_json::Value) -> Output {
    match format {
        Format::Human | Format::Table => Output::Human(human),
        Format::Json | Format::Ndjson => Output::Json(json),
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

/// Map a graduation refusal to the single typed Core error. The
/// refusal code is part of the reason so a caller sees both the class
/// (`graduation-not-validated`) and the named field.
fn graduation_invalid(refusal: GraduationRefusal) -> ForgeError {
    ForgeError::GraduationInvalid {
        reason: format!("{}: {}", refusal.code, refusal.detail),
    }
}

/// Read, decode and validate an artifact. This is the only path into
/// the graduation surface, so `preview`, the dry run and the confirmed
/// import all apply the same gate.
fn load_graduation(artifact: &str) -> Result<GraduationImport, ForgeError> {
    use forge::graduation::{parse_artifact, read_artifact, validate_graduation};
    let raw = read_artifact(artifact)?;
    let record = parse_artifact(&raw).map_err(graduation_invalid)?;
    validate_graduation(&record).map_err(graduation_invalid)
}

fn render_preview_output(
    preview: &GraduationPreview,
    format: Format,
) -> Result<Output, ForgeError> {
    let human = render_preview_human(preview);
    let proposed = match &preview.proposal {
        Some(proposal) => serde_json::to_value(proposal).map_err(|err| ForgeError::Registry {
            reason: err.to_string(),
        })?,
        None => serde_json::Value::Null,
    };
    let json = serde_json::json!({
        "contract": GRADUATION_CONTRACT_VERSION,
        "preview": {
            "artifact": preview.artifact,
            "source": preview.import.source,
            "brief": preview.import.brief,
            "experiment": preview.import.experiment,
            "proposed": proposed,
        },
    });
    Ok(as_output(format, human, json))
}

fn cmd_graduation(
    db_path: &Path,
    command: &GraduationCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        GraduationCommands::Preview { artifact } => cmd_graduation_preview(artifact, format),
        GraduationCommands::Import {
            artifact,
            path,
            profile,
            id,
            actor,
            confirm,
        } => cmd_graduation_import(
            db_path,
            artifact,
            path,
            profile,
            id.as_deref(),
            actor,
            *confirm,
            format,
        ),
    }
}

fn cmd_graduation_preview(artifact: &str, format: Format) -> Result<Output, ForgeError> {
    let import = load_graduation(artifact)?;
    let preview = GraduationPreview {
        artifact: artifact.to_string(),
        import,
        proposal: None,
    };
    render_preview_output(&preview, format)
}

#[allow(clippy::too_many_arguments)]
fn cmd_graduation_import(
    db_path: &Path,
    artifact: &str,
    path: &Path,
    profile: &str,
    id: Option<&str>,
    actor: &str,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let import = load_graduation(artifact)?;
    let proposal = build_proposal(&import, profile, path, id)?;
    if !confirm {
        let preview = GraduationPreview {
            artifact: artifact.to_string(),
            import,
            proposal: Some(proposal),
        };
        return render_preview_output(&preview, format);
    }
    let mut registry = open_registry(db_path)?;
    let now = chrono::Utc::now();
    let adoption = adopt_graduation(&mut registry, &proposal, &import, actor, now)?;
    let human = format!(
        "imported {} ({})\nprofile: {}\nreceipt: {}\nsource: {} hypora_revision={}\nNo scaffold, deploy, network call or gate approval was performed.",
        adoption.record.id,
        adoption.record.path,
        proposal.profile,
        adoption.receipt_path,
        import.source.contract,
        import.source.hypora_revision,
    );
    let json = serde_json::json!({
        "imported": adoption.record,
        "graduation": {
            "contract": GRADUATION_CONTRACT_VERSION,
            "receipt": adoption.receipt_path,
            "source": import.source,
        },
    });
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
    standard_pack: Option<&str>,
    format: Format,
) -> Result<Output, ForgeError> {
    let mut request = if profile.is_some() {
        normalize_explicit(profile, id, name, features, path, standard_pack)?
    } else {
        let stdin = std::io::stdin();
        let mut reader = std::io::BufReader::new(stdin.lock());
        let mut writer = std::io::stderr();
        parse_interactive(
            &mut reader,
            &mut writer,
            path,
            profile,
            id,
            name,
            features,
            standard_pack,
        )?
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

fn cmd_standard(command: &StandardCommands, format: Format) -> Result<Output, ForgeError> {
    use forge::standard;
    match command {
        StandardCommands::List => {
            let packs = standard::all_packs();
            let mut human = format!(
                "{:<18} {:<8} {:<11} {:<10} {}",
                "Pack", "Version", "State", "Evidence", "Compatible profiles"
            );
            for pack in &packs {
                human.push_str(&format!(
                    "\n{:<18} {:<8} {:<11} {:<10} {}",
                    pack.id,
                    pack.version,
                    pack_state_word(pack.support_state),
                    pack_evidence_word(pack.evidence),
                    pack.compatible_profiles.join(", ")
                ));
            }
            let json = serde_json::json!({"packs": packs});
            Ok(as_output(format, human, json))
        }
        StandardCommands::Inspect { pack } => {
            let descriptor = standard::inspect_pack(pack)?;
            let mut human = format!(
                "pack {}@{}\nsupport state: {}\nevidence: {}\nasset digest: {}\ncompatible profiles: {}",
                descriptor.id,
                descriptor.version,
                pack_state_word(descriptor.support_state),
                pack_evidence_word(descriptor.evidence),
                descriptor.asset_digest,
                descriptor.compatible_profiles.join(", ")
            );
            if let Some(source) = descriptor.external_source.as_deref() {
                human.push_str(&format!(
                    "\nexternal template source: {source} (bundled local fallback; never fetched)"
                ));
            }
            human.push_str(&format!(
                "\nselectable for new generation: {}",
                descriptor.is_selectable()
            ));
            let json = serde_json::to_value(&descriptor).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        StandardCommands::Check { path } => {
            let report = standard::check_snapshot(path)?;
            let mut human = format!(
                "standard snapshot: {} ({})",
                snapshot_state_word(report.state),
                report.path
            );
            if let Some(pack) = report.pack.as_deref() {
                human.push_str(&format!(
                    "\npack {}@{} for profile {}",
                    pack,
                    report.version.as_deref().unwrap_or("?"),
                    report.profile.as_deref().unwrap_or("?")
                ));
            }
            for file in &report.files {
                human.push_str(&format!(
                    "\n  {} {}",
                    file_state_word(file.state),
                    file.path
                ));
            }
            for issue in &report.issues {
                human.push_str(&format!("\nissue: {issue}"));
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        StandardCommands::Diff { path, against } => {
            let report = standard::diff_snapshot(path, against)?;
            let mut human = format!(
                "standard diff {} -> {}\nproject {} (profile {})",
                report.from, report.against, report.project, report.profile
            );
            for entry in &report.entries {
                human.push_str(&format!(
                    "\n  {} {}",
                    diff_change_word(entry.change),
                    entry.path
                ));
            }
            if !report.conflicts.is_empty() {
                human.push_str(&format!(
                    "\nconflicts: {} (upgrade refuses without --force)",
                    report.conflicts.join(", ")
                ));
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        StandardCommands::Upgrade {
            path,
            to,
            confirm,
            force,
        } => {
            let report = standard::upgrade_snapshot(
                path,
                to,
                *confirm,
                *force,
                &chrono::Utc::now().to_rfc3339(),
            )?;
            let mut human = format!(
                "upgraded {} ({}) to {}; {} file(s) written",
                report.project,
                report.profile,
                report.to,
                report.written.len()
            );
            if !report.forced.is_empty() {
                human.push_str(&format!(
                    "\nforced replacements: {}",
                    report.forced.join(", ")
                ));
            }
            if !report.orphaned.is_empty() {
                human.push_str(&format!(
                    "\npreserved (not in target): {}",
                    report.orphaned.join(", ")
                ));
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
    }
}

fn cmd_remediate(
    registry_path: &Path,
    command: &RemediateCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let build = |args: &RemediationArgs| -> Result<RemediationPlan, ForgeError> {
        if let Some(path) = args.plan.as_deref() {
            remediation::load_plan(path)
        } else {
            remediation::build_plan(
                &args.target,
                args.finding
                    .as_deref()
                    .ok_or_else(|| ForgeError::RemediationInvalid {
                        reason: "--finding is required when --plan is not supplied".to_string(),
                    })?,
                args.pack.as_deref(),
            )
        }
    };
    match command {
        RemediateCommands::Scan(args) if args.finding.is_none() && args.plan.is_none() => {
            let report = remediation::scan(&args.target)?;
            let names: Vec<_> = report
                .findings
                .iter()
                .map(|finding| finding.finding_id.as_str())
                .collect();
            let human = if names.is_empty() {
                format!(
                    "no automatic remediation findings for {}",
                    report.target.project_id
                )
            } else {
                format!(
                    "automatic findings for {}: {}",
                    report.target.project_id,
                    names.join(", ")
                )
            };
            let json =
                serde_json::to_value(&report).map_err(|err| ForgeError::RemediationInvalid {
                    reason: format!("cannot serialize scan report: {err}"),
                })?;
            Ok(as_output(format, human, json))
        }
        RemediateCommands::Scan(args) | RemediateCommands::Plan(args) => {
            let plan = build(args)?;
            let json = serde_json::json!({
                "contract": remediation::REMEDIATION_CONTRACT_VERSION,
                "plan": plan,
            });
            Ok(as_output(
                format,
                format!(
                    "remediation plan {}\n{} action(s)",
                    json["plan"]["plan_id"],
                    json["plan"]["actions"].as_array().map_or(0, Vec::len)
                ),
                json,
            ))
        }
        RemediateCommands::Diff(args) => {
            let plan = build(args)?;
            let entries = remediation::diff(&plan);
            let json = serde_json::json!({
                "contract": remediation::REMEDIATION_CONTRACT_VERSION,
                "plan": plan,
                "diff": entries,
            });
            Ok(as_output(
                format,
                format!("remediation diff: {} action(s)", entries.len()),
                json,
            ))
        }
        RemediateCommands::Apply(args) => {
            let plan = build(args)?;
            let outcome = remediation::apply(&plan, args.confirm, registry_path)?;
            let json = serde_json::json!({
                "contract": remediation::REMEDIATION_CONTRACT_VERSION,
                "plan": plan,
                "outcome": outcome,
            });
            Ok(as_output(
                format,
                format!(
                    "remediation {}: {}",
                    json["outcome"]["status"], json["outcome"]["detail"]
                ),
                json,
            ))
        }
    }
}

fn cmd_describe(command: &DescribeCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        DescribeCommands::Suggest(args) => {
            let request = build_suggest_request(ProposalKind::Description, args)?;
            let outcome = semantic::suggest(&request)?;
            semantic_suggest_output(&outcome, format)
        }
        DescribeCommands::List { target } => {
            let project_path = resolve_spec_target(target)?;
            let entries = semantic::list(&project_path)?;
            semantic_list_output(&entries, format)
        }
        DescribeCommands::Show { proposal, target } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            match semantic::read(&project_path, &id)? {
                Some(proposal) => semantic_show_output(&proposal, format),
                None => Err(ForgeError::SemanticInvalid {
                    reason: format!("proposal `{proposal}` was not found under `.forge/semantic/`"),
                }),
            }
        }
        DescribeCommands::Approve {
            proposal,
            target,
            confirm,
        } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            let outcome = semantic::approve(&project_path, &id, chrono::Utc::now(), *confirm)?;
            semantic_decide_output(&outcome, format)
        }
        DescribeCommands::Reject {
            proposal,
            target,
            confirm,
        } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            let outcome = semantic::reject(&project_path, &id, chrono::Utc::now(), *confirm)?;
            semantic_decide_output(&outcome, format)
        }
    }
}

fn cmd_classify(command: &ClassifyCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        ClassifyCommands::Suggest(args) => {
            let kind = ProposalKind::Domain;
            let request = build_suggest_request(kind, args)?;
            let outcome = semantic::suggest(&request)?;
            semantic_suggest_output(&outcome, format)
        }
        ClassifyCommands::List { target } => {
            let project_path = resolve_spec_target(target)?;
            let entries = semantic::list(&project_path)?;
            semantic_list_output(&entries, format)
        }
        ClassifyCommands::Show { proposal, target } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            match semantic::read(&project_path, &id)? {
                Some(proposal) => semantic_show_output(&proposal, format),
                None => Err(ForgeError::SemanticInvalid {
                    reason: format!("proposal `{proposal}` was not found under `.forge/semantic/`"),
                }),
            }
        }
        ClassifyCommands::Approve {
            proposal,
            target,
            confirm,
        } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            let outcome = semantic::approve(&project_path, &id, chrono::Utc::now(), *confirm)?;
            semantic_decide_output(&outcome, format)
        }
        ClassifyCommands::Reject {
            proposal,
            target,
            confirm,
        } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            let outcome = semantic::reject(&project_path, &id, chrono::Utc::now(), *confirm)?;
            semantic_decide_output(&outcome, format)
        }
    }
}

fn cmd_delivery(
    db_path: &Path,
    command: &DeliveryCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        DeliveryCommands::Status { project } => cmd_delivery_status(db_path, project, format),
        DeliveryCommands::Preflight { project } => cmd_delivery_preflight(db_path, project, format),
        DeliveryCommands::Stage {
            project,
            confirm_operation_id,
        } => {
            let Some(op_id) = *confirm_operation_id else {
                return Err(ForgeError::DeliveryInvalid {
                    reason: "delivery stage: --confirm-operation-id is required".to_string(),
                });
            };
            cmd_delivery_stage(db_path, project, op_id, format)
        }
        DeliveryCommands::Promote {
            project,
            confirm_revision,
        } => {
            let Some(rev) = confirm_revision.as_deref() else {
                return Err(ForgeError::DeliveryInvalid {
                    reason: "delivery promote: --confirm-revision is required".to_string(),
                });
            };
            cmd_delivery_promote(db_path, project, rev, format)
        }
        DeliveryCommands::HermoraRetry {
            project,
            deployment_url,
            secret_ref,
        } => {
            let Some(url) = deployment_url.as_deref() else {
                return Err(ForgeError::DeliveryInvalid {
                    reason: "delivery hermora-retry: --deployment-url is required".to_string(),
                });
            };
            let Some(ref_name) = secret_ref.as_deref() else {
                return Err(ForgeError::DeliveryInvalid {
                    reason: "delivery hermora-retry: --secret-ref is required".to_string(),
                });
            };
            cmd_delivery_hermora_retry(db_path, project, url, ref_name, format)
        }
    }
}

fn cmd_delivery_status(
    db_path: &Path,
    project: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let (report, json) = forge::delivery::cli::cmd_delivery_status(db_path, project)?;
    let human = forge::delivery::projection::render_report_human(&report);
    Ok(as_output(format, human, json))
}

fn cmd_delivery_preflight(
    db_path: &Path,
    project: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let outcome = forge::delivery::cli::cmd_delivery_preflight(db_path, project)?;
    let json =
        serde_json::to_value(&outcome.report).map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!("delivery preflight: cannot encode report: {err}"),
        })?;
    let provider_line = outcome
        .provider_status
        .as_deref()
        .map(|s| format!("\nprovider_status: {s}"))
        .unwrap_or_default();
    let human = format!(
        "delivery preflight recorded for `{project}`\nop_id: {}\nphase: {}{provider_line}",
        outcome.op_id, outcome.report.phase
    );
    Ok(as_output(format, human, json))
}

fn cmd_delivery_stage(
    db_path: &Path,
    project: &str,
    confirm_operation_id: i64,
    format: Format,
) -> Result<Output, ForgeError> {
    let outcome = forge::delivery::cli::cmd_delivery_stage(db_path, project, confirm_operation_id)?;
    let json =
        serde_json::to_value(&outcome.report).map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!("delivery stage: cannot encode report: {err}"),
        })?;
    let provider_line = outcome
        .provider_status
        .as_deref()
        .map(|s| format!("\nprovider_status: {s}"))
        .unwrap_or_default();
    let human = format!(
        "delivery stage recorded for `{project}`\nop_id: {}\nphase: {}{provider_line}",
        outcome.op_id, outcome.report.phase
    );
    Ok(as_output(format, human, json))
}

fn cmd_delivery_promote(
    db_path: &Path,
    project: &str,
    confirm_revision: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let outcome = forge::delivery::cli::cmd_delivery_promote(db_path, project, confirm_revision)?;
    let json =
        serde_json::to_value(&outcome.report).map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!("delivery promote: cannot encode report: {err}"),
        })?;
    let provider_line = outcome
        .provider_status
        .as_deref()
        .map(|s| format!("\nprovider_status: {s}"))
        .unwrap_or_default();
    let human = format!(
        "delivery promote recorded for `{project}`\nop_id: {}\nphase: {}{provider_line}",
        outcome.op_id, outcome.report.phase
    );
    Ok(as_output(format, human, json))
}

fn cmd_delivery_hermora_retry(
    db_path: &Path,
    project: &str,
    deployment_url: &str,
    secret_ref: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let outcome = forge::delivery::cli::cmd_delivery_hermora_retry(
        db_path,
        project,
        deployment_url,
        secret_ref,
    )?;
    let json =
        serde_json::to_value(&outcome.report).map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!("delivery hermora-retry: cannot encode report: {err}"),
        })?;
    let provider_line = outcome
        .provider_status
        .as_deref()
        .map(|s| format!("\nhermora_status: {s}"))
        .unwrap_or_default();
    let human = format!(
        "delivery hermora-retry recorded for `{project}`\nop_id: {}\nphase: {}{provider_line}",
        outcome.op_id, outcome.report.phase
    );
    Ok(as_output(format, human, json))
}

fn build_suggest_request(
    kind: ProposalKind,
    args: &SemanticSuggestArgs,
) -> Result<SuggestRequest, ForgeError> {
    let project_path = resolve_spec_target(&args.target)?;
    if args.evidence_paths.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: "suggest requires at least one --evidence-path".to_string(),
        });
    }
    if args.evidence_revisions.len() != 1 {
        return Err(ForgeError::SemanticInvalid {
            reason: "--evidence-revision must be passed exactly once; the same revision applies to every --evidence-path".to_string(),
        });
    }
    let revision = &args.evidence_revisions[0];
    let mut evidence: Vec<ProposalEvidence> = Vec::with_capacity(args.evidence_paths.len());
    for (index, path) in args.evidence_paths.iter().enumerate() {
        let excerpt = args
            .evidence_excerpts
            .get(index)
            .cloned()
            .unwrap_or_default();
        evidence.push(ProposalEvidence::new(
            path.clone(),
            revision.clone(),
            excerpt,
        )?);
    }
    Ok(SuggestRequest {
        project_path,
        kind,
        current_value: args.current_value.clone(),
        suggested_value: args.suggested_value.clone(),
        confidence: forge::semantic::parse_confidence(&args.confidence)?,
        provider: forge::semantic::parse_provider(&args.provider)?,
        evidence,
        note: args.note.clone(),
        now: chrono::Utc::now(),
    })
}

fn parse_proposal_id(
    _target: &str,
    project_path: &std::path::Path,
    raw: &str,
) -> Result<forge::semantic::ProposalId, ForgeError> {
    let (kind_label, hash) = raw.rsplit_once('-').ok_or_else(|| ForgeError::SemanticInvalid {
        reason: format!(
            "proposal id `{raw}` is malformed: expected `<kind>-<hash>` (e.g. `description-deadbeef`)"
        ),
    })?;
    let kind = forge::semantic::parse_kind(kind_label)?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(project_path, None)
        .map_err(|err| ForgeError::SemanticInvalid {
            reason: format!("cannot resolve project id: {err}"),
        })?;
    Ok(forge::semantic::ProposalId {
        project_id: manifest.project.id,
        kind,
        hash: hash.to_string(),
    })
}

fn semantic_suggest_output(
    outcome: &forge::semantic::SuggestOutcome,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "status": outcome.status_label(),
        "note": outcome.note,
        "files_written": outcome.files_written,
        "proposal": outcome.proposal,
    });
    let human = match outcome.proposal.as_ref() {
        Some(proposal) => format!(
            "semantic {}: {}\nfiles: {}\n{}",
            outcome.status_label(),
            proposal.id.dir_name(),
            if outcome.files_written.is_empty() {
                "(none)".to_string()
            } else {
                outcome.files_written.join(", ")
            },
            outcome.note
        ),
        None => format!(
            "semantic {}: {}\n{}",
            outcome.status_label(),
            outcome.note,
            "(no proposal stored)"
        ),
    };
    Ok(as_output(format, human, json))
}

fn semantic_list_output(
    entries: &[forge::semantic::ProposalListEntry],
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"proposals": entries});
    let mut human = format!(
        "{:<48} {:<16} {:<11} {:<8} {:<8} {}",
        "Proposal", "Kind", "State", "Conf", "Provider", "Evidence"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<48} {:<16} {:<11} {:<8} {:<8} {}",
            entry.dir_name,
            entry.kind.label(),
            entry.state.label(),
            entry.confidence.label(),
            entry.provider.label(),
            entry.evidence_count
        ));
    }
    Ok(as_output(format, human, json))
}

fn semantic_show_output(
    proposal: &forge::semantic::Proposal,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"proposal": proposal});
    let mut human = format!(
        "proposal: {}\nstate: {}\nkind: {}\nconfidence: {}\nprovider: {}\nsuggested_value: {}\n",
        proposal.id.dir_name(),
        proposal.state,
        proposal.kind,
        proposal.confidence,
        proposal.provider,
        proposal.suggested_value
    );
    for ev in &proposal.evidence {
        human.push_str(&format!(
            "evidence: path=`{}` revision=`{}`\n",
            ev.path, ev.revision
        ));
    }
    if let Some(conflict) = &proposal.conflict {
        human.push_str(&format!(
            "conflict: first=`{}` second=`{}`\n",
            conflict.first_value, conflict.second_value
        ));
    }
    Ok(as_output(format, human, json))
}

fn semantic_decide_output(
    outcome: &forge::semantic::DecideOutcome,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "id": outcome.id,
        "state": outcome.state,
        "files_written": outcome.files_written,
        "note": outcome.note,
    });
    let human = format!(
        "semantic proposal {} moved to state {}\nfiles: {}\n{}",
        outcome.id.dir_name(),
        outcome.state,
        if outcome.files_written.is_empty() {
            "(none)".to_string()
        } else {
            outcome.files_written.join(", ")
        },
        outcome.note
    );
    Ok(as_output(format, human, json))
}

fn pack_state_word(state: forge::standard::PackSupportState) -> &'static str {
    match state {
        forge::standard::PackSupportState::Proposed => "proposed",
        forge::standard::PackSupportState::Supported => "supported",
        forge::standard::PackSupportState::Deprecated => "deprecated",
    }
}

fn pack_evidence_word(evidence: forge::standard::PackEvidence) -> &'static str {
    match evidence {
        forge::standard::PackEvidence::Verified => "verified",
        forge::standard::PackEvidence::Unverified => "unverified",
    }
}

fn snapshot_state_word(state: forge::standard::SnapshotState) -> &'static str {
    match state {
        forge::standard::SnapshotState::Absent => "absent",
        forge::standard::SnapshotState::Rendered => "rendered",
        forge::standard::SnapshotState::Modified => "modified",
        forge::standard::SnapshotState::Unknown => "unknown-pack",
    }
}

fn file_state_word(state: forge::standard::FileState) -> &'static str {
    match state {
        forge::standard::FileState::Present => "ok",
        forge::standard::FileState::Modified => "modified",
        forge::standard::FileState::Missing => "missing",
    }
}

fn diff_change_word(change: forge::standard::DiffChange) -> &'static str {
    match change {
        forge::standard::DiffChange::Added => "added",
        forge::standard::DiffChange::Updated => "updated",
        forge::standard::DiffChange::Unchanged => "unchanged",
        forge::standard::DiffChange::Modified => "modified",
        forge::standard::DiffChange::Foreign => "foreign",
        forge::standard::DiffChange::Orphaned => "orphaned",
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
        Output::Raw(text) => print!("{text}"),
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
        Format::Human | Format::Table => Output::Human(render_fleet_human(report)),
        Format::Json | Format::Ndjson => Output::Json(serde_json::json!({"fleet": report})),
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
        Format::Human | Format::Table => eprintln!("error[{}]: {}", err.code(), err),
        Format::Json | Format::Ndjson => {
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
            Output::Raw(text) => print!("{text}"),
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
            Output::Raw(text) => print!("{text}"),
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
            Output::Raw(text) => print!("{text}"),
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
            Output::Raw(text) => print!("{text}"),
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
            Output::Raw(text) => print!("{text}"),
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
                jobs,
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
                    *jobs,
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
/// Maximum `--jobs` value: 32 workers bound the target and the
/// controller without a thread-per-project explosion on large
/// rosters (`fleet-live-rollout`).
pub const FLEET_MAX_JOBS: usize = 32;

/// Default `--jobs` value: four concurrent per-project publishes.
pub const FLEET_DEFAULT_JOBS: usize = 4;

/// Validate the `--jobs` flag: `1..=FLEET_MAX_JOBS`. Refuses
/// out-of-range values with a typed error before anything runs.
pub fn validate_fleet_jobs(jobs: usize) -> Result<usize, ForgeError> {
    if !(1..=FLEET_MAX_JOBS).contains(&jobs) {
        return Err(ForgeError::PublishInvalid {
            reason: format!("fleet --jobs {jobs} is out of bounds; expected 1..={FLEET_MAX_JOBS}"),
        });
    }
    Ok(jobs)
}

fn cmd_publish_fleet(
    db_path: &Path,
    inventory: Option<std::path::PathBuf>,
    registry_path: Option<std::path::PathBuf>,
    workspace_root: Option<std::path::PathBuf>,
    dry_run: bool,
    lifecycle: String,
    fail_fast: bool,
    jobs: usize,
    provider: Option<String>,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::publish::fleet::{default_registry_path, default_workspace_root};
    use forge::publish::inventory::{
        classify, resolve_source, InventoryClassification, DEFAULT_DOMAIN,
    };

    let jobs = validate_fleet_jobs(jobs)?;
    // An explicit `--fleet-registry <path>` always routes through
    // the legacy `projects.json` compatibility adapter — never the
    // inventory branch — so a valid `projects.json` classifies its
    // entries instead of refusing a contract error
    // (`fleet-live-rollout` D3). `--inventory` (flag or env) keeps
    // precedence; the default registry path applies only when no
    // explicit source exists at all.
    let (snapshot, source_label) = if let Some(path) = registry_path.clone() {
        let workspace_root = workspace_root
            .clone()
            .unwrap_or_else(default_workspace_root);
        let snapshot = legacy_inventory_snapshot(&path, &workspace_root, &lifecycle)?;
        (snapshot, format!("registry:{}", path.display()))
    } else if let Some(path) = inventory
        .as_ref()
        .map(|p| p.to_path_buf())
        .or_else(|| resolve_source(None))
    {
        let snapshot = load_inventory_snapshot(&path)?;
        let label = format!("inventory:{}", path.display());
        (snapshot, label)
    } else {
        // No explicit source — synthesise an inventory from the
        // legacy workspace-governance registry so the seven-project
        // handoff keeps working without configuration.
        let workspace_root = workspace_root.unwrap_or_else(default_workspace_root);
        let registry_path = default_registry_path(Some(&workspace_root));
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

    // The external `--provider` branch always stays sequential: it
    // shells through `cmd_publish_provider` (which re-opens the
    // registry per project), so no thread owns it
    // (`fleet-live-rollout` D1).
    let use_parallel = provider.is_none() && jobs > 1 && !dry_run;
    if use_parallel {
        // Concurrent fleet publish (`fleet-live-rollout` D1, phased
        // refinement): worker threads own phase execution
        // (per-thread adapter + transport, both cheap to construct
        // and holding `RefCell` state that is never shared). The
        // main thread replays every journal row serially from the
        // returned reports and renders all output — row shapes and
        // queue id semantics are unchanged.
        //
        // Phases exist because Prepare renders *global* target
        // state (the shared `port-registry.json`, Caddyfile and
        // index) from whatever the target currently holds: running
        // N Prepares concurrently is last-writer-wins on shared
        // files and each render misses the sibling projects. So:
        // Phase A runs Sync+Db in parallel (per-project remote
        // dirs; Db is idempotent), Phase B runs Prepare serially in
        // roster order so the registry converges (each render sees
        // every prior project), Phase C runs Deploy in parallel
        // (per-project `forge-<id>` compose projects). Per-project
        // order Sync -> Db -> Prepare -> Deploy is preserved; with
        // fail-fast, later phases never start a project whose
        // earlier phase failed, and Phase B/C stop scheduling new
        // projects after the first failure they observe.
        let legacy_lane = use_legacy_publish_adapter();
        let db_path_owned = db_path.to_path_buf();
        let phase_a = move |entry: forge::publish::inventory::InventoryFleetEntry| {
            run_fleet_entry_phases(
                entry,
                &db_path_owned,
                legacy_lane,
                dry_run,
                &[PublishAction::Sync, PublishAction::Db],
            )
        };
        let phase_a_outcomes = run_fleet_concurrent(&eligible, jobs, fail_fast, phase_a);
        // Per-project accumulated state across phases.
        struct FleetProgress {
            reports: Vec<forge::publish::PublishReport>,
            failed: bool,
            fail_error: Option<ForgeError>,
            skipped: bool,
        }
        let mut progress: Vec<FleetProgress> = phase_a_outcomes
            .into_iter()
            .map(|outcome| match outcome {
                FleetEntryOutcome::Phased { reports, error } => FleetProgress {
                    reports,
                    failed: error.is_some(),
                    fail_error: error,
                    skipped: false,
                },
                FleetEntryOutcome::Failed { error, .. } => FleetProgress {
                    reports: Vec::new(),
                    failed: true,
                    fail_error: Some(error),
                    skipped: false,
                },
                FleetEntryOutcome::Published { .. } => FleetProgress {
                    reports: Vec::new(),
                    failed: true,
                    fail_error: Some(ForgeError::PublishInvalid {
                        reason: "internal fleet phase error: unexpected outcome unit".to_string(),
                    }),
                    skipped: false,
                },
            })
            .collect();
        let core_registry = open_registry(db_path)?;
        // Replay Phase A journal rows serially (per-stage + summary
        // rows exactly as `run_publish` writes them) and render.
        for (entry, prog) in eligible.iter().zip(progress.iter()) {
            for report in &prog.reports {
                replay_publish_journal(&core_registry, report);
                let human = render_publish_report_human(report);
                let mut value =
                    serde_json::to_value(report).map_err(|err| ForgeError::PublishInvalid {
                        reason: format!("cannot encode report for {}: {err}", entry.id),
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
                render_output(as_output(format, human, value));
            }
        }
        // Phase B: serial Prepare in roster order. The shared
        // adapter/transport pair is main-thread owned, exactly like
        // the sequential path.
        let jenkins_adapter = JenkinsAdapter::from_env();
        let remote_adapter = RemoteComposeAdapter::from_env();
        let adapter: &dyn forge::publish::PublishAdapter = if legacy_lane {
            &jenkins_adapter
        } else {
            &remote_adapter
        };
        let transport = SubprocessTransport::default();
        let mut phase_b_stopped = false;
        for (entry, prog) in eligible.iter().zip(progress.iter_mut()) {
            if prog.failed || (fail_fast && phase_b_stopped) {
                if !prog.failed {
                    prog.skipped = true;
                }
                continue;
            }
            let source_path = entry
                .source_path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let request = build_publish_request(
                entry.id.clone(),
                source_path,
                PublishAction::Prepare,
                dry_run,
            );
            match run_publish(&request, adapter, &transport, Some(&core_registry)) {
                Ok(report) => {
                    let human = render_publish_report_human(&report);
                    let mut value = serde_json::to_value(&report).map_err(|err| {
                        ForgeError::PublishInvalid {
                            reason: format!("cannot encode report for {}: {err}", entry.id),
                        }
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
                    render_output(as_output(format, human, value));
                    if !report.healthy {
                        prog.failed = true;
                        prog.fail_error = Some(ForgeError::PublishDeployFailed {
                            reason: format!("fleet prepare failed at `{}`", entry.id),
                        });
                        if fail_fast {
                            phase_b_stopped = true;
                        }
                    }
                    prog.reports.push(report);
                }
                Err(err) => {
                    prog.failed = true;
                    prog.fail_error = Some(err);
                    if fail_fast {
                        phase_b_stopped = true;
                    }
                }
            }
        }
        // Phase C: parallel Deploy for projects whose A+B succeeded.
        let deployable: Vec<(usize, forge::publish::inventory::InventoryFleetEntry)> = eligible
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                progress
                    .get(entry_index(eligible.as_slice(), &entry.id))
                    .map(|p| !p.failed && !p.skipped)
                    .unwrap_or(false)
            })
            .map(|(i, e)| (i, e.clone()))
            .collect();
        let db_path_owned_c = db_path.to_path_buf();
        let phase_c = move |entry: forge::publish::inventory::InventoryFleetEntry| {
            run_fleet_entry_phases(
                entry,
                &db_path_owned_c,
                legacy_lane,
                dry_run,
                &[PublishAction::Deploy],
            )
        };
        let deploy_entries: Vec<forge::publish::inventory::InventoryFleetEntry> =
            deployable.iter().map(|(_, e)| e.clone()).collect();
        let phase_c_outcomes = run_fleet_concurrent(&deploy_entries, jobs, fail_fast, phase_c);
        for ((index, _), outcome) in deployable.iter().zip(phase_c_outcomes.into_iter()) {
            let prog = &mut progress[*index];
            match outcome {
                FleetEntryOutcome::Phased { reports, error } => {
                    for report in &reports {
                        replay_publish_journal(&core_registry, report);
                        let human = render_publish_report_human(report);
                        let entry = &eligible[*index];
                        let mut value = serde_json::to_value(report).map_err(|err| {
                            ForgeError::PublishInvalid {
                                reason: format!("cannot encode report for {}: {err}", entry.id),
                            }
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
                        render_output(as_output(format, human, value));
                    }
                    prog.reports.extend(reports);
                    if let Some(err) = error {
                        prog.failed = true;
                        prog.fail_error = Some(err);
                    }
                }
                FleetEntryOutcome::Failed { error, .. } => {
                    prog.failed = true;
                    prog.fail_error = Some(error);
                }
                FleetEntryOutcome::Published { .. } => {
                    prog.failed = true;
                    prog.fail_error = Some(ForgeError::PublishInvalid {
                        reason: "internal fleet phase error: unexpected outcome unit".to_string(),
                    });
                }
            }
        }
        // Fleet summary: one queue-tagged terminal row per project,
        // serially, in roster order — identical shape to the
        // sequential path.
        for (entry, prog) in eligible.iter().zip(progress.iter_mut()) {
            let project_id = entry.id.clone();
            if prog.skipped {
                failure_count += 1;
                let detail = format!(
                    "fleet publish for `{project_id}` skipped: fail-fast stopped scheduling"
                );
                summaries.push(serde_json::json!({
                    "project": project_id,
                    "healthy": false,
                    "error": "fail-fast-skipped",
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
                if first_failure.is_none() {
                    first_failure = Some(ForgeError::PublishInvalid { reason: detail });
                }
                continue;
            }
            let healthy = !prog.failed && prog.reports.iter().all(|report| report.healthy);
            let stage_count: usize = prog.reports.iter().map(|r| r.stages.len()).sum();
            let subdomain = prog
                .reports
                .iter()
                .filter_map(|r| r.subdomain.clone())
                .next();
            summaries.push(serde_json::json!({
                "project": project_id,
                "healthy": healthy,
                "subdomain": subdomain,
                "stages": stage_count,
            }));
            let terminal_state = if healthy { "done" } else { "failed" };
            let journal = if prog.reports.is_empty() {
                format!(
                    "fleet publish failed before any stage: {}",
                    prog.fail_error
                        .as_ref()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "unknown error".to_string())
                )
            } else {
                format!("fleet stages={stage_count} healthy={healthy}")
            };
            if let Ok(op_id) = core_registry.record_queue_operation(
                "publish",
                &project_id,
                &queue_id,
                terminal_state,
                Some(&journal),
            ) {
                queue_op_ids.push((project_id.clone(), op_id));
            }
            if healthy {
                success_count += 1;
            } else {
                failure_count += 1;
                if fail_fast && first_failure.is_none() {
                    first_failure = prog.fail_error.take().or_else(|| {
                        Some(ForgeError::PublishDeployFailed {
                            reason: format!("fleet publish failed at `{project_id}`"),
                        })
                    });
                }
            }
        }
    }

    if !use_parallel {
        let jenkins_adapter = JenkinsAdapter::from_env();
        let remote_adapter = RemoteComposeAdapter::from_env();
        let adapter: &dyn forge::publish::PublishAdapter = if use_legacy_publish_adapter() {
            &jenkins_adapter
        } else {
            &remote_adapter
        };
        let core_registry = open_registry(db_path)?;
        let transport = SubprocessTransport::default();

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
                                    forge::publish::providers::compose_project_name(
                                        &project_id,
                                        rev,
                                    )
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
                                    reason: format!(
                                        "fleet provider publish failed at `{project_id}`"
                                    ),
                                });
                                break;
                            }
                        }
                    }
                    Ok(Output::Human(text)) | Ok(Output::Raw(text)) => {
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
                    let mut value = serde_json::to_value(&report).map_err(|err| {
                        ForgeError::PublishInvalid {
                            reason: format!("cannot encode report for {project_id}: {err}"),
                        }
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

    let summary = {
        let mut summary = summary;
        if let Some(fleet) = summary.get_mut("fleet").and_then(|f| f.as_object_mut()) {
            fleet.insert("jobs".to_string(), serde_json::json!(jobs));
        }
        summary
    };
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

/// One per-project fleet outcome returned by a worker thread. The
/// worker performs the publish phases (transport + in-memory
/// reports) but writes no journal row and prints nothing; the main
/// thread replays every journal row serially and renders all
/// output so concurrent runs keep identical row shapes and queue
/// id semantics (`fleet-live-rollout` D1).
enum FleetEntryOutcome {
    Published {
        human: String,
        value: serde_json::Value,
        healthy: bool,
        stage_count: usize,
        subdomain: Option<String>,
        journal: String,
    },
    Failed {
        error: ForgeError,
        journal: Option<String>,
    },
    /// Phased parallel outcome: the full phase reports (carrying
    /// every stage outcome the journal replay needs) plus the
    /// phase-terminal error when the phase did not complete.
    Phased {
        reports: Vec<forge::publish::PublishReport>,
        error: Option<ForgeError>,
    },
}

/// Roster index of a project id. The phased fleet loop keeps worker
/// results aligned to roster order through indices, not clones.
fn entry_index(
    eligible: &[forge::publish::inventory::InventoryFleetEntry],
    project_id: &str,
) -> usize {
    eligible
        .iter()
        .position(|entry| entry.id == project_id)
        .unwrap_or(0)
}

/// Replay the exact journal rows `run_publish` would have written
/// for one report: one per-stage row plus the summary row. The main
/// thread calls this serially for every worker-returned report, so
/// parallel runs persist byte-identical row shapes to sequential
/// runs (only timestamps differ, as between any two runs).
fn replay_publish_journal(registry: &Registry, report: &forge::publish::PublishReport) {
    use forge::publish::publish_journal_state;
    for outcome in &report.stages {
        let _ = registry.record_operation(
            "publish",
            &report.project_id,
            publish_journal_state(&outcome.status),
            &format!(
                "publish {} via {}: {} ({}, {}ms)",
                outcome.stage, report.adapter, outcome.note, outcome.status, outcome.elapsed_ms
            ),
        );
    }
    let verdict = if report.dry_run || report.healthy {
        "done"
    } else {
        "failed"
    };
    let _ = registry.record_operation(
        "publish",
        &report.project_id,
        verdict,
        &format!(
            "publish {} summary via {}: healthy={} stages={}",
            report.action,
            report.adapter,
            report.healthy,
            report.stages.len()
        ),
    );
}

/// Run a subset of publish phases for one eligible fleet entry on
/// the calling thread. A fresh adapter + transport pair is built
/// per call (adapters hold `RefCell` state: not shareable, cheap
/// to construct) and the publish runs with `registry = None` so no
/// row escapes the worker; the main thread replays every journal
/// row serially from the returned reports.
fn run_fleet_entry_phases(
    entry: forge::publish::inventory::InventoryFleetEntry,
    db_path: &std::path::Path,
    legacy_lane: bool,
    dry_run: bool,
    phases: &[PublishAction],
) -> FleetEntryOutcome {
    let project_id = entry.id.clone();
    let source_path = entry
        .source_path
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    if !source_path.is_dir() {
        let detail = format!(
            "fleet publish: project `{project_id}` source `{}` is not a directory",
            source_path.display()
        );
        return FleetEntryOutcome::Failed {
            error: ForgeError::PublishInvalid {
                reason: detail.clone(),
            },
            journal: Some(detail),
        };
    }
    // Touch the registry once so a missing/unusable database fails
    // closed on the worker with the same typed error the
    // sequential path surfaces — without writing any row.
    if let Err(err) = open_registry(db_path) {
        return FleetEntryOutcome::Failed {
            error: err,
            journal: None,
        };
    }
    let jenkins_adapter = JenkinsAdapter::from_env();
    let remote_adapter = RemoteComposeAdapter::from_env();
    let adapter: &dyn forge::publish::PublishAdapter = if legacy_lane {
        &jenkins_adapter
    } else {
        &remote_adapter
    };
    let transport = SubprocessTransport::default();
    let mut reports = Vec::with_capacity(phases.len());
    for action in phases {
        let request = build_publish_request(
            project_id.clone(),
            source_path.clone(),
            action.clone(),
            dry_run,
        );
        match run_publish(&request, adapter, &transport, None) {
            Ok(report) => {
                let healthy = report.healthy;
                reports.push(report);
                // Phase order is Sync -> Db -> Prepare -> Deploy: a
                // failed phase never starts the next one for this
                // project (same fail-closed rule the All action
                // applies inside `run_publish`).
                if !healthy && !dry_run {
                    break;
                }
            }
            Err(err) => {
                return FleetEntryOutcome::Phased {
                    reports,
                    error: Some(err),
                };
            }
        }
    }
    FleetEntryOutcome::Phased {
        reports,
        error: None,
    }
}

/// Schedule per-project fleet units across at most `jobs` worker
/// threads. Results return in roster order. With `fail_fast`,
/// scheduling stops once a worker reports failure — already-running
/// projects finish, no new projects start.
fn run_fleet_concurrent<F>(
    eligible: &[forge::publish::inventory::InventoryFleetEntry],
    jobs: usize,
    fail_fast: bool,
    worker: F,
) -> Vec<FleetEntryOutcome>
where
    F: Fn(forge::publish::inventory::InventoryFleetEntry) -> FleetEntryOutcome + Sync,
{
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    let width = jobs.max(1).min(FLEET_MAX_JOBS).min(eligible.len().max(1));
    let stop = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel::<(usize, FleetEntryOutcome)>();
    // Atomic index dispenser: workers claim the next roster slot.
    let next: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..width {
            scope.spawn(|| {
                loop {
                    if fail_fast && stop.load(Ordering::SeqCst) {
                        return;
                    }
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(entry) = eligible.get(index) else {
                        return;
                    };
                    if fail_fast && stop.load(Ordering::SeqCst) {
                        return;
                    }
                    let outcome = worker(entry.clone());
                    if fail_fast
                        && !matches!(outcome, FleetEntryOutcome::Published { healthy: true, .. })
                    {
                        stop.store(true, Ordering::SeqCst);
                    }
                    // The main thread always drains; a send failure
                    // means it went away, so the worker exits.
                    if tx.send((index, outcome)).is_err() {
                        return;
                    }
                }
            });
        }
    });
    drop(tx);
    let mut ordered: Vec<Option<FleetEntryOutcome>> = (0..eligible.len()).map(|_| None).collect();
    for (index, outcome) in rx {
        if index < ordered.len() {
            ordered[index] = Some(outcome);
        }
    }
    // Fail-fast may leave tail slots unclaimed; fill them with a
    // typed skip so result order still matches roster order.
    ordered
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            slot.unwrap_or_else(|| FleetEntryOutcome::Failed {
                error: ForgeError::PublishInvalid {
                    reason: format!(
                        "fleet publish for `{}` was not scheduled after an earlier fail-fast failure",
                        eligible[index].id
                    ),
                },
                journal: Some(format!(
                    "fleet publish for `{}` skipped: fail-fast stopped scheduling",
                    eligible[index].id
                )),
            })
        })
        .collect()
}

#[cfg(test)]
mod fleet_jobs_tests {
    use super::{run_fleet_concurrent, validate_fleet_jobs, FleetEntryOutcome};
    use forge::core::ForgeError;

    #[test]
    fn jobs_flag_is_bounded() {
        assert!(validate_fleet_jobs(1).is_ok());
        assert!(validate_fleet_jobs(4).is_ok());
        assert!(validate_fleet_jobs(32).is_ok());
        assert!(validate_fleet_jobs(0).is_err());
        assert!(validate_fleet_jobs(33).is_err());
        assert!(validate_fleet_jobs(usize::MAX).is_err());
    }

    #[test]
    fn concurrent_scheduler_preserves_roster_order() {
        let entries: Vec<forge::publish::inventory::InventoryFleetEntry> = (0..8)
            .map(|i| forge::publish::inventory::InventoryFleetEntry {
                id: format!("proj-{i:02}"),
                runtime: forge::publish::inventory::RuntimeClass::Web,
                profile: String::new(),
                revision: "0".repeat(40),
                classification: forge::publish::inventory::InventoryClassification::ComposeReady,
                compose_file: Some("docker-compose.yml".to_string()),
                source_path: None,
                public_http: false,
                public_port: None,
                subdomain: None,
                reason: None,
            })
            .collect();
        let outcomes =
            run_fleet_concurrent(&entries, 4, false, |entry| FleetEntryOutcome::Published {
                human: entry.id.clone(),
                value: serde_json::json!({}),
                healthy: true,
                stage_count: 4,
                subdomain: None,
                journal: String::new(),
            });
        let ids: Vec<String> = outcomes
            .iter()
            .map(|o| match o {
                FleetEntryOutcome::Published { human, .. } => human.clone(),
                FleetEntryOutcome::Failed { .. } => "failed".to_string(),
                FleetEntryOutcome::Phased { .. } => "phased".to_string(),
            })
            .collect();
        assert_eq!(
            ids,
            (0..8).map(|i| format!("proj-{i:02}")).collect::<Vec<_>>()
        );
    }

    #[test]
    fn phased_outcomes_preserve_roster_order() {
        let entries: Vec<forge::publish::inventory::InventoryFleetEntry> = (0..5)
            .map(|i| forge::publish::inventory::InventoryFleetEntry {
                id: format!("proj-{i:02}"),
                runtime: forge::publish::inventory::RuntimeClass::Web,
                profile: String::new(),
                revision: "0".repeat(40),
                classification: forge::publish::inventory::InventoryClassification::ComposeReady,
                compose_file: Some("docker-compose.yml".to_string()),
                source_path: None,
                public_http: false,
                public_port: None,
                subdomain: None,
                reason: None,
            })
            .collect();
        let outcomes =
            run_fleet_concurrent(&entries, 3, false, |_entry| FleetEntryOutcome::Phased {
                reports: Vec::new(),
                error: None,
            });
        assert_eq!(outcomes.len(), 5);
        assert!(outcomes.iter().all(|o| matches!(
            o,
            FleetEntryOutcome::Phased {
                reports: _,
                error: None
            }
        )));
    }

    #[test]
    fn concurrent_scheduler_stops_scheduling_on_fail_fast() {
        let entries: Vec<forge::publish::inventory::InventoryFleetEntry> = (0..6)
            .map(|i| forge::publish::inventory::InventoryFleetEntry {
                id: format!("proj-{i:02}"),
                runtime: forge::publish::inventory::RuntimeClass::Web,
                profile: String::new(),
                revision: "0".repeat(40),
                classification: forge::publish::inventory::InventoryClassification::ComposeReady,
                compose_file: Some("docker-compose.yml".to_string()),
                source_path: None,
                public_http: false,
                public_port: None,
                subdomain: None,
                reason: None,
            })
            .collect();
        let outcomes = run_fleet_concurrent(&entries, 1, true, |entry| {
            if entry.id == "proj-00" {
                FleetEntryOutcome::Failed {
                    error: ForgeError::PublishInvalid {
                        reason: "boom".to_string(),
                    },
                    journal: None,
                }
            } else {
                FleetEntryOutcome::Published {
                    human: entry.id.clone(),
                    value: serde_json::json!({}),
                    healthy: true,
                    stage_count: 4,
                    subdomain: None,
                    journal: String::new(),
                }
            }
        });
        assert!(matches!(outcomes[0], FleetEntryOutcome::Failed { .. }));
        // Tail slots were never scheduled: the scheduler fills them
        // with a typed skip rather than running them.
        assert!(matches!(outcomes[1], FleetEntryOutcome::Failed { .. }));
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

fn portfolio_invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioInvalid { reason }
}

fn parse_lifecycle(raw: &str) -> Result<forge::portfolio::Lifecycle, ForgeError> {
    forge::portfolio::Lifecycle::parse(raw).map_err(portfolio_invalid)
}

fn parse_confidence(raw: &str) -> Result<forge::portfolio::Confidence, ForgeError> {
    forge::portfolio::Confidence::parse(raw).map_err(portfolio_invalid)
}

fn parse_relation_type(raw: &str) -> Result<forge::portfolio::RelationType, ForgeError> {
    forge::portfolio::RelationType::parse(raw).map_err(portfolio_invalid)
}

fn parse_evidence_status(raw: &str) -> Result<forge::portfolio::EvidenceStatus, ForgeError> {
    forge::portfolio::EvidenceStatus::parse(raw).map_err(portfolio_invalid)
}

/// Read an evidence payload supplied as inline JSON or as `@path`.
/// A file read is bounded by the same [`forge::portfolio::MAX_EVIDENCE_BYTES`]
/// cap as an inline payload, so a large file cannot pin the CLI.
fn read_evidence_payload(raw: &str) -> Result<String, ForgeError> {
    let trimmed = raw.trim();
    let Some(path) = trimmed.strip_prefix('@') else {
        return Ok(trimmed.to_string());
    };
    let metadata = std::fs::metadata(path).map_err(|err| {
        portfolio_invalid(format!("evidence file `{path}` could not be read: {err}"))
    })?;
    if metadata.len() > forge::portfolio::MAX_EVIDENCE_BYTES as u64 {
        return Err(portfolio_invalid(format!(
            "evidence file `{path}` is larger than {} bytes",
            forge::portfolio::MAX_EVIDENCE_BYTES
        )));
    }
    std::fs::read_to_string(path).map_err(|err| {
        portfolio_invalid(format!("evidence file `{path}` could not be read: {err}"))
    })
}

fn cmd_portfolio(
    db_path: &Path,
    command: &PortfolioCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    match command {
        PortfolioCommands::Show { project } => cmd_portfolio_show(&registry, project, format),
        PortfolioCommands::Tag { command } => cmd_portfolio_tag(&registry, command, format),
        PortfolioCommands::Relation { command } => {
            cmd_portfolio_relation(&registry, command, format)
        }
        PortfolioCommands::Review { command } => cmd_portfolio_review(&registry, command, format),
        PortfolioCommands::Goal { command } => cmd_portfolio_goal(&registry, command, format),
        PortfolioCommands::Evidence { command } => {
            cmd_portfolio_evidence(&registry, command, format)
        }
        PortfolioCommands::Share { command } => cmd_portfolio_share(&registry, command, format),
        PortfolioCommands::Interest { command } => {
            cmd_portfolio_interest(&registry, command, format)
        }
        PortfolioCommands::Activation { command } => {
            cmd_portfolio_activation(&registry, command, format)
        }
    }
}

fn cmd_portfolio_show(
    registry: &Registry,
    project: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let now = chrono::Utc::now();
    let view = registry.portfolio_project_view(project, now)?;
    let human = {
        let mut out = String::new();
        out.push_str(&format!("project: {}\n", view.profile.project_id));
        out.push_str(&format!(
            "lifecycle: {}\n",
            view.profile
                .lifecycle
                .map(|v| v.label().to_string())
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "confidence: {}\n",
            view.profile
                .confidence
                .map(|v| v.label().to_string())
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "next action: {}\n",
            view.profile
                .next_action
                .clone()
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "blocker: {}\n",
            view.profile
                .blocker
                .clone()
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "reviewed at: {}\n",
            view.profile
                .reviewed_at
                .clone()
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "tags: {}\n",
            if view.tags.is_empty() {
                "—".to_string()
            } else {
                view.tags
                    .iter()
                    .map(|t| t.name.clone())
                    .collect::<Vec<String>>()
                    .join(", ")
            }
        ));
        out.push_str(&format!(
            "goals: {}\n",
            if view.goals.is_empty() {
                "—".to_string()
            } else {
                view.goals
                    .iter()
                    .map(|g| format!("{} [{}]", g.title, g.status))
                    .collect::<Vec<String>>()
                    .join(", ")
            }
        ));
        out.push_str("relations:\n");
        if view.relations.is_empty() {
            out.push_str("  —\n");
        }
        for relation in &view.relations {
            out.push_str(&format!(
                "  {} {} {}",
                relation.direction,
                relation.relation_type.label(),
                relation.other_project
            ));
            if let Some(note) = &relation.note {
                out.push_str(&format!(" — {note}"));
            }
            out.push('\n');
        }
        out.push_str("evidence:\n");
        if view.evidence.is_empty() {
            out.push_str("  — no source observation imported\n");
        }
        for snapshot in &view.evidence {
            out.push_str(&format!(
                "  {} {} (source revision {}, observed {}",
                snapshot.source_system,
                snapshot.effective_status.label(),
                snapshot.source_revision,
                snapshot.observed_at
            ));
            if let Some(bound) = &snapshot.stale_after {
                out.push_str(&format!(", stale after {bound}"));
            }
            out.push_str(")\n");
        }
        out.push_str("reviews:\n");
        if view.reviews.is_empty() {
            out.push_str("  —\n");
        }
        for review in &view.reviews {
            out.push_str(&format!(
                "  {} {}",
                review.reviewed_at,
                review.confidence.label()
            ));
            if let Some(note) = &review.note {
                out.push_str(&format!(" — {note}"));
            }
            out.push('\n');
        }
        out
    };
    Ok(as_output(
        format,
        human,
        serde_json::to_value(&view).unwrap(),
    ))
}

fn cmd_portfolio_tag(
    registry: &Registry,
    command: &PortfolioTagCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioTagCommands::Add {
            project,
            name,
            color,
        } => {
            let tag = registry.portfolio_add_tag(project, name, color.as_deref())?;
            let human = format!(
                "tagged {} with {} (tag id {})\n",
                project, tag.name, tag.tag_id
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "tag": tag,
                }),
            ))
        }
        PortfolioTagCommands::Remove { project, name } => {
            let removed = registry.portfolio_remove_tag(project, name)?;
            let human = if removed {
                format!("removed tag {name} from {project}\n")
            } else {
                format!("tag {name} was not attached to {project}; nothing changed\n")
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "tag": name,
                    "removed": removed,
                }),
            ))
        }
        PortfolioTagCommands::List { project } => {
            let all = registry.portfolio_all_tags()?;
            if project.trim().is_empty() {
                let human = if all.is_empty() {
                    "No portfolio tags.".to_string()
                } else {
                    let mut out = String::new();
                    for (tag, projects) in &all {
                        out.push_str(&format!(
                            "{} {}{}\n",
                            tag.name,
                            tag.color.clone().unwrap_or_else(|| "—".to_string()),
                            if projects.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", projects.join(", "))
                            }
                        ));
                    }
                    out
                };
                let json: Vec<serde_json::Value> = all
                    .iter()
                    .map(|(tag, projects)| serde_json::json!({ "tag": tag, "projects": projects }))
                    .collect();
                return Ok(as_output(
                    format,
                    human,
                    serde_json::json!({
                        "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                        "tags": json,
                    }),
                ));
            }
            let tags = registry.portfolio_tags_for(project)?;
            let human = if tags.is_empty() {
                format!("No portfolio tags on {project}.\n")
            } else {
                let mut out = String::new();
                for tag in &tags {
                    out.push_str(&format!(
                        "{} {}\n",
                        tag.name,
                        tag.color.clone().unwrap_or_else(|| "—".to_string())
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "tags": tags,
                }),
            ))
        }
    }
}

fn cmd_portfolio_relation(
    registry: &Registry,
    command: &PortfolioRelationCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioRelationCommands::Add {
            from,
            to,
            r#type,
            note,
        } => {
            let kind = parse_relation_type(r#type)?;
            let relation = registry.portfolio_add_relation(from, to, kind, note.as_deref())?;
            let human = format!(
                "{} {} {} (relation {})\n",
                relation.from_project,
                relation.relation_type.label(),
                relation.to_project,
                relation.relation_id
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "relation": relation,
                }),
            ))
        }
        PortfolioRelationCommands::Remove { from, to, r#type } => {
            let kind = parse_relation_type(r#type)?;
            let removed = registry.portfolio_remove_relation(from, to, kind)?;
            let human = if removed {
                format!("removed {from} {} {to}\n", kind.label())
            } else {
                format!(
                    "{from} {} {to} was not recorded; nothing changed\n",
                    kind.label()
                )
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "from": from,
                    "to": to,
                    "relation_type": kind.label(),
                    "removed": removed,
                }),
            ))
        }
        PortfolioRelationCommands::List { project } => {
            let (human, json) = if project.trim().is_empty() {
                let all = registry.portfolio_all_relations()?;
                let human = if all.is_empty() {
                    "No portfolio relations.".to_string()
                } else {
                    let mut out = String::new();
                    for relation in &all {
                        out.push_str(&format!(
                            "{} {} {} (relation {})\n",
                            relation.from_project,
                            relation.relation_type.label(),
                            relation.to_project,
                            relation.relation_id
                        ));
                    }
                    out
                };
                (
                    human,
                    serde_json::json!({
                        "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                        "relations": all,
                    }),
                )
            } else {
                let relations = registry.portfolio_relations_for(project)?;
                let human = if relations.is_empty() {
                    format!("No portfolio relations on {project}.\n")
                } else {
                    let mut out = String::new();
                    for relation in &relations {
                        out.push_str(&format!(
                            "{} {} {}\n",
                            relation.direction,
                            relation.relation_type.label(),
                            relation.other_project
                        ));
                    }
                    out
                };
                (
                    human,
                    serde_json::json!({
                        "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                        "project_id": project,
                        "relations": relations,
                    }),
                )
            };
            Ok(as_output(format, human, json))
        }
    }
}

fn cmd_portfolio_review(
    registry: &Registry,
    command: &PortfolioReviewCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioReviewCommands::Set {
            project,
            confidence,
            note,
            lifecycle,
            next_action,
            blocker,
        } => {
            let confidence_value = parse_confidence(confidence)?;
            let lifecycle_value = lifecycle.as_deref().map(parse_lifecycle).transpose()?;
            let write = forge::registry::PortfolioWrite {
                lifecycle: lifecycle_value,
                confidence: Some(confidence_value),
                next_action: next_action.clone(),
                blocker: blocker.clone(),
            };
            let profile = registry.portfolio_write(project, &write)?;
            let review =
                registry.portfolio_record_review(project, confidence_value, note.as_deref())?;
            let human = format!(
                "reviewed {project}: confidence {}, lifecycle {}, next action {}, blocker {}\n",
                confidence_value.label(),
                lifecycle_value.map(|v| v.label()).unwrap_or("unchanged"),
                profile
                    .next_action
                    .clone()
                    .unwrap_or_else(|| "—".to_string()),
                profile.blocker.clone().unwrap_or_else(|| "—".to_string()),
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "profile": profile,
                    "review": review,
                }),
            ))
        }
        PortfolioReviewCommands::List { project } => {
            let reviews = registry.portfolio_reviews_for(project, 50)?;
            let human = if reviews.is_empty() {
                format!("No portfolio reviews for {project}.\n")
            } else {
                let mut out = String::new();
                for review in &reviews {
                    out.push_str(&format!(
                        "{} {}",
                        review.reviewed_at,
                        review.confidence.label()
                    ));
                    if let Some(note) = &review.note {
                        out.push_str(&format!(" — {note}"));
                    }
                    out.push('\n');
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "reviews": reviews,
                }),
            ))
        }
    }
}

fn cmd_portfolio_goal(
    registry: &Registry,
    command: &PortfolioGoalCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioGoalCommands::Add {
            title,
            status,
            description,
        } => {
            let goal = registry.portfolio_add_goal(title, status, description.as_deref())?;
            let human = format!(
                "goal {} [{}] (goal {})\n",
                goal.title, goal.status, goal.goal_id
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "goal": goal,
                }),
            ))
        }
        PortfolioGoalCommands::Link { goal, project } => {
            let linked = registry.portfolio_link_goal(goal, project)?;
            let human = format!(
                "linked {} to {} ({})\n",
                linked.projects.join(", "),
                linked.title,
                linked.status
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "goal": linked,
                }),
            ))
        }
        PortfolioGoalCommands::List => {
            let goals = registry.portfolio_goals()?;
            let human = if goals.is_empty() {
                "No portfolio goals.".to_string()
            } else {
                let mut out = String::new();
                for goal in &goals {
                    out.push_str(&format!(
                        "{} [{}] {}\n",
                        goal.title,
                        goal.status,
                        if goal.projects.is_empty() {
                            "—".to_string()
                        } else {
                            goal.projects.join(", ")
                        }
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "goals": goals,
                }),
            ))
        }
    }
}

fn cmd_portfolio_evidence(
    registry: &Registry,
    command: &PortfolioEvidenceCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioEvidenceCommands::Import {
            project,
            source,
            revision,
            status,
            observed_at,
            stale_after,
            evidence,
        } => {
            let status_value = parse_evidence_status(status)?;
            let payload = read_evidence_payload(evidence)?;
            let write = forge::registry::SnapshotWrite {
                source_system: source.clone(),
                source_revision: revision.clone(),
                observed_at: observed_at
                    .clone()
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                status: status_value,
                stale_after: stale_after.clone(),
                evidence_json: payload,
            };
            let snapshot = registry.portfolio_import_snapshot(project, &write)?;
            let human = format!(
                "imported {} snapshot {} for {} (source revision {})\n",
                snapshot.status.label(),
                snapshot.snapshot_id,
                project,
                snapshot.source_revision
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "snapshot": snapshot,
                }),
            ))
        }
        PortfolioEvidenceCommands::List { project } => {
            let now = chrono::Utc::now();
            let mut snapshots = registry.portfolio_snapshots_for(project, 100)?;
            for snapshot in &mut snapshots {
                snapshot.effective_status =
                    forge::portfolio::snapshot_effective_status(snapshot, now);
            }
            let human = if snapshots.is_empty() {
                format!("No portfolio evidence snapshots for {project}.\n")
            } else {
                let mut out = String::new();
                for snapshot in &snapshots {
                    out.push_str(&format!(
                        "{} {} {} {} (source revision {})\n",
                        snapshot.observed_at,
                        snapshot.source_system,
                        snapshot.status.label(),
                        snapshot.effective_status.label(),
                        snapshot.source_revision
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "generated_at": now.to_rfc3339(),
                    "snapshots": snapshots,
                }),
            ))
        }
    }
}

fn share_invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioShareInvalid { reason }
}

/// The one CLI entry point for the public share allowlist. Every
/// subcommand dispatches into the same
/// [`forge::portfolio::publication`] orchestration the JSON API uses,
/// so the terminal adds no publication rule of its own.
fn cmd_portfolio_share(
    registry: &Registry,
    command: &PortfolioShareCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::portfolio::share::{
        self as share, ShowcaseStatus, Visibility, SHARE_CONTRACT_VERSION,
    };
    let now = chrono::Utc::now();
    let contract = SHARE_CONTRACT_VERSION;
    match command {
        PortfolioShareCommands::Set {
            project,
            title,
            summary,
            category,
            source_url,
            demo_url,
            visibility,
            featured,
            status,
            evidence,
            surfaces,
        } => {
            let visibility = Visibility::parse(visibility).map_err(share_invalid)?;
            let showcase_status = ShowcaseStatus::parse(status).map_err(share_invalid)?;
            let mut parsed = Vec::with_capacity(surfaces.len());
            for surface in surfaces {
                parsed.push(share::ShareSurface::split(surface).map_err(share_invalid)?);
            }
            let write = forge::portfolio::share::ShareWrite {
                title: title.clone(),
                summary: summary.clone(),
                category: category.clone(),
                source_url: source_url.clone(),
                demo_url: demo_url.clone(),
                visibility,
                featured: *featured,
                showcase_status,
                status_evidence: match evidence {
                    Some(raw) => Some(read_evidence_payload(raw)?),
                    None => None,
                },
                surfaces: parsed,
            };
            let record = registry.share_upsert_record(project, &write)?;
            let human = format!(
                "share record for {} is {} at revision {} with {} public surface(s)\n",
                record.project_id,
                record.state.label(),
                record.revision,
                record.surfaces.len()
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "project_id": project,
                    "share": record,
                }),
            ))
        }
        PortfolioShareCommands::Remove { project } => {
            let removed = registry.share_remove_record(project)?;
            let human = if removed {
                format!("withdrew the share record for {project}\n")
            } else {
                format!("{project} had no share record to withdraw\n")
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "project_id": project,
                    "shared": false,
                    "removed": removed,
                }),
            ))
        }
        PortfolioShareCommands::Show { project } => {
            let record = registry.share_record(project)?;
            let findings = registry.share_findings(20)?;
            let findings: Vec<_> = findings
                .into_iter()
                .filter(|finding| finding.project_id == *project)
                .collect();
            let human = match &record {
                None => {
                    format!("{project} has no share record; it is absent from the public catalog\n")
                }
                Some(record) => {
                    let mut out = String::new();
                    out.push_str(&format!("project: {}\n", record.project_id));
                    out.push_str(&format!("state: {}\n", record.state.label()));
                    out.push_str(&format!("revision: {}\n", record.revision));
                    out.push_str(&format!("title: {}\n", record.title));
                    out.push_str(&format!("category: {}\n", record.category));
                    out.push_str(&format!("source url: {}\n", record.source_url));
                    out.push_str(&format!(
                        "demo url: {}\n",
                        record.demo_url.clone().unwrap_or_else(|| "—".to_string())
                    ));
                    out.push_str(&format!("visibility: {}\n", record.visibility));
                    out.push_str(&format!("showcase status: {}\n", record.showcase_status));
                    out.push_str(&format!("featured: {}\n", record.featured));
                    out.push_str(&format!(
                        "status evidence: {}\n",
                        record
                            .status_evidence
                            .clone()
                            .unwrap_or_else(|| "—".to_string())
                    ));
                    if record.surfaces.is_empty() {
                        out.push_str("surfaces: none\n");
                    } else {
                        out.push_str("surfaces:\n");
                        for surface in &record.surfaces {
                            out.push_str(&format!("  {} {}\n", surface.label, surface.url));
                        }
                    }
                    out
                }
            };
            // The persisted refusals are the reason a record is not
            // public; an operator reading `show` needs them, not just
            // the machine projection.
            let human = if findings.is_empty() {
                human
            } else {
                let mut out = human;
                out.push_str("findings:\n");
                for finding in &findings {
                    out.push_str(&format!(
                        "  {} {} [{}] {}\n",
                        finding.project_id, finding.field, finding.code, finding.detail
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "project_id": project,
                    "shared": record.is_some(),
                    "share": record,
                    "findings": findings,
                }),
            ))
        }
        PortfolioShareCommands::List { project } => {
            let records = if project.is_empty() {
                registry.share_records()?
            } else {
                registry.share_record(project)?.into_iter().collect()
            };
            let human = if records.is_empty() {
                "No share records.\n".to_string()
            } else {
                let mut out = String::new();
                for record in &records {
                    out.push_str(&format!(
                        "{} {} {} {} {}\n",
                        record.project_id,
                        record.state.label(),
                        record.visibility,
                        record.showcase_status,
                        record.title
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "records": records,
                }),
            ))
        }
        PortfolioShareCommands::Preview { document } => {
            let draft = forge::portfolio::publication::preview_manifest(registry)?;
            let mut human = format!(
                "manifest revision {}\nproject count: {}\nmanifest sha256: {}\napprovable: {}\n",
                draft.body.manifest_revision,
                draft.project_count(),
                draft.manifest_sha256(),
                draft.approvable()
            );
            if draft.findings.is_empty() {
                human.push_str("findings: none\n");
            } else {
                human.push_str("findings:\n");
                for finding in &draft.findings {
                    human.push_str(&format!(
                        "  {} {} [{}] {}\n",
                        finding.project_id, finding.field, finding.code, finding.detail
                    ));
                }
            }
            if *document {
                human.push_str(&draft.document(&now.to_rfc3339()));
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "preview": {
                        "manifest_revision": draft.body.manifest_revision,
                        "manifest_sha256": draft.manifest_sha256(),
                        "project_count": draft.project_count(),
                        "approvable": draft.approvable(),
                        "canonical_body": draft.body.canonical_json(),
                        "document": draft.document(&now.to_rfc3339()),
                        "findings": draft.findings,
                    },
                }),
            ))
        }
        PortfolioShareCommands::Approve { hash, actor } => {
            let approval = registry.share_approve(hash, actor)?;
            let human = format!(
                "approved manifest revision {} ({}) with {} project(s) as {}\n",
                approval.revision, approval.manifest_sha256, approval.project_count, approval.actor
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "approval": approval,
                }),
            ))
        }
        PortfolioShareCommands::Publish {
            target,
            operation_key,
            actor,
            adapter,
        } => {
            let plan = forge::portfolio::publication::PublishPlan {
                operation_key: operation_key.clone(),
                target: target.clone(),
                actor: actor.clone(),
                adapter: adapter.clone(),
            };
            let report =
                forge::portfolio::publication::publish_approved_manifest(registry, &plan, now)?;
            let human = format!(
                "publication {} of manifest revision {} ({} project(s)) via {} is {}{}\n",
                report.publication_id,
                report.manifest_revision,
                report.project_count,
                report.publisher,
                report.status_label(),
                if report.already_present {
                    " (already present; no second publication)"
                } else {
                    ""
                }
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "publication": report,
                }),
            ))
        }
        PortfolioShareCommands::Reconcile {
            publication,
            result,
            actor,
        } => {
            let status = share::PublicationStatus::parse(result).map_err(share_invalid)?;
            let attempt = registry.share_reconcile_publication(*publication, status, actor)?;
            let human = format!(
                "publication {} is now {} after reconciliation by {}\n",
                attempt.publication_id, attempt.status, attempt.actor
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "publication": attempt,
                }),
            ))
        }
        PortfolioShareCommands::Audit { limit } => {
            if *limit == 0 || *limit > 500 {
                return Err(share_invalid("limit must be between 1 and 500".to_string()));
            }
            let approvals = registry.share_approvals(*limit)?;
            let publications = registry.share_publications(*limit)?;
            let unreconciled = registry.share_unreconciled_publication()?;
            let human = {
                let mut out = String::new();
                out.push_str("approvals:\n");
                if approvals.is_empty() {
                    out.push_str("  none\n");
                }
                for approval in &approvals {
                    out.push_str(&format!(
                        "  revision {} {} {} {} by {}\n",
                        approval.revision,
                        approval.state,
                        approval.manifest_sha256,
                        format!("{} project(s)", approval.project_count),
                        approval.actor
                    ));
                }
                out.push_str("publications:\n");
                if publications.is_empty() {
                    out.push_str("  none\n");
                }
                for attempt in &publications {
                    out.push_str(&format!(
                        "  #{} {} revision {} {} target {}{}\n",
                        attempt.publication_id,
                        attempt.status,
                        attempt.manifest_revision,
                        attempt.manifest_sha256,
                        attempt.target,
                        attempt
                            .error_code
                            .clone()
                            .map(|code| format!(" [{code}]"))
                            .unwrap_or_default()
                    ));
                }
                out.push_str(&format!(
                    "unreconciled: {}\n",
                    match &unreconciled {
                        Some(attempt) => format!("#{}", attempt.publication_id),
                        None => "none".to_string(),
                    }
                ));
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "approvals": approvals,
                    "publications": publications,
                    "unreconciled": unreconciled,
                }),
            ))
        }
    }
}

fn interest_invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioInterestInvalid { reason }
}

/// The one CLI entry point for aggregate interest evidence. Every
/// subcommand dispatches into the same
/// [`forge::portfolio::interest_report`] orchestration the JSON API
/// uses, so the terminal adds no import or comparison rule of its own.
fn cmd_portfolio_interest(
    registry: &Registry,
    command: &PortfolioInterestCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::portfolio::interest::{
        self as interest, InterestMetric, RawSnapshot, INTEREST_CONTRACT_VERSION,
    };
    use forge::portfolio::interest_report as report;
    let now = chrono::Utc::now();
    let contract = INTEREST_CONTRACT_VERSION;
    match command {
        PortfolioInterestCommands::Import { file, actor } => {
            let raw = read_import_document(file)?;
            let decoded = interest::parse_import_document(&raw).map_err(interest_invalid)?;
            let records: Vec<RawSnapshot> = decoded
                .into_iter()
                .map(|(index, record)| RawSnapshot { index, record })
                .collect();
            let imported = report::import_snapshots(
                registry,
                &interest::InterestImport { records },
                actor,
                now,
            )?;
            // A batch where every record was refused is a failed
            // import, and saying so through the exit code is what lets
            // a script notice without parsing the report.
            let human = format!(
                "imported {} snapshot(s): {} accepted, {} already present, {} superseded, {} refused\n",
                imported.received,
                imported.accepted.len(),
                imported.already_present.len(),
                imported.supersessions.len(),
                imported.refused()
            );
            let json = serde_json::json!({
                "contract": contract,
                "generated_at": now.to_rfc3339(),
                "import": imported,
            });
            let rendered = as_output(format, human, json);
            if imported.is_complete_failure() {
                // A batch where every record was refused is a failed
                // import, and the reason is the diagnosis: naming each
                // refused record and its code is what lets an importer
                // fix the batch without re-running the CLI to find out
                // which of fifty records was wrong.
                return Err(interest_invalid(format!(
                    "every snapshot in the batch was refused: {}",
                    summarize_refusals(&imported.rejected)
                )));
            }
            Ok(rendered)
        }
        PortfolioInterestCommands::List {
            project,
            limit,
            stale_after_days,
        } => {
            if *limit == 0 || *limit > 500 {
                return Err(interest_invalid(
                    "limit must be between 1 and 500".to_string(),
                ));
            }
            let snapshots: Vec<forge::portfolio::interest::InterestSnapshot> = if project.is_empty()
            {
                let mut all = Vec::new();
                for record in registry.list()? {
                    all.extend(registry.interest_snapshots(&record.id, *limit)?);
                }
                all
            } else {
                registry.interest_snapshots(project, *limit)?
            };
            let human = if snapshots.is_empty() {
                "No interest snapshots.\n".to_string()
            } else {
                let mut out = String::new();
                for snapshot in &snapshots {
                    out.push_str(&format!(
                        "{} {} {} {}..{} {} {} {}\n",
                        snapshot.project_id,
                        snapshot.source,
                        snapshot.source_revision,
                        snapshot.window_start,
                        snapshot.window_end,
                        snapshot.state.label(),
                        snapshot.privacy_mode,
                        snapshot.freshness(now, *stale_after_days).label(),
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "stale_after_days": stale_after_days,
                    "snapshots": snapshots,
                }),
            ))
        }
        PortfolioInterestCommands::Show {
            project,
            stale_after_days,
        } => {
            let projection = report::project_interest(registry, project, *stale_after_days, now)?;
            let human = format!(
                "project: {}\nstale after: {} day(s)\nstale windows: {}\nsuperseded revisions: {}\n",
                projection.project_id,
                projection.stale_after_days,
                projection.stale_snapshots,
                projection.superseded_snapshots
            );
            let mut human = human;
            if projection.snapshots.is_empty() {
                human.push_str("snapshots: none\n");
            } else {
                human.push_str("snapshots:\n");
                for snapshot in &projection.snapshots {
                    human.push_str(&format!(
                        "  {} {}..{} {} {} {}\n",
                        snapshot.source,
                        snapshot.window_start,
                        snapshot.window_end,
                        snapshot.source_revision,
                        snapshot.state.label(),
                        snapshot.freshness(now, projection.stale_after_days).label(),
                    ));
                    for value in &snapshot.metrics {
                        human.push_str(&format!("    {} = {}\n", value.metric, value.value));
                    }
                }
            }
            if !projection.findings.is_empty() {
                human.push_str("refusals:\n");
                for finding in &projection.findings {
                    human.push_str(&format!(
                        "  {} [{}] {}\n",
                        finding.field, finding.code, finding.detail
                    ));
                }
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "interest": projection,
                }),
            ))
        }
        PortfolioInterestCommands::Compare {
            projects,
            metrics,
            source,
            stale_after_days,
        } => {
            let selected = parse_interest_metrics(metrics)?;
            let comparison = report::compare_projects(
                registry,
                projects,
                &selected,
                source.as_deref(),
                *stale_after_days,
                now,
            )?;
            let human = render_comparison(&comparison);
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "comparison": comparison,
                }),
            ))
        }
        PortfolioInterestCommands::Trend {
            project,
            metric,
            limit,
            stale_after_days,
        } => {
            let metric = InterestMetric::parse(metric.trim()).map_err(interest_invalid)?;
            let trend =
                report::interest_trend(registry, project, metric, *limit, *stale_after_days, now)?;
            let human = format!(
                "{} {}\n{} window(s) plotted, {} window(s) reported no such metric\nnote: {}\n",
                trend.project_id,
                trend.metric,
                trend.points.len(),
                trend.unreported_windows,
                trend.note
            );
            let mut human = human;
            for point in &trend.points {
                human.push_str(&format!(
                    "  {}..{} {} {} {}\n",
                    point.window_start,
                    point.window_end,
                    point.value,
                    point.privacy_mode,
                    point.freshness
                ));
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "trend": trend,
                }),
            ))
        }
        PortfolioInterestCommands::Audit { limit } => {
            if *limit == 0 || *limit > 500 {
                return Err(interest_invalid(
                    "limit must be between 1 and 500".to_string(),
                ));
            }
            let findings = registry.interest_findings(*limit)?;
            let human = if findings.is_empty() {
                "No refused interest snapshots.\n".to_string()
            } else {
                let mut out = String::new();
                for finding in &findings {
                    out.push_str(&format!(
                        "  {} {} [{}] {}\n",
                        finding.project_id, finding.field, finding.code, finding.detail
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "refusals": findings,
                }),
            ))
        }
    }
}

/// The one CLI entry point for activation readiness. Mirrors
/// `cmd_fleet_online` exactly: build the report, print it, then return
/// the typed gate error when any evaluated project is `not-ready`.
/// Input errors are typed `portfolio-interest-invalid` refusals with
/// empty stdout; only a verdict report may print on stdout with a
/// non-zero exit.
fn cmd_portfolio_activation(
    registry: &Registry,
    command: &PortfolioActivationCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::portfolio::interest::{self as interest, InterestMetric};
    use forge::portfolio::interest_report as report;
    let now = chrono::Utc::now();
    let contract = interest::ACTIVATION_CONTRACT_VERSION;
    match command {
        PortfolioActivationCommands::Readiness {
            project,
            metric,
            min_value,
            source,
            window,
            stale_after_days,
        } => {
            let metric = InterestMetric::parse(metric.trim()).map_err(interest_invalid)?;
            let threshold = match min_value {
                Some(value) => {
                    Some(interest::validate_threshold(*value).map_err(interest_invalid)?)
                }
                None => None,
            };
            let source = match source {
                Some(raw) if raw.trim().is_empty() => {
                    return Err(interest_invalid("source must not be blank".to_string()));
                }
                Some(raw) => Some(raw.trim().to_string()),
                None => None,
            };
            let parsed_window = match window {
                Some(raw) => Some(interest::parse_window(raw.trim()).map_err(interest_invalid)?),
                None => None,
            };
            let window_pair = parsed_window
                .as_ref()
                .map(|(start, end)| (start.as_str(), end.as_str()));
            let projects: Vec<String> = if project.trim().is_empty() {
                let records = registry.list()?;
                if records.is_empty() {
                    return Err(interest_invalid(
                        "readiness needs at least one project to evaluate".to_string(),
                    ));
                }
                records.into_iter().map(|record| record.id).collect()
            } else {
                vec![project.trim().to_string()]
            };
            let activation = report::activation_readiness(
                registry,
                &projects,
                metric,
                threshold,
                source.as_deref(),
                window_pair,
                *stale_after_days,
                now,
            )?;
            let human = render_activation(&activation);
            let json = serde_json::json!({
                "contract": contract,
                "generated_at": now.to_rfc3339(),
                "activation": {
                    "metric": activation.metric,
                    "threshold": activation.threshold,
                    "stale_after_days": activation.stale_after_days,
                    "requested_window": activation.requested_window,
                    "requested_source": activation.requested_source,
                    "ready": activation.is_ready(),
                    "ready_count": activation.ready_count,
                    "not_ready_count": activation.not_ready_count,
                    "verdicts": activation.verdicts,
                },
            });
            let output = as_output(format, human, json);
            if !activation.is_ready() {
                match &output {
                    Output::Human(text) => println!("{text}"),
                    Output::Json(value) => {
                        println!("{}", serde_json::to_string_pretty(value).unwrap());
                    }
                    Output::Raw(text) => print!("{text}"),
                }
                return Err(ForgeError::PortfolioActivationNotReady {
                    reason: format!(
                        "{} of {} project(s) are not ready ({})",
                        activation.not_ready_count,
                        activation.verdicts.len(),
                        activation.verdict_reason_labels().join(", "),
                    ),
                });
            }
            Ok(output)
        }
    }
}

/// Parse the `--window <START>..<END>` flag shared by the activation
/// readiness command. Both sides normalize through the interest
/// timestamp rule so an offset form matches stored UTC.
#[allow(dead_code)]
fn parse_activation_window(raw: &str) -> Result<(String, String), ForgeError> {
    forge::portfolio::interest::parse_window(raw.trim()).map_err(interest_invalid)
}

/// Render a readiness report: one header line, one block per verdict
/// in verdict order, then the summary.
fn render_activation(activation: &forge::portfolio::interest::ActivationReport) -> String {
    let mut out = String::new();
    let threshold = activation
        .threshold
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string());
    out.push_str(&format!(
        "activation readiness ({}): metric={} threshold={} stale_after_days={}\n",
        forge::portfolio::interest::ACTIVATION_CONTRACT_VERSION,
        activation.metric,
        threshold,
        activation.stale_after_days,
    ));
    for verdict in &activation.verdicts {
        if let Some(evidence) = &verdict.evidence {
            out.push_str(&format!(
                "  {} {}      window={}..{} source={} revision={} privacy={} coverage={} freshness={} value={}\n",
                verdict.project_id,
                verdict.readiness.label(),
                evidence.window_start,
                evidence.window_end,
                evidence.source,
                evidence.source_revision,
                evidence.privacy_mode,
                evidence.coverage,
                evidence.freshness,
                evidence.value,
            ));
        } else {
            out.push_str(&format!(
                "  {} {}\n",
                verdict.project_id,
                verdict.readiness.label(),
            ));
        }
        for reason in &verdict.reasons {
            out.push_str(&format!(
                "      {}: {}\n",
                reason.reason.label(),
                reason.detail
            ));
        }
        for note in &verdict.notes {
            out.push_str(&format!("    {note}\n"));
        }
    }
    out.push_str(&format!(
        "summary: ready={} not-ready={}\n",
        activation.ready_count, activation.not_ready_count
    ));
    out
}

/// Summarize a fully refused batch into one refusal message.
///
/// The message names each refused record's index, its stable code and
/// its reason — so a one-record batch is diagnosable without a second
/// command, and a five-hundred record batch cannot produce a
/// five-hundred line error. Each detail has already been scrubbed by
/// the domain, so no offending value can reach the message.
fn summarize_refusals(rejected: &[forge::portfolio::interest::InterestRejection]) -> String {
    const MAX_LISTED: usize = 5;
    const MAX_DETAIL_CHARS: usize = 200;
    let listed: Vec<String> = rejected
        .iter()
        .take(MAX_LISTED)
        .map(|rejection| {
            let detail: String = rejection.detail.chars().take(MAX_DETAIL_CHARS).collect();
            let ellipsis = if rejection.detail.chars().count() > MAX_DETAIL_CHARS {
                "…"
            } else {
                ""
            };
            format!(
                "record {} [{}]: {detail}{ellipsis}",
                rejection.index, rejection.code
            )
        })
        .collect();
    let mut summary = listed.join("; ");
    if rejected.len() > MAX_LISTED {
        summary.push_str(&format!(
            "; and {} more, see `forge portfolio interest audit`",
            rejected.len() - MAX_LISTED
        ));
    }
    summary
}

/// Read an import document from a path or from stdin, refusing
/// anything past the size bound before it is parsed.
fn read_import_document(file: &str) -> Result<String, ForgeError> {
    const MAX: usize = forge::portfolio::interest::MAX_IMPORT_BYTES;
    let trimmed = file.trim();
    if trimmed == "-" {
        // Read one byte past the bound so an oversized document is
        // detected rather than silently truncated into a parse error
        // that would read like malformed JSON.
        let mut reader = std::io::Read::take(std::io::stdin().lock(), MAX as u64 + 1);
        let mut raw = String::new();
        std::io::Read::read_to_string(&mut reader, &mut raw)
            .map_err(|err| interest_invalid(format!("import document could not be read: {err}")))?;
        if raw.len() > MAX {
            return Err(interest_invalid(format!(
                "import document is larger than {MAX} bytes"
            )));
        }
        return Ok(raw);
    }
    let metadata = std::fs::metadata(trimmed).map_err(|err| {
        interest_invalid(format!("import document `{trimmed}` is unreadable: {err}"))
    })?;
    if metadata.len() > MAX as u64 {
        return Err(interest_invalid(format!(
            "import document is larger than {MAX} bytes"
        )));
    }
    std::fs::read_to_string(trimmed).map_err(|err| {
        interest_invalid(format!("import document `{trimmed}` is unreadable: {err}"))
    })
}

/// Resolve the requested metrics, defaulting to the whole allowlist.
///
/// An unknown metric is a typed refusal rather than a silent skip: an
/// importer that asked for a metric Forge does not carry should learn
/// that, not see a comparison quietly missing a column.
fn parse_interest_metrics(
    raw: &[String],
) -> Result<Vec<forge::portfolio::interest::InterestMetric>, ForgeError> {
    use forge::portfolio::interest::InterestMetric;
    if raw.is_empty() {
        return Ok(InterestMetric::ALL.to_vec());
    }
    let mut selected = Vec::with_capacity(raw.len());
    for value in raw {
        let metric = InterestMetric::parse(value.trim()).map_err(interest_invalid)?;
        if !selected.contains(&metric) {
            selected.push(metric);
        }
    }
    Ok(selected)
}

/// Render a comparison as a window-labelled table.
///
/// There is deliberately no total row and no ordering column: the
/// honest answer to "which project is more interesting" is a set of
/// labelled figures, and Forge will not manufacture a ranking the
/// windows do not support.
fn render_comparison(comparison: &forge::portfolio::interest::Comparison) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "comparable across windows: {}\nrows: {}\n",
        comparison.comparable,
        comparison.rows.len()
    ));
    for window in &comparison.windows {
        out.push_str(&format!("window: {window}\n"));
    }
    if comparison.rows.is_empty() {
        out.push_str("figures: none\n");
    } else {
        out.push_str("figures:\n");
        for row in &comparison.rows {
            out.push_str(&format!(
                "  {} {} = {} [{} / {} / {} / {}]\n",
                row.project_id,
                row.metric,
                row.value,
                row.source,
                row.source_revision,
                row.privacy_mode,
                row.freshness
            ));
        }
    }
    if !comparison.notes.is_empty() {
        out.push_str("notes:\n");
        for note in &comparison.notes {
            out.push_str(&format!("  {note}\n"));
        }
    }
    out
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
                    Format::Human | Format::Table => human,
                    Format::Json => serde_json::to_string_pretty(&json).unwrap(),
                    Format::Ndjson => serde_json::to_string(&json).unwrap(),
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

fn cmd_project(
    db_path: &Path,
    command: &ProjectCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ProjectCommands::Gaps {
            project,
            filters,
            categories,
            statuses,
            remediation_classes,
        } => cmd_project_gaps(
            db_path,
            project.as_deref(),
            filters,
            categories,
            statuses,
            remediation_classes,
            format,
        ),
        ProjectCommands::List { filters } => cmd_project_list(db_path, filters, format),
        ProjectCommands::Inspect { project, filters } => {
            cmd_project_inspect(db_path, project, filters, format)
        }
        ProjectCommands::Tags { filters } => cmd_project_tags(db_path, filters, format),
        ProjectCommands::Languages { filters } => cmd_project_languages(db_path, filters, format),
        ProjectCommands::Github { command } => cmd_project_github(db_path, command, format),
    }
}

fn cmd_project_list(
    db_path: &Path,
    args: &CatalogFilterArgs,
    format: Format,
) -> Result<Output, ForgeError> {
    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });
    let pairs = catalog_filter_pairs(args);
    let query = CatalogQuery::from_pairs(&pairs, args.limit, args.cursor.clone())?.normalize();
    let mut page = catalog::apply(&bundle.records, &query, &bundle.observed_at)?;
    page.sources = bundle.statuses.clone();
    catalog_page_output(page, format)
}

fn cmd_project_inspect(
    db_path: &Path,
    project: &str,
    args: &CatalogFilterArgs,
    format: Format,
) -> Result<Output, ForgeError> {
    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });
    let records = catalog::inspect_records(&bundle, project)?;
    catalog_records_output(project, &records, format)
}

fn cmd_project_tags(
    db_path: &Path,
    args: &CatalogFilterArgs,
    format: Format,
) -> Result<Output, ForgeError> {
    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });
    let pairs = catalog_filter_pairs(args);
    let query = CatalogQuery::from_pairs(&pairs, args.limit, args.cursor.clone())?.normalize();
    let filtered = catalog::filter(&bundle.records, &query);
    let counts = catalog::tag_counts(&filtered);
    catalog_counts_output("tags", &counts, format)
}

fn cmd_project_languages(
    db_path: &Path,
    args: &CatalogFilterArgs,
    format: Format,
) -> Result<Output, ForgeError> {
    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });
    let pairs = catalog_filter_pairs(args);
    let query = CatalogQuery::from_pairs(&pairs, args.limit, args.cursor.clone())?.normalize();
    let filtered = catalog::filter(&bundle.records, &query);
    let counts = catalog::language_counts(&filtered);
    catalog_counts_output("languages", &counts, format)
}

fn cmd_project_github(
    db_path: &Path,
    command: &GithubCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        GithubCommands::Observe { repositories, host } => {
            cmd_github_observe(db_path, repositories, host, format)
        }
        GithubCommands::Propose {
            repository,
            host,
            mode,
            confirm,
            sets,
        } => cmd_github_propose(repository, host, mode, confirm.as_deref(), sets, format),
        GithubCommands::Auth { host } => cmd_github_cli_auth(host, format),
        GithubCommands::Clone {
            repository,
            destination,
            confirm,
        } => cmd_github_cli_clone(db_path, repository, destination, *confirm, format),
        GithubCommands::Create {
            project,
            repo,
            visibility,
            confirm_public,
            push_source,
            confirm,
        } => cmd_github_cli_create(
            db_path,
            project,
            repo,
            visibility,
            *confirm_public,
            *push_source,
            *confirm,
            format,
        ),
        GithubCommands::PullRequest {
            project,
            title,
            body,
            draft,
            confirm,
        } => cmd_github_cli_pull_request(db_path, project, title, body, *draft, *confirm, format),
    }
}

fn cmd_github_observe(
    db_path: &Path,
    repositories: &[String],
    host: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{
        normalize_observation, GithubAdapter, GithubObservationRequest, GITHUB_CONTRACT_VERSION,
    };
    let adapter = GithubAdapter::from_env();
    if !adapter.binary_available() {
        return Err(ForgeError::GithubAdapterUnavailable {
            reason: format!(
                "no GitHub adapter binary is configured ({})",
                adapter.source
            ),
        });
    }
    if adapter.token.is_none() {
        return Err(ForgeError::GithubInvalid {
            reason: format!(
                "{} is not set; refusing to send an unauthenticated observation request",
                forge::github::GITHUB_TOKEN_ENV
            ),
        });
    }
    let explicit = repositories
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let selection_repos = if explicit.is_empty() {
        github_repositories_from_registry(db_path)?
    } else {
        explicit
    };
    if selection_repos.is_empty() {
        return Err(ForgeError::GithubInvalid {
            reason: "no repositories to observe; pass `owner/repo` arguments or register a \
                     project whose git_remote points to github.com"
                .to_string(),
        });
    }
    let request = GithubObservationRequest {
        host: host.to_string(),
        repositories: selection_repos,
    };
    let observations = adapter.observe(&request)?;
    let now = chrono::Utc::now();
    let max_age = forge::catalog::DEFAULT_MAX_AGE_SECONDS;
    let records: Vec<forge::catalog::CatalogRecord> = observations
        .iter()
        .map(|observation| normalize_observation(observation, max_age, now))
        .collect();
    let human = render_github_observe_human(host, &observations, &records);
    let json = serde_json::json!({
        "contract": GITHUB_CONTRACT_VERSION,
        "host": host,
        "adapter_source": adapter.source,
        "observations": observations,
        "records": records,
    });
    Ok(as_output(format, human, json))
}

fn cmd_github_propose(
    repository: &str,
    host: &str,
    mode: &str,
    confirm: Option<&str>,
    sets: &[String],
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{
        GithubAdapter, MutationMode, ProposeRequest, ProposedChange, GITHUB_CONTRACT_VERSION,
    };
    let adapter = GithubAdapter::from_env();
    if !adapter.binary_available() {
        return Err(ForgeError::GithubAdapterUnavailable {
            reason: format!(
                "no GitHub adapter binary is configured ({})",
                adapter.source
            ),
        });
    }
    if adapter.token.is_none() {
        return Err(ForgeError::GithubInvalid {
            reason: format!(
                "{} is not set; refusing to mutate without authentication",
                forge::github::GITHUB_TOKEN_ENV
            ),
        });
    }
    let mode_trimmed = mode.trim();
    let mutation_mode = match mode_trimmed {
        "pull-request" | "pr" | "" => MutationMode::PullRequest,
        "direct" => match confirm {
            Some(token) if !token.trim().is_empty() => MutationMode::Direct {
                confirmation: token.trim().to_string(),
            },
            _ => {
                return Err(ForgeError::GithubInvalid {
                    reason: "direct mode requires a non-empty --confirm token; refusing the \
                             mutation"
                        .to_string(),
                });
            }
        },
        other => {
            return Err(ForgeError::GithubInvalid {
                reason: format!(
                    "unknown --mode `{other}`; expected `pull-request` (default) or `direct`"
                ),
            });
        }
    };
    let mut changes: Vec<ProposedChange> = Vec::new();
    for raw in sets {
        let (field, value) = raw
            .split_once('=')
            .ok_or_else(|| ForgeError::GithubInvalid {
                reason: format!("proposed change `{raw}` is not in `field=value` form"),
            })?;
        changes.push(ProposedChange {
            field: field.trim().to_string(),
            new_value: value.trim().to_string(),
        });
    }
    let request = ProposeRequest {
        host: host.to_string(),
        repository: repository.to_string(),
        mode: mutation_mode,
        changes,
    };
    let outcome = adapter.propose(&request)?;
    let human = render_github_propose_human(&outcome);
    let json = serde_json::json!({
        "contract": GITHUB_CONTRACT_VERSION,
        "host": host,
        "repository": repository,
        "adapter_source": adapter.source,
        "outcome": outcome,
    });
    Ok(as_output(format, human, json))
}

fn github_repositories_from_registry(db_path: &Path) -> Result<Vec<String>, ForgeError> {
    if !db_path.is_file() {
        return Ok(Vec::new());
    }
    let registry = forge::registry::Registry::open_read_only(db_path)?;
    let projects = registry.list()?;
    let mut out: Vec<String> = Vec::new();
    for project in projects {
        let Some(remote) = project.git_remote.as_deref() else {
            continue;
        };
        if let Some(repository) = forge::catalog::source::github_repository_from_remote(remote) {
            if !out.contains(&repository) {
                out.push(repository);
            }
        }
    }
    Ok(out)
}

fn render_github_observe_human(
    host: &str,
    observations: &[forge::github::GithubObservation],
    records: &[forge::catalog::CatalogRecord],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "forge project github observe — contract {} host={}\n",
        forge::github::GITHUB_CONTRACT_VERSION,
        host
    ));
    out.push_str(&format!("observations: {}\n", observations.len()));
    for observation in observations {
        out.push_str(&format!(
            "  repository={} state={} observed_at={} revision={}\n",
            observation.repository,
            observation.state.id(),
            observation.observed_at,
            observation.source_revision.as_deref().unwrap_or("unknown"),
        ));
        if let Some(description) = observation.description.as_deref() {
            out.push_str(&format!("    description: {description}\n"));
        }
        if !observation.topics.is_empty() {
            out.push_str(&format!("    topics: {}\n", observation.topics.join(",")));
        }
        if !observation.languages.is_empty() {
            out.push_str(&format!(
                "    languages: {}\n",
                observation.languages.join(",")
            ));
        }
        if let Some(branch) = observation.default_branch.as_deref() {
            out.push_str(&format!("    default_branch: {branch}\n"));
        }
        if observation.archived {
            out.push_str("    archived: true\n");
        }
        if !observation.workflows.is_empty() {
            out.push_str(&format!(
                "    workflows: {}\n",
                observation.workflows.join(",")
            ));
        }
        if !observation.releases.is_empty() {
            out.push_str(&format!(
                "    releases: {}\n",
                observation.releases.join(",")
            ));
        }
        if !observation.note.is_empty() {
            out.push_str(&format!("    note: {}\n", observation.note));
        }
    }
    out.push_str(&format!("records: {}\n", records.len()));
    for record in records {
        out.push_str(&format!(
            "  project_id={} source={} evidence={} freshness={}\n",
            record.project_id,
            record.source,
            record.evidence.id(),
            record.freshness.id(),
        ));
    }
    out.trim_end().to_string()
}

fn render_github_propose_human(outcome: &forge::github::ProposeOutcome) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "forge project github propose — contract {}\n",
        forge::github::GITHUB_CONTRACT_VERSION
    ));
    text.push_str(&format!(
        "mode={} state={}",
        outcome.mode,
        outcome.state.id()
    ));
    if let Some(artifact) = outcome.artifact_id.as_deref() {
        text.push_str(&format!(" artifact={artifact}"));
    }
    text.push('\n');
    if !outcome.note.is_empty() {
        text.push_str(&format!("note: {}\n", outcome.note));
    }
    text.trim_end().to_string()
}

fn cmd_github_cli_auth(host: &str, format: Format) -> Result<Output, ForgeError> {
    use forge::github::{run_auth, GhCli};
    let host_trimmed = host.trim();
    let host = if host_trimmed.is_empty() {
        forge::github::GITHUB_CLI_DEFAULT_HOST
    } else {
        host_trimmed
    };
    let cli = GhCli::from_env();
    let result = run_auth(&cli, host);
    let json = serde_json::json!({
        "contract": forge::github::GITHUB_CLI_CONTRACT_VERSION,
        "operation": result.operation.id(),
        "outcome": result.outcome.id(),
        "exit_code": result.exit_code,
        "stderr_tail": result.stderr_tail,
        "note": result.note,
        "cli_source": cli.source,
    });
    let human = format!(
        "forge project github auth — contract {}\n\
         cli_source={}\n\
         outcome={}\n",
        forge::github::GITHUB_CLI_CONTRACT_VERSION,
        cli.source,
        result.outcome.id(),
    );
    if !result.note.is_empty() {
        return Err(result.to_error("github cli auth"));
    }
    Ok(as_output(format, human, json))
}

fn cmd_github_cli_clone(
    _db_path: &Path,
    repository: &str,
    destination: &Path,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{parse_repository, run_clone, GhCli, GITHUB_CLI_CONTRACT_VERSION};
    if !confirm {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli clone requires explicit --confirm; refusing to clone `{repository}` without it"
            ),
        });
    }
    parse_repository(repository).map_err(|reason| ForgeError::GithubCliInvalid {
        reason: format!("repository `{repository}`: {reason}"),
    })?;
    if destination.exists() {
        if destination.is_dir() {
            let read =
                std::fs::read_dir(destination).map_err(|err| ForgeError::GithubCliConflict {
                    reason: format!(
                        "destination `{}` is an unreadable existing directory: {err}",
                        destination.display()
                    ),
                })?;
            if read.count() > 0 {
                return Err(ForgeError::GithubCliConflict {
                    reason: format!(
                        "destination `{}` already exists and is not empty; refusing to clone over it",
                        destination.display()
                    ),
                });
            }
        } else {
            return Err(ForgeError::GithubCliConflict {
                reason: format!(
                    "destination `{}` already exists and is not a directory",
                    destination.display()
                ),
            });
        }
    }
    let cli = GhCli::from_env();
    let result = run_clone(&cli, repository, destination);
    let artifact = result.artifact_url.clone();
    let json = serde_json::json!({
        "contract": GITHUB_CLI_CONTRACT_VERSION,
        "operation": result.operation.id(),
        "outcome": result.outcome.id(),
        "exit_code": result.exit_code,
        "repository": repository,
        "destination": destination.display().to_string(),
        "artifact_url": artifact,
        "stderr_tail": result.stderr_tail,
        "note": result.note,
        "cli_source": cli.source,
    });
    if result.outcome != forge::github::GhOutcome::Done {
        return Err(clone_error(&result));
    }
    let human = format!(
        "forge project github clone — contract {}\n\
         cli_source={}\n\
         outcome=done\n\
         repository={}\n\
         destination={}\n",
        GITHUB_CLI_CONTRACT_VERSION,
        cli.source,
        repository,
        destination.display(),
    );
    Ok(as_output(format, human, json))
}

fn clone_error(result: &forge::github::GhResult) -> ForgeError {
    use forge::github::{GhOutcome, GhResult};
    let ctx = format!("github cli clone {}", result.repository());
    match result {
        GhResult {
            outcome: GhOutcome::Done,
            ..
        } => ForgeError::GithubCliInvalid {
            reason: "internal: done outcome cannot be turned into an error".to_string(),
        },
        _ => result.to_error(&ctx),
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_github_cli_create(
    _db_path: &Path,
    project: &str,
    repo: &str,
    visibility: &str,
    confirm_public: bool,
    push_source: bool,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{
        parse_repository, run_create, GhCli, GhOutcome, GhVisibility, GITHUB_CLI_CONTRACT_VERSION,
    };
    if !confirm {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli create requires explicit --confirm; refusing to create the remote for `{project}` without it"
            ),
        });
    }
    parse_repository(repo).map_err(|reason| ForgeError::GithubCliInvalid {
        reason: format!("--repo `{repo}`: {reason}"),
    })?;
    let visibility = match visibility.trim() {
        "private" => GhVisibility::Private,
        "public" => GhVisibility::Public,
        other => {
            return Err(ForgeError::GithubCliInvalid {
                reason: format!(
                    "unknown --visibility `{other}`; expected `private` (default) or `public`"
                ),
            });
        }
    };
    if matches!(visibility, GhVisibility::Public) && !confirm_public {
        return Err(ForgeError::GithubCliInvalid {
            reason: "github cli create with --visibility public requires --confirm-public; refusing to publish a public repository without it".to_string(),
        });
    }
    if push_source {
        // The CLI confirm flag is the same one we already required;
        // when --push-source is set, the caller is explicitly opting
        // in to the initial push. The `--confirm` flag remains the
        // single gate.
    }
    let project_path = PathBuf::from(project);
    let source = if project_path.exists() && project_path.is_dir() {
        project_path
    } else {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli create expects an existing local project directory; `{project}` is not a directory"
            ),
        });
    };
    let cli = GhCli::from_env();
    let name = repo.split_once('/').map(|(_, n)| n).unwrap_or(repo);
    let result = run_create(&cli, name, &source, visibility, push_source);
    let outcome = result.outcome.clone();
    let json = serde_json::json!({
        "contract": GITHUB_CLI_CONTRACT_VERSION,
        "operation": result.operation.id(),
        "outcome": outcome.id(),
        "exit_code": result.exit_code,
        "repository": repo,
        "visibility": visibility.id(),
        "push_source": push_source,
        "source": source.display().to_string(),
        "artifact_url": result.artifact_url,
        "stderr_tail": result.stderr_tail,
        "note": result.note,
        "cli_source": cli.source,
    });
    if outcome != GhOutcome::Done {
        return Err(create_error(&result));
    }
    let human = format!(
        "forge project github create — contract {}\n\
         cli_source={}\n\
         outcome=done\n\
         repository={}\n\
         visibility={}\n\
         push_source={}\n",
        GITHUB_CLI_CONTRACT_VERSION,
        cli.source,
        repo,
        visibility.id(),
        push_source,
    );
    Ok(as_output(format, human, json))
}

fn create_error(result: &forge::github::GhResult) -> ForgeError {
    use forge::github::{GhOutcome, GhResult};
    let ctx = format!("github cli create {}", result.repository());
    match result {
        GhResult {
            outcome: GhOutcome::Done,
            ..
        } => ForgeError::GithubCliInvalid {
            reason: "internal: done outcome cannot be turned into an error".to_string(),
        },
        _ => result.to_error(&ctx),
    }
}

fn cmd_github_cli_pull_request(
    _db_path: &Path,
    project: &str,
    title: &str,
    body: &str,
    draft: bool,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{
        run_pull_request, GhCli, GhOutcome, GITHUB_CLI_CONTRACT_VERSION, MAX_BODY_BYTES,
        MAX_TITLE_BYTES,
    };
    if !confirm {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli pull-request requires explicit --confirm; refusing to open a PR for `{project}` without it"
            ),
        });
    }
    if title.trim().is_empty() {
        return Err(ForgeError::GithubCliInvalid {
            reason: "--title is required and must not be empty".to_string(),
        });
    }
    if title.len() > MAX_TITLE_BYTES {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "--title exceeds the {MAX_TITLE_BYTES}-byte cap (was {} bytes)",
                title.len()
            ),
        });
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "--body exceeds the {MAX_BODY_BYTES}-byte cap (was {} bytes)",
                body.len()
            ),
        });
    }
    if std::path::Path::new(project).exists() {
        // Local working tree preflight: clean tree + remote present.
        let tree = Command::new("git")
            .arg("-C")
            .arg(project)
            .args(["status", "--porcelain"])
            .output();
        match tree {
            Ok(output) if output.status.success() => {
                let porcelain = String::from_utf8_lossy(&output.stdout);
                if !porcelain.trim().is_empty() {
                    return Err(ForgeError::GithubCliConflict {
                        reason: format!(
                            "github cli pull-request requires a clean working tree; `{project}` has uncommitted changes"
                        ),
                    });
                }
            }
            _ => {
                return Err(ForgeError::GithubCliConflict {
                    reason: format!(
                        "github cli pull-request requires `{project}` to be a git working tree"
                    ),
                });
            }
        }
        let remote = Command::new("git")
            .arg("-C")
            .arg(project)
            .args(["remote", "get-url", "origin"])
            .output();
        match remote {
            Ok(output) if output.status.success() => {
                let url = String::from_utf8_lossy(&output.stdout);
                let url = url.trim();
                if url.is_empty() {
                    return Err(ForgeError::GithubCliConflict {
                        reason: format!(
                            "github cli pull-request requires `{project}` to have an `origin` remote pointing at GitHub"
                        ),
                    });
                }
                if !url.contains("github.com") && !url.contains("github:") {
                    return Err(ForgeError::GithubCliConflict {
                        reason: format!(
                            "github cli pull-request requires the `origin` remote to point at GitHub; `{project}` points at `{url}`"
                        ),
                    });
                }
            }
            _ => {
                return Err(ForgeError::GithubCliConflict {
                    reason: format!(
                        "github cli pull-request requires `{project}` to have an `origin` remote"
                    ),
                });
            }
        }
    } else {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli pull-request expects an existing local project directory; `{project}` is not a directory"
            ),
        });
    }
    let cli = GhCli::from_env();
    let result = run_pull_request(&cli, title, body, draft);
    let outcome = result.outcome.clone();
    let json = serde_json::json!({
        "contract": GITHUB_CLI_CONTRACT_VERSION,
        "operation": result.operation.id(),
        "outcome": outcome.id(),
        "exit_code": result.exit_code,
        "title": title,
        "draft": draft,
        "artifact_url": result.artifact_url,
        "stderr_tail": result.stderr_tail,
        "note": result.note,
        "cli_source": cli.source,
    });
    if outcome != GhOutcome::Done {
        return Err(pr_error(&result));
    }
    let human = format!(
        "forge project github pull-request — contract {}\n\
         cli_source={}\n\
         outcome=done\n\
         title={}\n\
         draft={}\n",
        GITHUB_CLI_CONTRACT_VERSION, cli.source, title, draft,
    );
    Ok(as_output(format, human, json))
}

fn pr_error(result: &forge::github::GhResult) -> ForgeError {
    use forge::github::{GhOutcome, GhResult};
    let ctx = "github cli pull-request".to_string();
    match result {
        GhResult {
            outcome: GhOutcome::Done,
            ..
        } => ForgeError::GithubCliInvalid {
            reason: "internal: done outcome cannot be turned into an error".to_string(),
        },
        _ => result.to_error(&ctx),
    }
}

fn cmd_project_gaps(
    db_path: &Path,
    project: Option<&str>,
    args: &CatalogFilterArgs,
    categories: &[String],
    statuses: &[String],
    remediation_classes: &[String],
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::doctor::gaps::{self, GapCategory, GapFilters, GapStatus, RemediationClass};

    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });

    // When a project id is supplied, scope the records to that id; an
    // unknown project is an `unknown-project` refusal, matching
    // `forge project inspect`.
    let mut records: Vec<forge::catalog::CatalogRecord> = if let Some(id) = project {
        catalog::inspect_records(&bundle, id)?
    } else {
        bundle.records.clone()
    };
    // Stable order: by (project_id, source), matching `forge project list`.
    records.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.source.cmp(&b.source))
    });

    let parsed_categories: Vec<GapCategory> = categories
        .iter()
        .map(|raw| {
            GapCategory::parse(raw).ok_or_else(|| {
                catalog::catalog_invalid(format!(
                    "unknown --category `{raw}`; expected one of \
                     description|tags|ci|compose|manifest|docs|repository"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let parsed_statuses: Vec<GapStatus> = statuses
        .iter()
        .map(|raw| {
            GapStatus::parse(raw).ok_or_else(|| {
                catalog::catalog_invalid(format!(
                    "unknown --status `{raw}`; expected one of \
                     pass|warn|fail|unavailable|not_applicable"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let parsed_classes: Vec<RemediationClass> = remediation_classes
        .iter()
        .map(|raw| {
            RemediationClass::parse(raw).ok_or_else(|| {
                catalog::catalog_invalid(format!(
                    "unknown --remediation-class `{raw}`; expected one of \
                     automatic|semantic|manual"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let filters = GapFilters {
        categories: parsed_categories,
        statuses: parsed_statuses,
        remediation_classes: parsed_classes,
    };
    let report = gaps::build_report(&records, &bundle.statuses, project);
    gaps_output(&report, &filters, &bundle.statuses, format)
}

fn gaps_output(
    report: &forge::doctor::gaps::GapReport,
    filters: &forge::doctor::gaps::GapFilters,
    sources: &[forge::catalog::SourceStatus],
    format: Format,
) -> Result<Output, ForgeError> {
    let filtered: Vec<&forge::doctor::gaps::GapFinding> = report
        .findings
        .iter()
        .filter(|f| filters.matches(f))
        .collect();
    let total = report.findings.len();
    match format {
        Format::Human | Format::Table => Ok(Output::Human(
            forge::doctor::gaps::render_report_human(report, &filtered, sources, total, filters),
        )),
        Format::Json => Ok(Output::Json(serde_json::json!({
            "gaps": {
                "contract": report.contract,
                "project_id": report.project_id,
                "sources": sources,
                "summary": {
                    "total": total,
                    "returned": filtered.len(),
                    "filters": filters_summary(filters),
                },
                "findings": filtered,
            }
        }))),
        Format::Ndjson => Ok(Output::Raw(gaps_ndjson(&filtered)?)),
    }
}

fn filters_summary(filters: &forge::doctor::gaps::GapFilters) -> serde_json::Value {
    serde_json::json!({
        "categories": filters.categories.iter().map(|c| c.id()).collect::<Vec<_>>(),
        "statuses": filters.statuses.iter().map(|s| s.id()).collect::<Vec<_>>(),
        "remediation_classes": filters.remediation_classes.iter().map(|r| r.id()).collect::<Vec<_>>(),
    })
}

fn gaps_ndjson(findings: &[&forge::doctor::gaps::GapFinding]) -> Result<String, ForgeError> {
    let mut out = String::new();
    for finding in findings {
        let value = serde_json::to_value(finding).map_err(|err| {
            catalog::catalog_invalid(format!("gap finding failed to serialize: {err}"))
        })?;
        out.push_str(&serde_json::to_string(&value).map_err(|err| {
            catalog::catalog_invalid(format!("gap finding failed to serialize: {err}"))
        })?);
        out.push('\n');
    }
    Ok(out)
}

/// Build the source selection from the flags, defaulting to the local
/// registry. Only declared sources are read.
fn catalog_selection(args: &CatalogFilterArgs) -> Result<CatalogSourceSelection, ForgeError> {
    let mut kinds: Vec<forge::catalog::SourceKind> = Vec::new();
    if args.sources.is_empty() {
        kinds.push(forge::catalog::SourceKind::Local);
    } else {
        for raw in &args.sources {
            let kind = forge::catalog::SourceKind::parse(raw).ok_or_else(|| {
                catalog::catalog_invalid(format!(
                    "unknown --source `{raw}`; expected one of \
                     local|git|workspace-registry|inventory|github"
                ))
            })?;
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    let workspace_registry = args
        .workspace_registry
        .clone()
        .or_else(|| fleet::resolve_registry_path(None));
    let inventory = args
        .inventory
        .clone()
        .or_else(|| forge::publish::inventory::resolve_source(None));
    Ok(CatalogSourceSelection {
        kinds,
        git_repositories: args.git_repositories.clone(),
        workspace_registry,
        inventory,
        github_repositories: args.github_repositories.clone(),
    })
}

/// Flatten the typed filters plus `key=value` filters into one ordered
/// pair list. `CatalogQuery::from_pairs` refuses an unknown key.
fn catalog_filter_pairs(args: &CatalogFilterArgs) -> Vec<String> {
    let mut pairs = Vec::new();
    for value in &args.tags {
        pairs.push(format!("tag={value}"));
    }
    for value in &args.languages {
        pairs.push(format!("language={value}"));
    }
    for value in &args.profiles {
        pairs.push(format!("profile={value}"));
    }
    for value in &args.lifecycles {
        pairs.push(format!("lifecycle={value}"));
    }
    for value in &args.repositories {
        pairs.push(format!("repository={value}"));
    }
    for value in &args.ci {
        pairs.push(format!("ci={value}"));
    }
    for value in &args.compose {
        pairs.push(format!("compose={value}"));
    }
    for value in &args.evidence {
        pairs.push(format!("evidence={value}"));
    }
    pairs.extend(args.filters.iter().cloned());
    pairs
}

fn catalog_page_output(
    page: forge::catalog::CatalogPage,
    format: Format,
) -> Result<Output, ForgeError> {
    match format {
        Format::Human | Format::Table => Ok(Output::Human(catalog::render_page_human(&page))),
        Format::Json => Ok(Output::Json(serde_json::json!({"catalog": page}))),
        Format::Ndjson => Ok(Output::Raw(catalog_ndjson(&page.records)?)),
    }
}

fn catalog_records_output(
    project: &str,
    records: &[forge::catalog::CatalogRecord],
    format: Format,
) -> Result<Output, ForgeError> {
    match format {
        Format::Human | Format::Table => Ok(Output::Human(catalog::render_records_human(
            "inspect", records,
        ))),
        Format::Json => Ok(Output::Json(serde_json::json!({
            "catalog": {
                "contract": CATALOG_CONTRACT_VERSION,
                "project_id": project,
                "records": records,
            }
        }))),
        Format::Ndjson => Ok(Output::Raw(catalog_ndjson(records)?)),
    }
}

fn catalog_counts_output(
    title: &str,
    counts: &[forge::catalog::CatalogValueCount],
    format: Format,
) -> Result<Output, ForgeError> {
    match format {
        Format::Human | Format::Table => {
            Ok(Output::Human(catalog::render_counts_human(title, counts)))
        }
        Format::Json => {
            let mut catalog = serde_json::Map::new();
            catalog.insert(
                "contract".to_string(),
                serde_json::json!(CATALOG_CONTRACT_VERSION),
            );
            catalog.insert(
                title.to_string(),
                serde_json::to_value(counts).map_err(|err| {
                    catalog::catalog_invalid(format!("cannot serialize {title}: {err}"))
                })?,
            );
            Ok(Output::Json(serde_json::json!({ "catalog": catalog })))
        }
        Format::Ndjson => {
            let mut out = String::new();
            for entry in counts {
                out.push_str(&serde_json::to_string(entry).map_err(|err| {
                    catalog::catalog_invalid(format!("cannot serialize {title}: {err}"))
                })?);
                out.push('\n');
            }
            Ok(Output::Raw(out))
        }
    }
}

fn catalog_ndjson(records: &[forge::catalog::CatalogRecord]) -> Result<String, ForgeError> {
    let mut out = String::new();
    for record in records {
        out.push_str(&serde_json::to_string(record).map_err(|err| {
            catalog::catalog_invalid(format!("cannot serialize catalog record: {err}"))
        })?);
        out.push('\n');
    }
    Ok(out)
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
    if matches!(format, Format::Json | Format::Ndjson) {
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
        Format::Human | Format::Table => Ok(Output::Human(summary)),
        Format::Json | Format::Ndjson => Ok(Output::Json(summary_json)),
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
        Output::Raw(text) => print!("{text}"),
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
            Output::Raw(text) => print!("{text}"),
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
