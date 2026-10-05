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
/// Bound on the bytes read from the revision lookup. `git rev-parse HEAD`
/// answers with one object name — 40 hex characters, or 64 under SHA-256 — so
/// anything beyond this is not the answer, and an unbounded read from a child
/// process would be the very defect this lookup is being bounded to fix.
const MAX_GIT_REVISION_BYTES: usize = 4 * 1024;
/// Appended to the adapter's stderr when a descendant kept a drained pipe open
/// past the deadline, so an incomplete read is visible instead of silent.
const TRUNCATED_DRAIN_MARKER: &str = " [adapter output pipe still open at the deadline]";
/// Re-check interval for the one window where both pipes are at end-of-file but
/// the child has not been reaped yet. Documented at its only use in
/// `run_bounded`: end-of-file is not an exit event, so that window has no
/// thread left to wake the waiter.
const EXIT_RECHECK: Duration = Duration::from_millis(1);

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
    let source_revision = git_revision(project_root, provider.timeout_ms);
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
        if let Err(err) = write_adapter_request(&mut stdin, request) {
            // A genuine write failure leaves the child running; reap it before
            // reporting, exactly as the deadline path below does.
            let _ = child.kill();
            let _ = child.wait();
            return Err(err);
        }
    }
    // The write end is closed before anything is waited on, so an adapter that
    // reads its request sees end-of-input and one that never reads it has
    // already been tolerated above.
    drop(child.stdin.take());
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let run = run_bounded(
        &mut child,
        deadline,
        MAX_ADAPTER_OUTPUT_BYTES,
        MAX_ADAPTER_OUTPUT_BYTES,
    )?;
    if run.timed_out {
        return Ok(AdapterOutput {
            status: synthetic_failure_status(),
            stdout: Vec::new(),
            stderr: format!("adapter exceeded timeout of {timeout_ms} ms"),
        });
    }
    if let Some(err) = run.stdout.error {
        return Err(ForgeError::GovernanceUnavailable {
            reason: format!("cannot read adapter stdout: {err}"),
        });
    }
    if let Some(err) = run.stderr.error {
        return Err(ForgeError::GovernanceUnavailable {
            reason: format!("cannot read adapter stderr: {err}"),
        });
    }
    if run.stdout.total > MAX_ADAPTER_OUTPUT_BYTES {
        return Err(ForgeError::GovernanceInvalid {
            reason: format!("adapter stdout exceeds {MAX_ADAPTER_OUTPUT_BYTES} bytes"),
        });
    }
    let mut stderr = limit_text(
        &redact_credentials(&String::from_utf8_lossy(&run.stderr.bytes)),
        500,
    );
    if run.truncated {
        stderr = format!("{stderr}{TRUNCATED_DRAIN_MARKER}");
    }
    Ok(AdapterOutput {
        status: run.status,
        stdout: run.stdout.bytes,
        stderr,
    })
}

/// One drained pipe.
///
/// `bytes` holds the first `cap` bytes and `total` counts **every** byte the
/// child wrote, so the caller can enforce its cap without the reader having to
/// stop. That distinction is the whole point: a reader that stopped at `cap`
/// would block the child on its next write and re-create the deadlock this
/// drain exists to remove.
#[derive(Clone, Debug)]
struct PipeDrain {
    bytes: Vec<u8>,
    total: usize,
    error: Option<String>,
}

/// The outcome of one bounded run: the child's exit status, both drained pipes,
/// and whether Forge stopped waiting first.
#[derive(Debug)]
struct BoundedRun {
    status: std::process::ExitStatus,
    stdout: PipeDrain,
    stderr: PipeDrain,
    timed_out: bool,
    /// A descendant inherited a pipe and still held it open past the deadline,
    /// so its bytes are incomplete. Recorded rather than assumed away.
    truncated: bool,
}

/// Which pipe a drain result belongs to.
#[derive(Clone, Copy)]
enum Pipe {
    Stdout,
    Stderr,
}

fn index_of(pipe: Pipe) -> usize {
    match pipe {
        Pipe::Stdout => 0,
        Pipe::Stderr => 1,
    }
}

