//! `forge` CLI transport: argument parsing and output rendering only.
//!
//! All project rules live in Core; this layer maps typed outcomes to
//! human text or JSON plus exit codes, independently of any GUI, AI or
//! network service.

use clap::{Parser, Subcommand, ValueEnum};
use forge::core::ForgeError;
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
