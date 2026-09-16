//! `forge` CLI transport: argument parsing and output rendering only.
//!
//! All project rules live in Core; this layer maps typed outcomes to
//! human text or JSON plus exit codes, independently of any GUI, AI or
//! network service.

use clap::{Parser, Subcommand, ValueEnum};
use forge::core::ForgeError;
use forge::doctor::{parse_target_level, render_report_human, run_doctor, RegistryObservation};
use forge::generate::{generate, normalize_explicit, parse_interactive, verify_native};
use forge::import::{adopt_import, inspect_import, render_proposal_human};
use forge::profile::{inspect_profile, list_profiles, preflight_profile, resolve_profile};
use forge::registry::{default_registry_path, ProjectRecord, Registry};
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    let db_path = cli.registry.clone().unwrap_or_else(default_registry_path);

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
    let report = run_doctor(path, level, observation.as_ref())?;
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

fn render_profile_human(p: &forge::profile::ProfileDescriptor) -> String {
    vec![
        format!("id: {}", p.id),
        format!("version: {}", p.version),
        format!("adapter: {}", p.adapter),
        format!("language: {}", p.language),
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
    ]
    .join("\n")
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
