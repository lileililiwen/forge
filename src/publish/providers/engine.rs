//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::contract::{
    CONTAINER_IDENTITY_MAX, METADATA_PROPOSE_CONTRACT, PHASE_BUILD, PHASE_COMPLETE, PHASE_RUN,
    PHASE_STATUS_FAILED, PHASE_STATUS_NOT_STARTED, PHASE_STATUS_SUCCEEDED, PHASE_STATUS_UNKNOWN,
    PROGRESS_DETAIL_MAX, PUBLISH_PROVIDER_CONTRACT, PUBLISH_PROVIDER_TIMEOUT, QUEUE_ID_MAX,
    REVISION_LEN, REVISION_SHA12_LEN,
};
use super::model::{
    MetadataProposeRequest, MetadataProposeResponse, ProgressEventDecision, ProviderConfig,
    ProviderContractError, ProviderEntry, PublishProviderRequest, PublishProviderResponse,
};

/// Validate a queue_id string. Accepts ASCII alphanumeric characters
/// plus `-` and `_`, length 1..=[QUEUE_ID_MAX]. Empty / blank / over-
/// length / non-conforming values are refused with
/// [`ProviderContractError::QueueIdShape`].
pub fn validate_queue_id(value: &str) -> Result<(), ProviderContractError> {
    if value.is_empty() || value.len() > QUEUE_ID_MAX {
        return Err(ProviderContractError::QueueIdShape(value.to_string()));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(ProviderContractError::QueueIdShape(value.to_string()));
    }
    Ok(())
}

/// Validate a Git revision string. Accepts exactly 40 lowercase or
/// uppercase hexadecimal characters (`[0-9a-fA-F]`). Any other value
/// — empty, short, long, non-hex — is refused with
/// [`ProviderContractError::RevisionShape`] so a stale or truncated
/// SHA never reaches the journal.
pub fn validate_revision(value: &str) -> Result<(), ProviderContractError> {
    if value.len() != REVISION_LEN {
        return Err(ProviderContractError::RevisionShape(value.to_string()));
    }
    if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ProviderContractError::RevisionShape(value.to_string()));
    }
    Ok(())
}

/// Compose project / container identity for a revision-bound
/// deployment. The 12-character SHA prefix is the human/container
/// identity; the full 40-character SHA stays in Forge state so the
/// status projection never has to round-trip through the provider.
pub fn compose_project_name(project_id: &str, revision: &str) -> String {
    let prefix: String = revision.chars().take(REVISION_SHA12_LEN).collect();
    format!("forge-{project_id}-{prefix}")
}

pub fn provider_timeout() -> Duration {
    std::env::var("FORGE_PUBLISH_PROVIDER_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.clamp(60, 7200)))
        .unwrap_or(PUBLISH_PROVIDER_TIMEOUT)
}

pub(super) fn default_enabled() -> bool {
    true
}

pub fn parse_request(value: Value) -> Result<PublishProviderRequest, ProviderContractError> {
    if !value.is_object() {
        return Err(ProviderContractError::NotObject);
    }
    let request: PublishProviderRequest = serde_json::from_value(value).map_err(|error| {
        let text = error.to_string();
        for field in ["provider", "project_id", "revision", "operation_id"] {
            if text.contains(field) {
                return ProviderContractError::MissingField(field);
            }
        }
        ProviderContractError::ContractMismatch(text)
    })?;
    if request.contract != PUBLISH_PROVIDER_CONTRACT {
        return Err(ProviderContractError::ContractMismatch(request.contract));
    }
    for (field, value) in [
        ("provider", request.provider.as_str()),
        ("project_id", request.project_id.as_str()),
        ("operation_id", request.operation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ProviderContractError::MissingField(field));
        }
    }
    validate_revision(&request.revision)?;
    if let Some(queue_id) = request.queue_id.as_deref() {
        validate_queue_id(queue_id)?;
    }
    Ok(request)
}

fn validate_phase_status(field: &'static str, value: &str) -> Result<(), ProviderContractError> {
    match value {
        PHASE_STATUS_SUCCEEDED
        | PHASE_STATUS_FAILED
        | PHASE_STATUS_NOT_STARTED
        | PHASE_STATUS_UNKNOWN => Ok(()),
        _ => match field {
            "build_status" => Err(ProviderContractError::BuildStatusShape(value.to_string())),
            "run_status" => Err(ProviderContractError::RunStatusShape(value.to_string())),
            _ => Err(ProviderContractError::ProgressShape("phase_status")),
        },
    }
}