/// Read one pipe to end on its own thread, keeping at most `cap` bytes, and
/// report the result when it reaches end-of-file.
///
/// Draining continues past the cap on purpose: the child must be able to run to
/// completion and Forge still needs its exit status. A send failure only means
/// the waiter has already given up, which it has already handled.
fn drain_pipe<R: Read + Send + 'static>(
    mut stream: R,
    cap: usize,
    pipe: Pipe,
    events: std::sync::mpsc::Sender<(Pipe, PipeDrain)>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut total = 0usize;
        let mut error = None;
        let mut chunk = [0u8; 8192];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    total += read;
                    if bytes.len() < cap {
                        let room = cap - bytes.len();
                        bytes.extend_from_slice(&chunk[..read.min(room)]);
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(err) => {
                    error = Some(err.to_string());
                    break;
                }
            }
        }
        let _ = events.send((
            pipe,
            PipeDrain {
                bytes,
                total,
                error,
            },
        ));
    })
}

/// Wait for `child` to exit, bounded by `deadline`, while both of its pipes
/// are drained on their own threads.
///
/// The wait is a blocking receive, not a poll: it wakes when a pipe reaches
/// end-of-file or when the deadline arrives, and never in between. `try_wait`
/// is kept rather than moving `child.wait()` into a thread precisely because it
/// reports the exit **without reaping**, which leaves this function holding the
/// `Child` and therefore able to `kill` **and** `wait` on every failure path. A
/// waiter-thread design would have to kill by pid and lose that guarantee.
fn run_bounded(
    child: &mut std::process::Child,
    deadline: Instant,
    stdout_cap: usize,
    stderr_cap: usize,
) -> Result<BoundedRun, ForgeError> {
    let (events_tx, events_rx) = std::sync::mpsc::channel::<(Pipe, PipeDrain)>();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    // The spare sender keeps the channel connected for the whole call, so
    // `recv_timeout` always blocks for real instead of returning
    // `Disconnected` immediately and spinning once both readers are done.
    let _events_guard = events_tx.clone();
    // The reader handles are deliberately dropped, not joined. A reader's end of
    // the job is to report on its channel, which is what the bounded receive
    // below waits for; joining it would wait instead for *every* process
    // holding the pipe to close it. `/bin/sh -c 'sleep 30'` forks, so killing
    // the child leaves a descendant holding the write end — measured: the pipe
    // stayed open the full 30 s after the child was killed. Joining there would
    // make this function unbounded in exactly the case it exists to bound. A
    // detached reader ends when its pipe does, and the partial read is reported
    // as `truncated` rather than waited for.
    let _readers = [
        stdout.map(|stream| drain_pipe(stream, stdout_cap, Pipe::Stdout, events_tx.clone())),
        stderr.map(|stream| drain_pipe(stream, stderr_cap, Pipe::Stderr, events_tx.clone())),
    ];

    let mut drains: [Option<PipeDrain>; 2] = [None, None];
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(err) => {
                terminate(child);
                return Err(ForgeError::GovernanceUnavailable {
                    reason: format!("adapter wait failed: {err}"),
                });
            }
        }
        let now = Instant::now();
        if now >= deadline {
            terminate(child);
            timed_out = true;
            break synthetic_failure_status();
        }
        // A pipe reaching end-of-file is not the same event as the child being
        // reaped. A forking shell reaches end-of-file when *its* child exits and
        // then lives a few microseconds longer, so once both pipes are done
        // there is no further event to wake this thread and the wait would run
        // out the whole budget on an adapter that already answered — measured:
        // every run then reported `exceeded timeout`. In that one narrow window
        // the wait therefore falls back to a short re-check. Everywhere else
        // the receive blocks for the full remaining budget and the wake-up *is*
        // the event.
        let slice = if drains.iter().all(|drain| drain.is_some()) {
            EXIT_RECHECK.min(deadline - now)
        } else {
            deadline - now
        };
        match events_rx.recv_timeout(slice) {
            Ok((pipe, drain)) => drains[index_of(pipe)] = Some(drain),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if drains.iter().all(|drain| drain.is_some()) {
                    continue;
                }
                terminate(child);
                timed_out = true;
                break synthetic_failure_status();
            }
            // Unreachable while the spare sender above is held; treated as a
            // wake-up so a future change cannot turn it into a spin.
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
        }
    };

    // The child is reaped, so a drain that has not reported by the deadline was
    // held open by a descendant that outlived it. That is reported, never waited
    // on: an unbounded wait here would be the defect this function exists to
    // remove.
    while drains.iter().any(|drain| drain.is_none()) {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        match events_rx.recv_timeout(deadline - now) {
            Ok((pipe, drain)) => drains[index_of(pipe)] = Some(drain),
            Err(_) => break,
        }
    }
    let truncated = drains.iter().any(|drain| drain.is_none());
    let empty = PipeDrain {
        bytes: Vec::new(),
        total: 0,
        error: None,
    };
    Ok(BoundedRun {
        status,
        stdout: drains[0].take().unwrap_or_else(|| empty.clone()),
        stderr: drains[1].take().unwrap_or(empty),
        timed_out,
        truncated,
    })
}

