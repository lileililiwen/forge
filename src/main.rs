//! `forge` CLI transport: argument parsing and output rendering only.
//!
//! All project rules live in Core; this layer maps typed outcomes to
//! human text or JSON plus exit codes, independently of any GUI, AI or
//! network service.
mod cli;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use forge::registry::default_registry_path;

use cli::agent::cmd_agent;
use cli::commands::{
    AgentCommands, ClassifyCommands, ContractCommands, DeliveryCommands, DocsCommands,
    GovernanceCommands, GraduationCommands, IntentCommands, PluginsCommands, PortalCommands,
    ProcedureCommands, PublishCommands, RemediateCommands, StudioCommands, UiPatternCommands,
    WorkspaceCommands,
};
use cli::commands_ops::{
    AnalyticsCommands, ComponentCommands, DeployCommands, DescribeCommands, FleetCommands,
    InventoryCommands, KitCommands, ProfileCommands, ProjectCommands, ProviderCommands,
    ReadinessCommands, SpecCommands, StandardCommands,
};
use cli::commands_portfolio::PortfolioCommands;
use cli::commands_services::{
    ApiCommands, FeatureCommands, IdentityCommands, McpCommands, ReleaseCommands, WebCommands,
};
use cli::component::cmd_component;
use cli::delivery::{cmd_delivery, cmd_studio};
use cli::feature::{cmd_feature, cmd_spec, cmd_upgrade, cmd_upgrade_fleet, render_error};
use cli::fleet_exec::cmd_fleet;
use cli::functions_13::{cmd_contract, cmd_gate};
use cli::functions_9::{cmd_analytics, cmd_api, cmd_portal, cmd_portfolio, cmd_readiness, cmd_web};
use cli::gitops::{cmd_commit, cmd_mcp, cmd_mirror, cmd_push, cmd_test};
use cli::governance::cmd_governance;
use cli::identity::cmd_identity;
use cli::intent::cmd_intent;
use cli::inventory::cmd_inventory;
use cli::plugins::cmd_plugins;
use cli::procedure::cmd_procedure;
use cli::project::cmd_project;
use cli::projects::{
    cmd_check, cmd_doctor, cmd_import, cmd_inspect, cmd_list, cmd_new, cmd_register,
};
use cli::provider::cmd_provider;
use cli::publish::cmd_publish;
use cli::release::{cmd_deploy, cmd_docs, cmd_release};
use cli::semantic::{cmd_classify, cmd_describe, cmd_remediate, cmd_standard};
use cli::setup::{cmd_graduation, cmd_kit, cmd_profile, cmd_workspace};
use cli::ui_pattern::cmd_ui_pattern;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Format {
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
pub(crate) struct Cli {
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
    /// Converge a workspace root into the registry with a per-directory report.
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommands,
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
    /// Repack or verify the committed shared-layer kit feed.
    Kit {
        #[command(subcommand)]
        command: KitCommands,
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
        /// Bypass an unmet shared-layer consumption floor, recording the
        /// reason visibly and dated in the generated `forge.yaml` and
        /// README. The reason is mandatory: an empty reason refuses.
        /// Never inferred, and never applied to a floor that was met.
        #[arg(long, value_name = "REASON")]
        kit_exception: Option<String>,
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
    /// Serve standalone Forge browser assets from the Rust static web server.
    Web {
        #[command(subcommand)]
        command: WebCommands,
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
    /// List every configured plugin (GitHub, OpenPanel, any future remote)
    /// with its kind, enabled state and declared capabilities.
    Plugins {
        #[command(subcommand)]
        command: PluginsCommands,
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
    /// Site Studio: review an AppSpec, run a bounded preview,
    /// and journal scoped refinement requests
    /// (`forge-studio-preview-refinement`).
    Studio {
        #[command(subcommand)]
        command: StudioCommands,
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

    // Workspace sync prints its per-directory report to stdout even when
    // some directories failed, so it owns its exit code instead of using
    // the generic error path (exit 0 iff no directory failed; skips never
    // fail the run).
    if let Commands::Workspace { command } = &cli.command {
        return cmd_workspace(&db_path, command, cli.format);
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
        Commands::Workspace { .. } => {
            // Handled by the early `if let` above (the sync run owns its
            // exit code so the per-directory report still prints when some
            // directories failed); this arm exists only to keep the match
            // exhaustive.
            return ExitCode::from(2);
        }
        Commands::Profile { command } => cmd_profile(command, cli.format),
        Commands::Kit { command } => cmd_kit(command, cli.format),
        Commands::New {
            path,
            profile,
            id,
            name,
            features,
            verify_native,
            no_workspace_metadata,
            standard_pack,
            kit_exception,
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
            kit_exception.as_deref(),
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
        Commands::Web { command } => cmd_web(command, cli.format),
        Commands::Portal { command } => cmd_portal(&db_path, command, cli.format),
        Commands::Portfolio { command } => cmd_portfolio(&db_path, command, cli.format),
        Commands::Readiness { command } => cmd_readiness(command, cli.format),
        Commands::Provider { command } => cmd_provider(&db_path, command, cli.format),
        Commands::Plugins { command } => cmd_plugins(&db_path, command, cli.format),
        Commands::Governance { command } => cmd_governance(command, cli.format),
        Commands::Fleet { command } => cmd_fleet(&db_path, command, cli.format),
        Commands::Project { command } => cmd_project(&db_path, command, cli.format),
        Commands::Contract { command } => cmd_contract(&db_path, command, cli.format),
        Commands::Inventory { command } => cmd_inventory(command, cli.format),
        Commands::Standard { command } => cmd_standard(command, cli.format),
        Commands::Remediate { command } => cmd_remediate(&db_path, command, cli.format),
        Commands::Describe { command } => cmd_describe(command, cli.format),
        Commands::Classify { command } => cmd_classify(&db_path, command, cli.format),
        Commands::Delivery { command } => cmd_delivery(&db_path, command, cli.format),
        Commands::Studio { command } => cmd_studio(&db_path, command, cli.format),
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

pub(crate) enum Output {
    Human(String),
    Json(serde_json::Value),
    /// Already-serialized machine output (NDJSON): printed verbatim.
    Raw(String),
}