fn validate_container_identity(value: &str) -> Result<(), ProviderContractError> {
    if value.is_empty() || value.len() > CONTAINER_IDENTITY_MAX {
        return Err(ProviderContractError::ContainerIdentityShape(
            value.to_string(),
        ));
    }
    Ok(())
}

pub fn validate_response(response: &PublishProviderResponse) -> Result<(), ProviderContractError> {
    if response.contract != PUBLISH_PROVIDER_CONTRACT {
        return Err(ProviderContractError::ContractMismatch(
            response.contract.clone(),
        ));
    }
    if let Some(queue_id) = response.queue_id.as_deref() {
        validate_queue_id(queue_id)?;
    }
    if let Some(revision) = response.revision.as_deref() {
        validate_revision(revision)?;
    }
    if let Some(build_status) = response.build_status.as_deref() {
        validate_phase_status("build_status", build_status)?;
    }
    if let Some(run_status) = response.run_status.as_deref() {
        validate_phase_status("run_status", run_status)?;
    }
    if let Some(container_identity) = response.container_identity.as_deref() {
        validate_container_identity(container_identity)?;
    }
    let rendered =
        serde_json::to_string(response).map_err(|_| ProviderContractError::SecretLeak)?;
    if contains_secret_marker(&rendered) {
        return Err(ProviderContractError::SecretLeak);
    }
    Ok(())
}

/// The shared secret-leak marker check. A provider response that
/// carries a credential shape is refused before it reaches the
/// journal or the operator.
fn contains_secret_marker(rendered: &str) -> bool {
    let lower = rendered.to_lowercase();
    [
        "password=",
        "token=",
        "secret=",
        "private_key",
        "-----begin",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

/// Validate one progress event against the active request envelope
/// (`active_operation_id`, `active_project_id`, `active_queue_id`).
/// `active_*` arguments are taken verbatim from the request the
/// provider received; any one missing from the active request is
/// passed as `None` so a single-project (no-queue) publish still
/// validates. The returned [`ProgressEventDecision`] distinguishes a
/// silently-ignored informational line from a provider protocol
/// violation that must be surfaced.
pub fn classify_progress_event(
    event: &Value,
    active_operation_id: &str,
    active_project_id: &str,
    active_queue_id: Option<&str>,
) -> ProgressEventDecision {
    if !event.is_object() {
        return ProgressEventDecision::Malformed {
            reason: "event is not an object".to_string(),
        };
    }
    let event_name = event.get("event").and_then(Value::as_str);
    if event_name != Some("publish.progress") {
        return ProgressEventDecision::Ignored;
    }
    let contract = event.get("contract").and_then(Value::as_str);
    if contract != Some(PUBLISH_PROVIDER_CONTRACT) {
        return ProgressEventDecision::Malformed {
            reason: format!(
                "event contract is `{}`, expected `{}`",
                contract.unwrap_or("<missing>"),
                PUBLISH_PROVIDER_CONTRACT
            ),
        };
    }
    let event_operation_id = event.get("operation_id").and_then(Value::as_str);
    let event_project_id = event.get("project_id").and_then(Value::as_str);
    let event_queue_id = event.get("queue_id").and_then(Value::as_str);
    let event_phase = event.get("phase").and_then(Value::as_str);
    let event_status = event.get("status").and_then(Value::as_str);
    let event_detail = event.get("detail").and_then(Value::as_str).unwrap_or("");
    if event_operation_id.is_none() {
        return ProgressEventDecision::Malformed {
            reason: "event operation_id is missing".to_string(),
        };
    }
    if event_project_id.is_none() {
        return ProgressEventDecision::Malformed {
            reason: "event project_id is missing".to_string(),
        };
    }
    if event_phase.is_none() {
        return ProgressEventDecision::Malformed {
            reason: "event phase is missing".to_string(),
        };
    }
    if event_status.is_none() {
        return ProgressEventDecision::Malformed {
            reason: "event status is missing".to_string(),
        };
    }
    let phase = event_phase.unwrap_or("");
    if !matches!(phase, PHASE_BUILD | PHASE_RUN | PHASE_COMPLETE) {
        return ProgressEventDecision::Malformed {
            reason: format!("event phase `{phase}` is not one of `build`/`run`/`complete`"),
        };
    }
    let status = event_status.unwrap_or("");
    if !matches!(
        status,
        PHASE_STATUS_SUCCEEDED
            | PHASE_STATUS_FAILED
            | PHASE_STATUS_NOT_STARTED
            | PHASE_STATUS_UNKNOWN
            | "started"
    ) {
        return ProgressEventDecision::Malformed {
            reason: format!(
                "event status `{status}` is not one of \
                 `started`/`succeeded`/`failed`/`not_started`/`unknown`"
            ),
        };
    }
    if event_operation_id != Some(active_operation_id) {
        return ProgressEventDecision::Mismatched;
    }
    if event_project_id != Some(active_project_id) {
        return ProgressEventDecision::Mismatched;
    }
    if let Some(active_q) = active_queue_id {
        match event_queue_id {
            Some(eq) if eq == active_q => {}
            _ => return ProgressEventDecision::Mismatched,
        }
        if validate_queue_id(active_q).is_err() {
            return ProgressEventDecision::Malformed {
                reason: format!("active queue_id `{active_q}` is not 1..=128 ASCII"),
            };
        }
    } else if event_queue_id.is_some() {
        return ProgressEventDecision::Mismatched;
    }
    let truncated = if event_detail.len() > PROGRESS_DETAIL_MAX {
        let mut end = PROGRESS_DETAIL_MAX.saturating_sub(1);
        while end > 0 && !event_detail.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &event_detail[..end])
    } else {
        event_detail.to_string()
    };
    ProgressEventDecision::Accepted {
        detail: crate::policy::redact_credentials(&truncated),
    }
}

pub fn load_config(path: &Path) -> Result<ProviderConfig, ForgeError> {
    let bytes = std::fs::read(path).map_err(|error| ForgeError::PublishInvalid {
        reason: format!(
            "cannot read publish provider config {}: {error}",
            path.display()
        ),
    })?;
    serde_yaml::from_slice(&bytes).map_err(|error| ForgeError::PublishInvalid {
        reason: format!(
            "invalid publish provider config {}: {error}",
            path.display()
        ),
    })
}

pub fn select_provider(config: &ProviderConfig, id: &str) -> Result<ProviderEntry, ForgeError> {
    let entry = config
        .providers
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| ForgeError::PublishInvalid {
            reason: format!("publish provider `{id}` is not configured"),
        })?;
    if !entry.enabled {
        return Err(ForgeError::PublishInvalid {
            reason: format!("publish provider `{id}` is disabled"),
        });
    }
    Ok(entry.clone())
}