/// Terminate and reap a child that outlived its budget. Both halves matter:
/// the kill stops it, the wait clears the zombie.
fn terminate(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Write the request to the adapter's standard input.
///
/// A `BrokenPipe` is the one write error that carries information about the
/// *adapter* rather than about Forge's plumbing: it can only be raised because
/// the adapter closed its input. An adapter that answers without reading its
/// request is a legitimate provider, so losing that race is Forge's timing, not
/// a provider failure — the caller proceeds to read the exit status and output
/// the adapter actually produced. Every other error keeps the typed
/// unavailable refusal.
fn write_adapter_request(stdin: &mut impl Write, request: &[u8]) -> Result<(), ForgeError> {
    match stdin.write_all(request) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(ForgeError::GovernanceUnavailable {
            reason: format!("cannot write adapter request: {err}"),
        }),
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

/// Resolve the project's recorded source revision.
///
/// The lookup runs under the same bounded-wait discipline as the adapter
/// itself, using the selected provider's existing `timeout_ms` — the value
/// [`validate_config`] already constrains and the value `run_adapter` already
/// uses — rather than a second, invented bound. A lookup that hangs, fails or
/// answers nothing usable yields no revision, exactly as before; what changes is
/// that a hanging `git` can no longer hang the check.
fn git_revision(project_root: &Path, timeout_ms: u64) -> Option<String> {
    let mut command = Command::new("git");
    command
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().ok()?;
    let run = run_bounded(
        &mut child,
        Instant::now() + Duration::from_millis(timeout_ms),
        MAX_GIT_REVISION_BYTES,
        MAX_GIT_REVISION_BYTES,
    )
    .ok()?;
    if !run.status.success() {
        return None;
    }
    let revision = String::from_utf8_lossy(&run.stdout.bytes)
        .trim()
        .to_string();
    if revision.is_empty() {
        None
    } else {
        Some(revision)
    }
}

fn limit_text(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// End-of-file is not an exit event, and this is the **deterministic** guard
    /// for that. The child closes both pipes itself and then lives on for a
    /// while, so end-of-file provably precedes the exit instead of racing it —
    /// which is why a realistic `printf` child is not enough here: closing its
    /// own pipes and exiting leaves a window a few microseconds wide, and a
    /// waiter that only woke on end-of-file passed the `printf` guard while
    /// still failing real adapters 2–4 times per run.
    ///
    /// A waiter that blocks out its budget once there is nothing left to wake it
    /// reports a timeout for a child that answered in 200 ms.
    #[cfg(unix)]
    #[test]
    fn a_child_that_closed_its_pipes_and_keeps_running_is_not_reported_as_a_timeout() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "exec 1>&- 2>&-; sleep 0.2; exit 0"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();

        let started = Instant::now();
        let run = run_bounded(
            &mut child,
            started + Duration::from_secs(5),
            MAX_GIT_REVISION_BYTES,
            MAX_GIT_REVISION_BYTES,
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert!(
            run.status.success(),
            "a child that closed its pipes and then exited 0 must not be reported as killed"
        );
        assert!(!run.timed_out, "200 ms of work is not a 5 s timeout");
        assert!(elapsed < Duration::from_secs(2), "elapsed {elapsed:?}");
    }

    /// A revision lookup that never answers gives up inside its bound instead
    /// of hanging the check forever.
    ///
    /// Before the fix the lookup was `Command::output()`, which blocks until the
    /// child exits with no timeout at all — so this command never returns and
    /// the caller waits indefinitely. The bound here is the same one
    /// `git_revision` receives: the selected provider's existing `timeout_ms`.
    #[cfg(unix)]
    #[test]
    fn a_revision_lookup_that_never_answers_gives_up_within_its_bound() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "sleep 30"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();

        let started = Instant::now();
        let run =
            run_bounded(&mut child, started + Duration::from_millis(200), 4096, 4096).unwrap();
        let elapsed = started.elapsed();

        assert!(
            run.timed_out,
            "a 30 s child must not report success: {run:?}"
        );
        // Generous next to the 200 ms bound so a loaded machine cannot turn a
        // passing run into a failure, while still being far below the 30 s the
        // child would otherwise take. Pre-fix this assertion is never reached:
        // the call does not return.
        assert!(
            elapsed < Duration::from_secs(10),
            "the bound did not take effect: {elapsed:?}"
        );
    }

    /// The positive control for the guard above, and the guard for the subtler
    /// half of the same defect: a pipe reaching end-of-file is **not** the child
    /// being reaped. `/bin/sh -c 'printf …'` forks on this host, so both pipes
    /// are done microseconds before the shell itself exits. A waiter that only
    /// woke on end-of-file and then blocked for the rest of its budget would
    /// report a timeout for an adapter that had already answered — measured,
    /// that is exactly what happened.
    #[cfg(unix)]
    #[test]
    fn a_bounded_revision_lookup_returns_the_object_name_it_printed() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "printf 'a1b2c3d4\\n'"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();

        let started = Instant::now();
        let run = run_bounded(
            &mut child,
            started + Duration::from_secs(5),
            MAX_GIT_REVISION_BYTES,
            MAX_GIT_REVISION_BYTES,
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert!(!run.timed_out, "a child that answered cannot time out");
        assert!(run.status.success());
        assert!(!run.truncated);
        assert_eq!(
            String::from_utf8_lossy(&run.stdout.bytes).trim(),
            "a1b2c3d4"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "answering took {elapsed:?} of a 5 s budget: the waiter slept past the exit"
        );
    }

    /// Bounding the lookup must not turn it into one that always reports no
    /// revision: a real repository's head is still recorded.
    #[test]
    fn git_revision_still_reports_the_repository_head() {
        let repo = TempDir::new().unwrap();
        let head = git_revision(repo.path(), 10_000);
        // A fresh temp directory is not a repository, so the honest assertion
        // here is the `None` mapping; the recorded head is exercised through
        // `evaluate_project` in `tests/governance_contract.rs`. What this pins
        // is that the bounded call runs and returns rather than panicking or
        // blocking.
        assert_eq!(head, None);

        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        if !root.join(".git").exists() {
            return;
        }
        let revision = git_revision(root, 10_000).expect("this repository's head");
        assert_eq!(revision.len(), 40, "{revision}");
        assert!(
            revision.chars().all(|c| c.is_ascii_hexdigit()),
            "{revision}"
        );
    }

    #[test]
    fn unknown_status_is_rejected() {
        assert_eq!(parse_status("healthy"), None);
    }

    #[test]
    fn normalized_failure_status_is_not_healthy() {
        assert!(!GovernanceStatus::from(ProviderStatus::Unavailable).is_healthy());
    }

    /// An adapter that answers without reading its request is a legitimate
    /// provider, so Forge losing the write race against it must not become an
    /// `unavailable` observation.
    ///
    /// Deterministic by construction: the child is reaped *before* the write,
    /// so the read end of the pipe is provably closed. An end-to-end test
    /// cannot establish that ordering, because `run_adapter` writes
    /// immediately after spawning and never waits first.
    #[cfg(unix)]
    #[test]
    fn a_request_write_to_an_adapter_that_already_exited_is_not_a_failure() {
        let mut child = Command::new("/bin/sh")
            .args(["-c", "exec 0<&-; exit 0"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());

        // Before the fix this was `Err(cannot write adapter request: Broken
        // pipe (os error 32))`, which `run_external_provider` reports as an
        // `Unavailable` observation while discarding the adapter's answer.
        write_adapter_request(&mut stdin, br#"{"action":"check"}"#).unwrap();
    }

    /// A write failure that is not the adapter closing its input is still a
    /// real inability to talk to the adapter, and keeps its typed refusal.
    #[test]
    fn a_request_write_failure_that_is_not_a_broken_pipe_still_refuses() {
        struct Failing;
        impl Write for Failing {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let refusal = write_adapter_request(&mut Failing, b"{}").unwrap_err();
        assert!(
            matches!(refusal, ForgeError::GovernanceUnavailable { .. }),
            "{refusal:?}"
        );
        assert!(refusal.to_string().contains("cannot write adapter request"));
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
