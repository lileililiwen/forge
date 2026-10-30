//! Auth commands (`auth`).
//!
//! The application-facing path: a project owner prepares the end-user identity of
//! one project. It is separate from Forge's own administrator login
//! (`forge identity`) and from a project's admin OIDC session.

use forge::auth::packs::{default_pack_directory, discover};
use forge::auth::setup::{AuthSetupOutcome, AuthSetupRequest, AuthSetupRunner};
use forge::core::ForgeError;
use serde_json::json;
use std::path::{Path, PathBuf};

use super::commands::{AuthCommands, AuthProviderCommands};
use super::projects::as_output;
use crate::{Format, Output};

pub(crate) fn cmd_auth(command: &AuthCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        AuthCommands::Setup {
            project,
            provider,
            packs,
            state_dir,
            port,
            callback_path,
            confirm,
        } => {
            let project_dir = PathBuf::from(project);
            let project_id = project_id_of(&project_dir)?;
            let catalog = discover(&pack_directory(packs.as_deref()));
            // Without an explicit choice, the setup uses the enabled local pack.
            let provider = match provider {
                Some(provider) => provider.clone(),
                None => default_provider(&catalog),
            };
            // The provider runtime keeps its state with the project unless the
            // owner names another directory.
            let state_dir = state_dir
                .clone()
                .unwrap_or_else(|| default_state_dir(&project_dir).display().to_string());
            let runner = AuthSetupRunner::new(catalog);
            let request = AuthSetupRequest {
                project_id: project_id.clone(),
                project_dir: project_dir.clone(),
                provider_id: provider.clone(),
                provider_input: json!({
                    "state_dir": state_dir,
                    "port": port,
                    "callback_path": callback_path,
                    "scopes": ["openid", "profile"],
                }),
                confirm: *confirm,
            };
            let outcome = runner.run(&request)?;

            match outcome {
                AuthSetupOutcome::Preview { plan, digest } => Ok(as_output(
                    format,
                    format!(
                        "Auth setup plan for project '{project_id}' via provider '{}':\n{}\nConfirm with --confirm to apply. Plan digest: {digest}",
                        plan.provider_id,
                        render_changes(&plan)
                    ),
                    json!({
                        "contract": "forge-auth-setup/1.0",
                        "status": "preview",
                        "project_id": project_id,
                        "provider": plan.provider_id,
                        "plan_digest": digest,
                        "changes": plan.proposed_changes,
                        "summary": plan.summary,
                    }),
                )),
                AuthSetupOutcome::Declined { plan } => Ok(as_output(
                    format,
                    format!("Auth setup for project '{project_id}' was declined. Nothing was written."),
                    json!({
                        "contract": "forge-auth-setup/1.0",
                        "status": "declined",
                        "project_id": project_id,
                        "provider": plan.provider_id,
                    }),
                )),
                AuthSetupOutcome::Blocked { reason, recovery } => Ok(as_output(
                    format,
                    format!(
                        "Auth setup for project '{project_id}' is blocked ({reason}). Nothing was written.{}",
                        render_recovery(&recovery)
                    ),
                    json!({
                        "contract": "forge-auth-setup/1.0",
                        "status": "blocked",
                        "project_id": project_id,
                        "reason": reason,
                        "recovery": recovery,
                    }),
                )),
                AuthSetupOutcome::Applied {
                    plan,
                    population,
                    manifest_path,
                } => Ok(as_output(
                    format,
                    format!(
                        "Auth setup for project '{project_id}' is ready. Provider '{}' serves population '{}' at {}. The project manifest records the configuration.",
                        plan.provider_id,
                        population.population_id,
                        population.issuer
                    ),
                    json!({
                        "contract": "forge-auth-setup/1.0",
                        "status": "applied",
                        "project_id": project_id,
                        "provider": plan.provider_id,
                        "population": population,
                        "manifest": manifest_path.display().to_string(),
                    }),
                )),
            }
        }
        AuthCommands::Provider {
            command: AuthProviderCommands::List { packs },
        } => {
            let catalog = discover(&pack_directory(packs.as_deref()));
            let listing: Vec<_> = catalog
                .packs
                .iter()
                .map(|pack| {
                    json!({
                        "id": pack.id,
                        "description": pack.description,
                        "modes": pack.modes,
                        "enabled": pack.enabled,
                        "available": pack.available,
                    })
                })
                .collect();
            Ok(as_output(
                format,
                render_pack_list(&catalog, &listing),
                json!({
                    "contract": "forge-auth-provider-catalog/1.0",
                    "packs": listing,
                }),
            ))
        }
        AuthCommands::Provider {
            command: AuthProviderCommands::Inspect { provider, packs },
        } => {
            let catalog = discover(&pack_directory(packs.as_deref()));
            let pack = catalog
                .find(provider)
                .ok_or_else(|| ForgeError::AuthSetupRefused {
                    reason: format!("the provider pack '{provider}' is not installed"),
                })?;
            Ok(as_output(
                format,
                format!(
                    "Provider pack '{}': {}\nModes: {}\nEnabled: {}\nExecutable present: {}",
                    pack.id,
                    pack.description,
                    pack.modes.join(", "),
                    pack.enabled,
                    pack.available
                ),
                json!({
                    "contract": "forge-auth-provider-catalog/1.0",
                    "provider": pack,
                }),
            ))
        }
    }
}

