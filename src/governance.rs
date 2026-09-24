//! Standalone-first governance provider contract.
//!
//! Forge owns the local provider and normalized observation model. External
//! governance systems are optional executable adapters that exchange bounded
//! JSON and never become Forge dependencies.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::policy::redact_credentials;

pub const GOVERNANCE_CONTRACT_VERSION: &str = "0.1.0";
pub const LOCAL_PROVIDER_ID: &str = "local";
/// Known-provider id whose adapter location is packaged as a preset: a
/// candidate below an explicitly supplied workspace root, never a guess.
pub const WORKSPACE_GOVERNANCE_PROVIDER_ID: &str = "workspace-governance";
/// Environment carrying the workspace root when `--workspace-root` is
/// absent. The root is configuration input only — Forge never searches
/// parent directories or the network for a provider.
pub const WORKSPACE_ROOT_ENV: &str = "FORGE_WORKSPACE_ROOT";
/// Packaged candidate relative to the workspace root. Live-verified
/// 2026-09-24: the real sibling layout nests the checkout inside the
/// portfolio it governs, so the packaged adapter lives at
/// `<workspace-root>/workspace-governance/scripts/forge_governance_adapter.py`
/// (the design's original `<root>/scripts/...` guess is disproved in
/// `tests/fixtures/governance-audit/NOTES.md`). The root itself is the
/// same value the adapter's own `WORKSPACE_ROOT` environment input takes.
const WORKSPACE_GOVERNANCE_ADAPTER_RELPATH: &str =
    "workspace-governance/scripts/forge_governance_adapter.py";
