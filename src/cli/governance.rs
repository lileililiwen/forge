//! Governance commands (`governance`).
//!
//! Typed CLI handlers for external governance provider selection.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use forge::governance::{
    check_project as check_governance_project, list_providers as list_governance_providers,
    resolve_known_adapter, save_provider_selection_with_root, GovernanceObservation,
    ProviderStatus, GOVERNANCE_CONTRACT_VERSION, LOCAL_PROVIDER_ID, WORKSPACE_ROOT_ENV,
};
use std::path::{Path, PathBuf};

use super::commands::GovernanceCommands;
use super::projects::as_output;
use crate::{Format, Output};

pub(crate) fn cmd_governance(
    command: &GovernanceCommands,
    format: Format,
) -> Result<Output, ForgeError> {
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
