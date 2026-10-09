//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use chrono::Utc;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::contract::{
    CONFIG_RELATIVE_PATH, DEFAULT_TIMEOUT_MS, EXIT_RECHECK, GOVERNANCE_CONTRACT_VERSION,
    LOCAL_PROVIDER_ID, MAX_ADAPTER_OUTPUT_BYTES, MAX_EVIDENCE_CHARS, MAX_GIT_REVISION_BYTES,
    OBSERVATIONS_RELATIVE_PATH, TRUNCATED_DRAIN_MARKER, WORKSPACE_GOVERNANCE_ADAPTER_RELPATH,
    WORKSPACE_GOVERNANCE_PROVIDER_ID, WORKSPACE_ROOT_ENV,
};
use super::model::{
    AdapterObservation, AdapterOutput, AdapterRequest, BoundedRun, GovernanceConfig,
    GovernanceObservation, GovernanceProviderConfig, GovernanceProviderDescriptor, Pipe, PipeDrain,
    ProviderStatus,
};

pub(super) fn default_enabled() -> bool {
    true
}

pub(super) fn default_protocol_version() -> String {
    GOVERNANCE_CONTRACT_VERSION.to_string()
}

pub(super) fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
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
pub(super) fn run_bounded(
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
///
/// The rule itself is [`crate::process::write_request`], shared with the other
/// three request boundaries; this is only the governance-specific mapping from
/// an `io::Error` to the typed refusal.
pub(super) fn write_adapter_request(
    stdin: &mut impl Write,
    request: &[u8],
) -> Result<(), ForgeError> {
    crate::process::write_request(stdin, request).map_err(|err| ForgeError::GovernanceUnavailable {
        reason: format!("cannot write adapter request: {err}"),
    })
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

pub(super) fn parse_status(value: &str) -> Option<ProviderStatus> {
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
pub(super) fn git_revision(project_root: &Path, timeout_ms: u64) -> Option<String> {
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