const CONFIG_RELATIVE_PATH: &str = ".forge/providers.yaml";
const OBSERVATIONS_RELATIVE_PATH: &str = ".forge/governance/observations.json";
const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const MAX_ADAPTER_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_EVIDENCE_CHARS: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderStatus {
    Pass,
    Fail,
    Blocked,
    Unknown,
    Unavailable,
    Stale,
    Disabled,
    Incompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernanceStatus(pub ProviderStatus);

impl From<ProviderStatus> for GovernanceStatus {
    fn from(status: ProviderStatus) -> Self {
        Self(status)
    }
}

impl GovernanceStatus {
    pub fn is_healthy(self) -> bool {
        matches!(self.0, ProviderStatus::Pass)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceProviderConfig {
    pub provider: String,
    #[serde(default)]
    pub adapter: Option<String>,
    /// Explicit workspace root recorded at selection time. When set it is
    /// passed to the adapter as `WORKSPACE_ROOT` at run time, so later
    /// checks never depend on the environment staying set. Absent-when-
    /// unset keeps every pre-change selection file byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_protocol_version() -> String {
    GOVERNANCE_CONTRACT_VERSION.to_string()
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

impl Default for GovernanceProviderConfig {
    fn default() -> Self {
        Self {
            provider: LOCAL_PROVIDER_ID.to_string(),
            adapter: None,
            workspace_root: None,
            enabled: true,
            protocol_version: default_protocol_version(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceConfig {
    #[serde(default)]
    pub provider: Option<GovernanceProviderConfig>,
}

impl GovernanceConfig {
    pub fn selected_provider(&self) -> &str {
        self.provider
            .as_ref()
            .map(|provider| provider.provider.as_str())
            .unwrap_or(LOCAL_PROVIDER_ID)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GovernanceObservation {
    pub provider: String,
    pub protocol_version: String,
    pub project_id: String,
    pub project_path: String,
    pub status: ProviderStatus,
    pub observed_at: String,
    #[serde(default)]
    pub source_revision: Option<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceProviderDescriptor {
    pub provider: String,
    pub configured: bool,
    pub enabled: bool,
    pub adapter: Option<String>,
    pub protocol_version: String,
}

#[derive(Debug, Deserialize)]
struct AdapterObservation {
    #[serde(default)]
    provider: Option<String>,
    protocol_version: String,
    project_id: String,
    status: String,
    #[serde(default)]
    source_revision: Option<String>,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct AdapterRequest<'a> {
    contract: &'static str,
    action: &'static str,
    project_id: &'a str,
    project_path: &'a str,
}

pub fn config_path(project_root: &Path) -> PathBuf {
    project_root.join(CONFIG_RELATIVE_PATH)
}

pub fn load_config(project_root: &Path) -> Result<GovernanceConfig, ForgeError> {
    let path = config_path(project_root);
    if !path.exists() {
        return Ok(GovernanceConfig::default());
    }
    let bytes = fs::read(&path).map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot read {}: {err}", path.display()),
    })?;
    let config: GovernanceConfig =
        serde_yaml::from_slice(&bytes).map_err(|err| ForgeError::GovernanceInvalid {
            reason: format!("invalid {}: {err}", path.display()),
        })?;
    validate_config(&config)?;
    Ok(config)
}

pub fn save_provider_selection(
    project_root: &Path,
    provider: &str,
    adapter: Option<&str>,
    enabled: bool,
    timeout_ms: u64,
) -> Result<(), ForgeError> {
    save_provider_selection_with_root(project_root, provider, adapter, None, enabled, timeout_ms)
}

/// Persist a provider selection, optionally recording the absolute
/// workspace root a packaged adapter was resolved from. The stored root is
/// later handed to the adapter as `WORKSPACE_ROOT` so re-checks do not
/// depend on the environment staying set. `workspace_root` must be an
/// absolute path; a relative one is refused.
pub fn save_provider_selection_with_root(
    project_root: &Path,
    provider: &str,
    adapter: Option<&str>,
    workspace_root: Option<&str>,
    enabled: bool,
    timeout_ms: u64,
) -> Result<(), ForgeError> {
    validate_provider_id(provider)?;
    if timeout_ms == 0 || timeout_ms > 300_000 {
        return Err(ForgeError::GovernanceInvalid {
            reason: "timeout_ms must be between 1 and 300000".to_string(),
        });
    }
    if provider == LOCAL_PROVIDER_ID && adapter.is_some() {
        return Err(ForgeError::GovernanceInvalid {
            reason: "the local provider does not accept an adapter".to_string(),
        });
    }
    if provider == LOCAL_PROVIDER_ID && workspace_root.is_some() {
        return Err(ForgeError::GovernanceInvalid {
            reason: "the local provider does not accept a workspace root".to_string(),
        });
    }
    if let Some(root) = workspace_root {
        if !Path::new(root).is_absolute() {
            return Err(ForgeError::GovernanceInvalid {
                reason: format!("workspace root `{root}` must be an absolute path"),
            });
        }
    }
    let config = GovernanceConfig {
        provider: Some(GovernanceProviderConfig {
            provider: provider.to_string(),
            adapter: adapter.map(str::to_string),
            workspace_root: workspace_root.map(str::to_string),
            enabled,
            protocol_version: GOVERNANCE_CONTRACT_VERSION.to_string(),
            timeout_ms,
        }),
    };
    validate_config(&config)?;
    let path = config_path(project_root);
    let parent = path.parent().ok_or_else(|| ForgeError::GovernanceInvalid {
        reason: format!("invalid configuration path {}", path.display()),
    })?;
    fs::create_dir_all(parent).map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot create {}: {err}", parent.display()),
    })?;
    let temp = parent.join(format!(".providers.yaml.tmp-{}", std::process::id()));
    let bytes = serde_yaml::to_string(&config).map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot serialize provider configuration: {err}"),
    })?;
    fs::write(&temp, bytes).map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot write {}: {err}", temp.display()),
    })?;
    fs::rename(&temp, &path).map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot promote {}: {err}", path.display()),
    })
}

/// Resolve a packaged adapter candidate for a known provider id from
/// explicit operator input only: the `workspace_root` argument wins over
/// the `env_root` value (the caller reads [`WORKSPACE_ROOT_ENV`]), and
/// neither being present is a refusal naming both input paths. The
/// candidate must be an existing regular file with an executable bit;
/// every refusal names the exact candidate path so the operator can add
/// `--adapter` or fix the root. Returns `None` for providers without a
/// packaged preset, leaving caller behavior (explicit `--adapter` or no
/// adapter) unchanged. No parent-directory search, no network lookup.
pub fn resolve_known_adapter(
    provider: &str,
    workspace_root: Option<&Path>,
    env_root: Option<&str>,
) -> Option<Result<PathBuf, ForgeError>> {
    if provider != WORKSPACE_GOVERNANCE_PROVIDER_ID {
        return None;
    }
    let root = workspace_root.map(Path::to_path_buf).or_else(|| {
        env_root
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    });
    let Some(root) = root else {
        return Some(Err(ForgeError::GovernanceInvalid {
            reason: format!(
                "provider `{provider}` has no packaged adapter without a workspace root: \
                 pass --workspace-root or set {WORKSPACE_ROOT_ENV}"
            ),
        }));
    };
    Some(resolve_packaged_candidate(provider, &root))
}

fn resolve_packaged_candidate(provider: &str, root: &Path) -> Result<PathBuf, ForgeError> {
    let candidate = root.join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH);
    let metadata = fs::metadata(&candidate).map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!(
            "provider `{provider}` candidate {} is not an existing regular file ({err})",
            candidate.display()
        ),
    })?;
    if !metadata.is_file() {
        return Err(ForgeError::GovernanceInvalid {
            reason: format!(
                "provider `{provider}` candidate {} is not a regular file",
                candidate.display()
            ),
        });
    }
    if !is_executable(&metadata) {
        return Err(ForgeError::GovernanceInvalid {
            reason: format!(
                "provider `{provider}` candidate {} is not an executable file",
                candidate.display()
            ),
        });
    }
    candidate
        .canonicalize()
        .map_err(|err| ForgeError::GovernanceInvalid {
            reason: format!(
                "provider `{provider}` candidate {} cannot be resolved ({err})",
                candidate.display()
            ),
        })
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}

