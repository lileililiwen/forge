//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use clap::Subcommand;
use forge::core::ForgeError;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum AgentCommands {
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
#[derive(Debug, Subcommand)]
pub(crate) enum PublishCommands {
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
/// One per-project fleet outcome returned by a worker thread. The
/// worker performs the publish phases (transport + in-memory
/// reports) but writes no journal row and prints nothing; the main
/// thread replays every journal row serially and renders all
/// output so concurrent runs keep identical row shapes and queue
/// id semantics (`fleet-live-rollout` D1).
pub(super) enum FleetEntryOutcome {
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
/// Site Studio subcommands
/// (`forge-studio-preview-refinement`).
#[derive(Debug, Subcommand)]
pub(crate) enum StudioCommands {
    /// Validate an AppSpec from a YAML file and print the
    /// closed `forge-app-spec/0.1.0` envelope. Read-only —
    /// the file is parsed, validated, and never written back.
    Spec {
        /// Registered project id the spec is scoped to.
        project: String,
        /// Path to the `forge.app.yaml` candidate (default:
        /// `<project>/forge.app.yaml`).
        #[arg(long)]
        from: Option<PathBuf>,
        /// Current revision to confirm against. Required with
        /// `--confirm yes`; use `r0` for the first save.
        #[arg(long = "expected-revision")]
        expected_revision: Option<String>,
        /// Persist the validated spec after explicit `--confirm yes`.
        /// Without it the command stays read-only.
        #[arg(long = "confirm")]
        confirm: Option<String>,
    },
    /// Read the current Studio session record (preview state,
    /// revisions, last journal row). Read-only.
    Preview {
        /// Registered project id.
        project: String,
        /// `--start` asks the bounded profile runner to bind the
        /// reserved port; `--stop` is the idempotent kill. With
        /// no flag the command prints the current state.
        #[arg(long, value_enum, default_value = "status")]
        action: StudioPreviewAction,
        /// Confirmation token required for `--start` and `--stop`
        /// so a refresh or a tab-restore cannot mutate state.
        #[arg(long = "confirm")]
        confirm: Option<String>,
    },
    /// Submit a refinement request. Validates the request,
    /// journals a `studio.refine` row, and bumps `app_revision`.
    /// Read-only at the file level — no editor patch is applied
    /// in this cycle (the agent-adapter hook is a follow-up).
    Refine {
        /// Registered project id.
        project: String,
        /// Expected revision (current `spec_revision` or
        /// `app_revision`). Mismatch is refused as a typed
        /// `studio-revision-conflict`.
        #[arg(long = "expected-revision")]
        expected_revision: String,
        /// Free-text refinement request (1..=2000 chars,
        /// credential-redacted before journal).
        #[arg(long = "request")]
        request: String,
        /// Comma-separated list of in-project file paths the
        /// refinement touches.
        #[arg(long = "selected-files", value_delimiter = ',')]
        selected_files: Vec<String>,
    },
}
#[derive(Debug, clap::Args)]
pub(crate) struct RemediationArgs {
    /// Project directory (default: current directory).
    #[arg(long, default_value = ".")]
    pub(super) target: PathBuf,
    /// Doctor finding id, for example gaps.ci.demo.ci.
    #[arg(long)]
    pub(super) finding: Option<String>,
    /// Standard pack selector, for example baseline-service@1.1.0.
    #[arg(long)]
    pub(super) pack: Option<String>,
    /// Read a previously saved JSON plan instead of assembling one.
    #[arg(long)]
    pub(super) plan: Option<PathBuf>,
    /// Required for apply; prevents accidental writes.
    #[arg(long)]
    pub(super) confirm: bool,
}
#[derive(Debug, Subcommand)]
pub(crate) enum GraduationCommands {
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
pub(crate) enum GovernanceCommands {
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
pub(crate) enum ClassifyCommands {
    /// Suggest a bounded classification for the named project.
    Suggest(SemanticSuggestArgs),
    /// Derive classification proposals from repository evidence (no model, no network, no project-field write).
    Derive {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
    },
    /// Apply approved classification metadata outward through the configured metadata plugin (PR mode only).
    Apply {
        /// Registered project id or filesystem path (default: current directory).
        #[arg(default_value = ".")]
        target: String,
        /// Required explicit confirmation; without it the command refuses.
        #[arg(long)]
        confirm: bool,
    },
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
#[derive(Debug, Subcommand)]
pub(crate) enum DocsCommands {
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
pub(crate) enum WorkspaceCommands {
    /// Scan the immediate children of ROOT and register or adopt each one.
    Sync {
        /// Workspace root (default: current directory).
        #[arg(default_value = ".")]
        root: PathBuf,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum PluginsCommands {
    /// List every configured plugin with its kind, state and capabilities.
    List {
        /// Project directory whose `.forge/providers.yaml` is read (default: current directory).
        #[arg(default_value = ".")]
        project: String,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum DeliveryCommands {
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
pub(crate) enum PublishProviderCommands {
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
pub(crate) enum UiPatternCommands {
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
pub(crate) enum ContractCommands {
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
pub(crate) enum ProcedureCommands {
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
pub(crate) enum RemediateCommands {
    /// Inspect automatic findings and produce a read-only plan.
    Scan(RemediationArgs),
    /// Produce a versioned read-only remediation plan.
    Plan(RemediationArgs),
    /// Show the files a remediation plan would change.
    Diff(RemediationArgs),
    /// Apply a plan after explicit confirmation.
    Apply(RemediationArgs),
}
#[derive(Debug, clap::Args)]
pub(crate) struct SemanticSuggestArgs {
    /// Registered project id or filesystem path (default: current directory).
    #[arg(default_value = ".")]
    pub(super) target: String,
    /// Bounded, control-free suggested value (e.g. `tools for inspecting catalogs`).
    #[arg(long)]
    pub(super) suggested_value: String,
    /// Optional current value (the value the project has today).
    #[arg(long = "current-value")]
    pub(super) current_value: Option<String>,
    /// Confidence label: `low`, `medium`, or `high` (default: `medium`).
    #[arg(long, default_value = "medium")]
    pub(super) confidence: String,
    /// Provider identity: `operator` (default) or `local`.
    #[arg(long, default_value = "operator")]
    pub(super) provider: String,
    /// Path to the evidence source (repeatable; one per source).
    #[arg(long = "evidence-path", value_name = "PATH")]
    pub(super) evidence_paths: Vec<String>,
    /// Revision the evidence was observed at. A single revision
    /// applies to every `--evidence-path`; an evidence path
    /// without a paired revision is refused.
    #[arg(long = "evidence-revision", value_name = "REV")]
    pub(super) evidence_revisions: Vec<String>,
    /// Optional bounded excerpt for each `--evidence-path`.
    #[arg(long = "evidence-excerpt", value_name = "EXCERPT")]
    pub(super) evidence_excerpts: Vec<String>,
    /// Optional human note recorded with the proposal.
    #[arg(long)]
    pub(super) note: Option<String>,
}
#[derive(Debug, Subcommand)]
pub(crate) enum PortalCommands {
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
pub(crate) enum IntentCommands {
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
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum StudioPreviewAction {
    Status,
    Start,
    Stop,
}
#[derive(Debug, Subcommand)]
pub(crate) enum CapCommands {
    /// List every capability group with live Ready plugin counts.
    List {
        /// Project directory whose `.forge/providers.yaml` is read (default: current directory).
        #[arg(default_value = ".")]
        project: String,
    },
    /// Inspect one capability group: Ready plugins plus mapped flat commands.
    Inspect {
        /// Capability group (gate, quality, agent, contract, analytics, delivery, metadata).
        capability: String,
        /// Project directory whose `.forge/providers.yaml` is read (default: current directory).
        #[arg(default_value = ".")]
        project: String,
    },
    /// Print the descriptor snippet for adding a plugin to a group.
    Add {
        /// Capability group to add a plugin to.
        capability: String,
        /// Plugin id to add.
        id: String,
    },
    /// Print the underlying flat `forge …` command for a group action.
    Run {
        /// Capability group to run (gate, agent, contract, delivery, quality, analytics).
        capability: String,
        /// Passthrough action hint (e.g. `list`, `status`).
        #[arg(default_value = "list")]
        action: String,
    },
}