/// Run one provider process: bounded argv (the configured command
/// with no extra arguments), JSON on stdin, stdout collected,
/// stderr lines handed to `on_stderr_line`, per-run timeout, and
/// the same kill-on-timeout discipline for every caller. This is
/// the single process-management core both the publish transport
/// and the metadata transport use, so a metadata request gets the
/// same timeout and secret-leak discipline as a publish request.
fn run_provider_process(
    entry: &ProviderEntry,
    input: &[u8],
    folder: &Path,
    on_stderr_line: impl Fn(&str) + Send + 'static,
) -> Result<(Vec<u8>, std::process::ExitStatus), ForgeError> {
    let mut child = Command::new(&entry.command)
        .current_dir(folder)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ForgeError::PublishInvalid {
            reason: format!("cannot start publish provider `{}`: {error}", entry.id),
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(error) = crate::process::write_request(&mut stdin, input) {
            // A genuine write failure leaves the provider running; reap it
            // before reporting, so no live provider is left behind.
            let _ = child.kill();
            let _ = child.wait();
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "cannot send request to publish provider `{}`: {error}",
                    entry.id
                ),
            });
        }
    }
    let stderr_reader = child.stderr.take().map(|stderr| {
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                on_stderr_line(line.trim());
                line.clear();
            }
        })
    });
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > provider_timeout() => {
                let _ = child.kill();
                return Err(ForgeError::PublishInvalid {
                    reason: format!("publish provider `{}` timed out", entry.id),
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                return Err(ForgeError::PublishInvalid {
                    reason: format!("publish provider `{}` wait failed: {error}", entry.id),
                })
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "cannot collect publish provider `{}` output: {error}",
                entry.id
            ),
        })?;
    if let Some(reader) = stderr_reader {
        let _ = reader.join();
    }
    Ok((output.stdout, output.status))
}