pub fn list_providers(
    project_root: &Path,
) -> Result<Vec<GovernanceProviderDescriptor>, ForgeError> {
    let config = load_config(project_root)?;
    let configured = config.provider.as_ref();
    let mut descriptors = vec![GovernanceProviderDescriptor {
        provider: LOCAL_PROVIDER_ID.to_string(),
        configured: configured.is_some_and(|p| p.provider == LOCAL_PROVIDER_ID),
        enabled: true,
        adapter: None,
        protocol_version: GOVERNANCE_CONTRACT_VERSION.to_string(),
    }];
    if let Some(provider) = configured.filter(|p| p.provider != LOCAL_PROVIDER_ID) {
        descriptors.push(GovernanceProviderDescriptor {
            provider: provider.provider.clone(),
            configured: true,
            enabled: provider.enabled,
            adapter: provider.adapter.clone(),
            protocol_version: provider.protocol_version.clone(),
        });
    }
    Ok(descriptors)
}

pub fn inspect(project_root: &Path) -> Result<GovernanceObservation, ForgeError> {
    evaluate_project(project_root)
}

pub fn check_project(project_root: &Path) -> Result<GovernanceObservation, ForgeError> {
    let observation = evaluate_project(project_root)?;
    persist_observation(project_root, &observation)?;
    Ok(observation)
}