/// Renders the installed provider packs for a human reader.
///
/// The list names every pack, so an owner can see what Forge would use
/// without reading the descriptor files.
fn render_pack_list(
    catalog: &forge::auth::packs::PackCatalog,
    listing: &[serde_json::Value],
) -> String {
    if listing.is_empty() {
        return format!(
            "No identity provider pack is installed in {}.",
            catalog.directory.display()
        );
    }
    let rows = listing
        .iter()
        .map(|pack| {
            let id = pack.get("id").and_then(|value| value.as_str()).unwrap_or_default();
            let description = pack
                .get("description")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let modes = pack
                .get("modes")
                .and_then(|value| value.as_array())
                .map(|modes| {
                    modes
                        .iter()
                        .filter_map(|mode| mode.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let enabled = pack
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let available = pack
                .get("available")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            format!(
                "  - {id}: {description} (modes: {modes}; enabled: {enabled}; executable present: {available})"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{} identity provider pack(s) in {}:\n{rows}",
        listing.len(),
        catalog.directory.display()
    )
}

fn render_changes(plan: &forge::auth::ProviderResponse) -> String {
    if plan.proposed_changes.is_empty() {
        return "  (the provider names no change)".to_string();
    }
    plan.proposed_changes
        .iter()
        .map(|change| format!("  - {}: {}", change.kind, change.summary))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_recovery(recovery: &[forge::auth::RecoveryAction]) -> String {
    if recovery.is_empty() {
        return String::new();
    }
    let lines = recovery
        .iter()
        .map(|action| format!("\n  {}: {}", action.reason, action.instruction))
        .collect::<Vec<_>>()
        .join("");
    lines.to_string()
}

/// The directory that holds the pack descriptors.
fn pack_directory(explicit: Option<&str>) -> PathBuf {
    match explicit {
        Some(path) => PathBuf::from(path),
        None => default_pack_directory(&forge_root()),
    }
}

/// The default provider state directory: inside the project, out of the way.
fn default_state_dir(project_dir: &Path) -> PathBuf {
    project_dir.join(".forge").join("identity")
}

/// The Forge checkout root, used to find the provider packs.
fn forge_root() -> PathBuf {
    if let Ok(root) = std::env::var("FORGE_ROOT") {
        return PathBuf::from(root);
    }
    let mut current = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    while let Some(parent) = current.parent() {
        if parent.join("modules").is_dir() && parent.join("Cargo.toml").is_file() {
            return parent.to_path_buf();
        }
        current = parent.to_path_buf();
    }
    PathBuf::from(".")
}

/// The project id of a project directory.
fn project_id_of(project_dir: &Path) -> Result<String, ForgeError> {
    let manifest = project_dir.join("forge.yaml");
    let bytes = std::fs::read(&manifest).map_err(|error| ForgeError::AuthSetupRefused {
        reason: format!("cannot read {}: {error}", manifest.display()),
    })?;
    let value: serde_json::Value =
        serde_yaml::from_slice(&bytes).map_err(|error| ForgeError::AuthSetupRefused {
            reason: format!("the project manifest is not valid YAML: {error}"),
        })?;
    value
        .get("project")
        .and_then(|project| project.get("id"))
        .and_then(|id| id.as_str())
        .map(str::to_string)
        .ok_or_else(|| ForgeError::AuthSetupRefused {
            reason: "the project manifest carries no project id".to_string(),
        })
}

/// The provider the setup uses when the owner names none: the first enabled and
/// present pack, or the local pack when no pack is installed.
fn default_provider(catalog: &forge::auth::packs::PackCatalog) -> String {
    catalog
        .selectable()
        .first()
        .map(|pack| pack.id.clone())
        .unwrap_or_else(|| "keycloak-local".to_string())
}