pub fn invoke_provider(
    entry: &ProviderEntry,
    request: &PublishProviderRequest,
    folder: &Path,
) -> Result<PublishProviderResponse, ForgeError> {
    let input = serde_json::to_vec(request).map_err(|error| ForgeError::PublishInvalid {
        reason: format!("cannot encode publish provider request: {error}"),
    })?;
    let provider_id = entry.id.clone();
    let operation_id = request.operation_id.clone();
    let project_id = request.project_id.clone();
    let queue_id = request.queue_id.clone();
    let (stdout, status) = run_provider_process(entry, &input, folder, move |line| {
        if let Ok(event) = serde_json::from_str::<Value>(line) {
            match classify_progress_event(&event, &operation_id, &project_id, queue_id.as_deref()) {
                ProgressEventDecision::Accepted { detail } => {
                    let phase = event.get("phase").and_then(Value::as_str).unwrap_or("?");
                    let status = event.get("status").and_then(Value::as_str).unwrap_or("?");
                    eprintln!(
                        "forge publish provider={provider_id} project={project_id} phase={phase} status={status} {detail}"
                    );
                }
                ProgressEventDecision::Malformed { reason } => {
                    eprintln!(
                        "forge publish provider={provider_id} project={project_id} progress malformed: {reason}"
                    );
                }
                ProgressEventDecision::Mismatched => {
                    eprintln!(
                        "forge publish provider={provider_id} progress event id mismatch; ignoring line"
                    );
                }
                ProgressEventDecision::Ignored => {}
            }
        }
    })?;
    let response: PublishProviderResponse =
        serde_json::from_slice(&stdout).map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "publish provider `{}` returned invalid JSON: {error}",
                entry.id
            ),
        })?;
    validate_response(&response).map_err(|error| ForgeError::PublishInvalid {
        reason: format!(
            "publish provider `{}` returned invalid response: {error}",
            entry.id
        ),
    })?;
    if !status.success() {
        return Err(ForgeError::PublishInvalid {
            reason: format!("publish provider `{}` failed: status {}", entry.id, status),
        });
    }
    Ok(response)
}

/// Validate a metadata propose response: the contract must match
/// and the serialized response must not carry a credential shape.
pub fn validate_metadata_response(
    response: &MetadataProposeResponse,
) -> Result<(), ProviderContractError> {
    if response.contract != METADATA_PROPOSE_CONTRACT {
        return Err(ProviderContractError::ContractMismatch(
            response.contract.clone(),
        ));
    }
    let rendered =
        serde_json::to_string(response).map_err(|_| ProviderContractError::SecretLeak)?;
    if contains_secret_marker(&rendered) {
        return Err(ProviderContractError::SecretLeak);
    }
    Ok(())
}

/// Invoke a metadata plugin over the same transport as
/// [`invoke_provider`]: same bounded argv, same per-run timeout,
/// same secret-leak rejection. Only the request/response schema
/// differs.
pub fn invoke_metadata_provider(
    entry: &ProviderEntry,
    request: &MetadataProposeRequest,
    folder: &Path,
) -> Result<MetadataProposeResponse, ForgeError> {
    let input = serde_json::to_vec(request).map_err(|error| ForgeError::PublishInvalid {
        reason: format!("cannot encode metadata propose request: {error}"),
    })?;
    let provider_id = entry.id.clone();
    let (stdout, status) = run_provider_process(entry, &input, folder, move |line| {
        if !line.is_empty() {
            eprintln!("forge metadata provider={provider_id}: {line}");
        }
    })?;
    let response: MetadataProposeResponse =
        serde_json::from_slice(&stdout).map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "metadata provider `{}` returned invalid JSON: {error}",
                entry.id
            ),
        })?;
    validate_metadata_response(&response).map_err(|error| ForgeError::PublishInvalid {
        reason: format!(
            "metadata provider `{}` returned invalid response: {error}",
            entry.id
        ),
    })?;
    if !status.success() {
        return Err(ForgeError::PublishInvalid {
            reason: format!("metadata provider `{}` failed: status {}", entry.id, status),
        });
    }
    Ok(response)
}