/// Evaluate a project without writing its observation history. This is used
/// by read-only transports; callers that want durable history use
/// [`check_project`].
pub fn evaluate_project(project_root: &Path) -> Result<GovernanceObservation, ForgeError> {
    let (manifest, _) = Manifest::load_from_dir(project_root, None)?;
    let config = load_config(project_root)?;
    let provider = config.provider.unwrap_or_default();
    let project_id = manifest.project.id;
    let source_revision = git_revision(project_root);
    let mut observation = if provider.provider == LOCAL_PROVIDER_ID {
        local_observation(project_root, &project_id, source_revision)
    } else if !provider.enabled {
        observation(
            &provider.provider,
            &project_id,
            ProviderStatus::Disabled,
            source_revision,
            vec![],
            Some("provider is disabled".to_string()),
            serde_json::Value::Null,
        )
    } else {
        run_external_provider(project_root, &project_id, &provider, source_revision)?
    };
    observation.project_path = project_root.display().to_string();
    Ok(observation)
}

fn local_observation(
    project_root: &Path,
    project_id: &str,
    source_revision: Option<String>,
) -> GovernanceObservation {
    observation(
        LOCAL_PROVIDER_ID,
        project_id,
        ProviderStatus::Pass,
        source_revision,
        vec![format!(
            "local manifest valid: {}",
            project_root.join("forge.yaml").display()
        )],
        Some("built-in local provider".to_string()),
        serde_json::json!({"mode": "standalone"}),
    )
}

fn run_external_provider(
    project_root: &Path,
    project_id: &str,
    config: &GovernanceProviderConfig,
    source_revision: Option<String>,
) -> Result<GovernanceObservation, ForgeError> {
    let Some(adapter) = config.adapter.as_deref() else {
        return Ok(observation(
            &config.provider,
            project_id,
            ProviderStatus::Unavailable,
            source_revision,
            vec![],
            Some(format!(
                "provider `{}` has no adapter configured",
                config.provider
            )),
            serde_json::Value::Null,
        ));
    };
    let request = serde_json::to_vec(&AdapterRequest {
        contract: GOVERNANCE_CONTRACT_VERSION,
        action: "check",
        project_id,
        project_path: &project_root.display().to_string(),
    })
    .map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot encode provider request: {err}"),
    })?;
    let output = match run_adapter(
        adapter,
        project_root,
        config.workspace_root.as_deref(),
        &request,
        config.timeout_ms,
    ) {
        Ok(output) => output,
        Err(ForgeError::GovernanceUnavailable { reason }) => {
            return Ok(observation(
                &config.provider,
                project_id,
                ProviderStatus::Unavailable,
                source_revision,
                vec![],
                Some(reason),
                serde_json::Value::Null,
            ));
        }
        Err(err) => return Err(err),
    };
    if !output.status.success() {
        return Ok(observation(
            &config.provider,
            project_id,
            ProviderStatus::Unavailable,
            source_revision,
            vec![],
            Some(format!(
                "adapter exited with {}: {}",
                output.status, output.stderr
            )),
            serde_json::Value::Null,
        ));
    }
    let raw: AdapterObservation = match serde_json::from_slice(&output.stdout) {
        Ok(raw) => raw,
        Err(err) => {
            return Ok(observation(
                &config.provider,
                project_id,
                ProviderStatus::Incompatible,
                source_revision,
                vec![],
                Some(format!("provider response is not valid JSON: {err}")),
                serde_json::Value::Null,
            ));
        }
    };
    if let Some(provider) = raw.provider.as_deref() {
        if provider != config.provider {
            return Ok(observation(
                &config.provider,
                project_id,
                ProviderStatus::Incompatible,
                source_revision,
                vec![],
                Some(format!("provider returned `{provider}`")),
                serde_json::Value::Null,
            ));
        }
    }
    if raw.protocol_version != GOVERNANCE_CONTRACT_VERSION {
        return Ok(observation(
            &config.provider,
            project_id,
            ProviderStatus::Incompatible,
            source_revision,
            vec![],
            Some(format!(
                "unsupported provider protocol `{}`",
                raw.protocol_version
            )),
            serde_json::Value::Null,
        ));
    }
    if raw.project_id != project_id {
        return Ok(observation(
            &config.provider,
            project_id,
            ProviderStatus::Incompatible,
            source_revision,
            vec![],
            Some(format!("provider returned project `{}`", raw.project_id)),
            serde_json::Value::Null,
        ));
    }
    let Some(status) = parse_status(&raw.status) else {
        return Ok(observation(
            &config.provider,
            project_id,
            ProviderStatus::Incompatible,
            source_revision,
            vec![],
            Some(format!("provider returned unknown status `{}`", raw.status)),
            serde_json::Value::Null,
        ));
    };
    let evidence = raw
        .evidence
        .into_iter()
        .map(|value| limit_text(&redact_credentials(&value), MAX_EVIDENCE_CHARS))
        .collect();
    Ok(observation(
        &config.provider,
        project_id,
        status,
        raw.source_revision.or(source_revision),
        evidence,
        raw.detail
            .map(|value| limit_text(&redact_credentials(&value), MAX_EVIDENCE_CHARS)),
        raw.metadata,
    ))
}

struct AdapterOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: String,
}

fn run_adapter(
    adapter: &str,
    project_root: &Path,
    workspace_root: Option<&str>,
    request: &[u8],
    timeout_ms: u64,
) -> Result<AdapterOutput, ForgeError> {
    let mut command = Command::new(adapter);
    command
        .current_dir(project_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A recorded workspace root is re-supplied to the adapter exactly as
    // the sibling documents its own invocation (`WORKSPACE_ROOT=<root>`),
    // so later checks do not depend on the environment staying set.
    if let Some(root) = workspace_root {
        command.env("WORKSPACE_ROOT", root);
    }
    let mut child = command
        .spawn()
        .map_err(|err| ForgeError::GovernanceUnavailable {
            reason: format!("cannot start adapter `{adapter}`: {err}"),
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request)
            .map_err(|err| ForgeError::GovernanceUnavailable {
                reason: format!("cannot write adapter request: {err}"),
            })?;
    }
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut stream) = child.stdout.take() {
                    stream.read_to_end(&mut stdout).map_err(|err| {
                        ForgeError::GovernanceUnavailable {
                            reason: format!("cannot read adapter stdout: {err}"),
                        }
                    })?;
                }
                if let Some(mut stream) = child.stderr.take() {
                    stream.read_to_end(&mut stderr).map_err(|err| {
                        ForgeError::GovernanceUnavailable {
                            reason: format!("cannot read adapter stderr: {err}"),
                        }
                    })?;
                }
                if stdout.len() > MAX_ADAPTER_OUTPUT_BYTES {
                    return Err(ForgeError::GovernanceInvalid {
                        reason: format!("adapter stdout exceeds {MAX_ADAPTER_OUTPUT_BYTES} bytes"),
                    });
                }
                return Ok(AdapterOutput {
                    status,
                    stdout,
                    stderr: limit_text(&redact_credentials(&String::from_utf8_lossy(&stderr)), 500),
                });
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(AdapterOutput {
                    status: synthetic_failure_status(),
                    stdout: Vec::new(),
                    stderr: format!("adapter exceeded timeout of {timeout_ms} ms"),
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ForgeError::GovernanceUnavailable {
                    reason: format!("adapter wait failed: {err}"),
                });
            }
        }
    }
}

fn synthetic_failure_status() -> std::process::ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(1)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(1)
    }
}

fn persist_observation(
    project_root: &Path,
    observation: &GovernanceObservation,
) -> Result<(), ForgeError> {
    let path = project_root.join(OBSERVATIONS_RELATIVE_PATH);
    let parent = path.parent().ok_or_else(|| ForgeError::GovernanceInvalid {
        reason: format!("invalid observation path {}", path.display()),
    })?;
    fs::create_dir_all(parent).map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot create {}: {err}", parent.display()),
    })?;
    let mut observations: Vec<GovernanceObservation> = if path.exists() {
        serde_json::from_slice(
            &fs::read(&path).map_err(|err| ForgeError::GovernanceInvalid {
                reason: format!("cannot read {}: {err}", path.display()),
            })?,
        )
        .map_err(|err| ForgeError::GovernanceInvalid {
            reason: format!("invalid observation history: {err}"),
        })?
    } else {
        Vec::new()
    };
    observations.push(observation.clone());
    if observations.len() > 32 {
        observations.drain(0..observations.len() - 32);
    }
    let temp = parent.join(format!(".observations.json.tmp-{}", std::process::id()));
    fs::write(
        &temp,
        serde_json::to_vec_pretty(&observations).map_err(|err| ForgeError::GovernanceInvalid {
            reason: format!("cannot serialize observation: {err}"),
        })?,
    )
    .map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot write {}: {err}", temp.display()),
    })?;
    fs::rename(&temp, &path).map_err(|err| ForgeError::GovernanceInvalid {
        reason: format!("cannot promote {}: {err}", path.display()),
    })
}

fn observation(
    provider: &str,
    project_id: &str,
    status: ProviderStatus,
    source_revision: Option<String>,
    evidence: Vec<String>,
    detail: Option<String>,
    metadata: serde_json::Value,
) -> GovernanceObservation {
    GovernanceObservation {
        provider: provider.to_string(),
        protocol_version: GOVERNANCE_CONTRACT_VERSION.to_string(),
        project_id: project_id.to_string(),
        project_path: String::new(),
        status,
        observed_at: Utc::now().to_rfc3339(),
        source_revision,
        evidence,
        detail,
        metadata,
    }
}

fn parse_status(value: &str) -> Option<ProviderStatus> {
    match value {
        "pass" => Some(ProviderStatus::Pass),
        "fail" => Some(ProviderStatus::Fail),
        "blocked" => Some(ProviderStatus::Blocked),
        "unknown" => Some(ProviderStatus::Unknown),
        "unavailable" => Some(ProviderStatus::Unavailable),
        "stale" => Some(ProviderStatus::Stale),
        "disabled" => Some(ProviderStatus::Disabled),
        "incompatible" => Some(ProviderStatus::Incompatible),
        _ => None,
    }
}

fn validate_config(config: &GovernanceConfig) -> Result<(), ForgeError> {
    if let Some(provider) = &config.provider {
        validate_provider_id(&provider.provider)?;
        if provider.protocol_version != GOVERNANCE_CONTRACT_VERSION {
            return Err(ForgeError::GovernanceInvalid {
                reason: format!(
                    "unsupported provider protocol `{}`",
                    provider.protocol_version
                ),
            });
        }
        if provider.timeout_ms == 0 || provider.timeout_ms > 300_000 {
            return Err(ForgeError::GovernanceInvalid {
                reason: "timeout_ms must be between 1 and 300000".to_string(),
            });
        }
    }
    Ok(())
}

fn validate_provider_id(provider: &str) -> Result<(), ForgeError> {
    if provider.is_empty()
        || !provider
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ForgeError::GovernanceInvalid {
            reason: format!("invalid provider id `{provider}`"),
        });
    }
    Ok(())
}

fn git_revision(project_root: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn limit_text(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn unknown_status_is_rejected() {
        assert_eq!(parse_status("healthy"), None);
    }

    #[test]
    fn normalized_failure_status_is_not_healthy() {
        assert!(!GovernanceStatus::from(ProviderStatus::Unavailable).is_healthy());
    }

    /// Build `<root>/workspace-governance/scripts/forge_governance_adapter.py`
    /// with the given executable state.
    fn staged_candidate(root: &Path, executable: bool) -> PathBuf {
        let candidate = root.join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH);
        fs::create_dir_all(candidate.parent().unwrap()).unwrap();
        fs::write(&candidate, "#!/bin/sh\nexit 0\n").unwrap();
        set_mode(&candidate, if executable { 0o755 } else { 0o644 });
        candidate
    }

    #[cfg(unix)]
    fn set_mode(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[cfg(not(unix))]
    fn set_mode(_path: &Path, _mode: u32) {}

    fn refusal(provider: &str, root: Option<&Path>, env: Option<&str>) -> String {
        match resolve_known_adapter(provider, root, env) {
            Some(Err(ForgeError::GovernanceInvalid { reason })) => reason,
            other => panic!("expected a governance-invalid refusal, got {other:?}"),
        }
    }

    #[test]
    fn providers_without_a_preset_are_left_to_the_caller() {
        assert!(resolve_known_adapter(LOCAL_PROVIDER_ID, None, None).is_none());
        assert!(resolve_known_adapter("external", Some(Path::new("/root")), None).is_none());
    }

    #[test]
    fn missing_root_refuses_naming_both_explicit_inputs() {
        let reason = refusal(WORKSPACE_GOVERNANCE_PROVIDER_ID, None, None);
        assert!(reason.contains("--workspace-root"), "{reason}");
        assert!(reason.contains(WORKSPACE_ROOT_ENV), "{reason}");
    }

    #[test]
    fn blank_env_root_is_treated_as_absent() {
        let reason = refusal(WORKSPACE_GOVERNANCE_PROVIDER_ID, None, Some("   "));
        assert!(reason.contains("--workspace-root"), "{reason}");
    }

    #[test]
    fn missing_candidate_refuses_naming_exact_path() {
        let workspace = TempDir::new().unwrap();
        let reason = refusal(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(workspace.path()),
            None,
        );
        assert!(
            reason.contains(
                &workspace
                    .path()
                    .join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH)
                    .display()
                    .to_string()
            ),
            "{reason}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_candidate_refuses_naming_exact_path() {
        let workspace = TempDir::new().unwrap();
        let candidate = staged_candidate(workspace.path(), false);
        let reason = refusal(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(workspace.path()),
            None,
        );
        assert!(reason.contains("not an executable file"), "{reason}");
        assert!(
            reason.contains(&candidate.display().to_string()),
            "{reason}"
        );
    }

    #[test]
    fn directory_candidate_refuses_as_not_regular() {
        let workspace = TempDir::new().unwrap();
        fs::create_dir_all(workspace.path().join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH)).unwrap();
        let reason = refusal(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(workspace.path()),
            None,
        );
        assert!(reason.contains("not a regular file"), "{reason}");
    }

    #[cfg(unix)]
    #[test]
    fn executable_candidate_resolves_to_a_canonical_absolute_path() {
        let workspace = TempDir::new().unwrap();
        let candidate = staged_candidate(workspace.path(), true);
        let resolved = resolve_known_adapter(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(workspace.path()),
            None,
        )
        .unwrap()
        .unwrap();
        assert!(resolved.is_absolute());
        assert_eq!(resolved, candidate.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn explicit_root_wins_over_env_and_never_falls_back() {
        let chosen = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        staged_candidate(chosen.path(), true);
        // The env root has no candidate: the flag still resolves.
        let resolved = resolve_known_adapter(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(chosen.path()),
            Some(other.path().to_str().unwrap()),
        )
        .unwrap()
        .unwrap();
        assert!(resolved.starts_with(chosen.path().canonicalize().unwrap()));
        // The flag root without a candidate refuses even when the env root
        // would hold one — the argument is authoritative, never a merge.
        let reason = refusal(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(other.path()),
            Some(chosen.path().to_str().unwrap()),
        );
        assert!(reason.contains("not an existing regular file"), "{reason}");
    }

    #[cfg(unix)]
    #[test]
    fn env_root_resolves_the_packaged_candidate() {
        let workspace = TempDir::new().unwrap();
        staged_candidate(workspace.path(), true);
        let resolved = resolve_known_adapter(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            None,
            Some(workspace.path().to_str().unwrap()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            resolved,
            workspace
                .path()
                .join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH)
                .canonicalize()
                .unwrap()
        );
    }
}
