//! External Forge publish-provider contract.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::core::ForgeError;

pub const PUBLISH_PROVIDER_CONTRACT: &str = "forge-publish-provider/0.1.0";
pub const PUBLISH_PROVIDER_TIMEOUT: Duration = Duration::from_secs(1800);

/// Bounded character length for the optional `queue_id` field carried
/// in provider requests, responses, and progress events. The bound is
/// shared with the queue state module so a single change tightens the
/// whole transport.
pub const QUEUE_ID_MAX: usize = 128;

/// Bounded character length for the optional progress `detail` field.
/// Longer values are truncated (with a trailing `…`) so a verbose
/// provider cannot blow the journal row width.
pub const PROGRESS_DETAIL_MAX: usize = 512;

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

pub fn provider_timeout() -> Duration {
    std::env::var("FORGE_PUBLISH_PROVIDER_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.clamp(60, 7200)))
        .unwrap_or(PUBLISH_PROVIDER_TIMEOUT)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOperation {
    Capabilities,
    Preflight,
    Publish,
    Verify,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishProviderRequest {
    pub contract: String,
    pub operation: ProviderOperation,
    pub provider: String,
    pub project_id: String,
    pub revision: String,
    pub operation_id: String,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
    /// Optional fleet run identifier. Carried through to the response
    /// and used by Forge to group per-project progress events under a
    /// single fleet invocation. Validated for shape (1..=128 chars,
    /// ASCII alphanumeric plus `-` and `_`) but never persisted by
    /// the provider; providers MUST echo it verbatim when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishProviderResponse {
    pub contract: String,
    pub provider: String,
    pub operation_id: String,
    pub status: String,
    pub health: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub recovery: Vec<String>,
    /// Optional echo of the request `queue_id`. Present only when the
    /// request carried one; used by Forge to match terminal responses
    /// back to the originating fleet run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProviderEntry {
    pub id: String,
    pub command: PathBuf,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct ProviderConfig {
    #[serde(default)]
    pub providers: Vec<ProviderEntry>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProviderContractError {
    #[error("provider request is not an object")]
    NotObject,
    #[error("provider contract must be {0}")]
    ContractMismatch(String),
    #[error("provider field `{0}` is required")]
    MissingField(&'static str),
    #[error("provider response contains a secret-like value")]
    SecretLeak,
    #[error("provider queue_id `{0}` is not 1..=128 ASCII alphanumeric/`-`/`_` characters")]
    QueueIdShape(String),
    #[error("provider progress event `{0}` is missing or invalid")]
    ProgressShape(&'static str),
    #[error("provider progress event claims a different operation_id than the active request")]
    ProgressOperationMismatch,
    #[error("provider progress event claims a different project_id than the active request")]
    ProgressProjectMismatch,
    #[error("provider progress event claims a different queue_id than the active request")]
    ProgressQueueMismatch,
    #[error("provider progress event claims a different contract than the active request")]
    ProgressContractMismatch,
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
        ("revision", request.revision.as_str()),
        ("operation_id", request.operation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ProviderContractError::MissingField(field));
        }
    }
    if let Some(queue_id) = request.queue_id.as_deref() {
        validate_queue_id(queue_id)?;
    }
    Ok(request)
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
    let rendered =
        serde_json::to_string(response).map_err(|_| ProviderContractError::SecretLeak)?;
    let lower = rendered.to_lowercase();
    if [
        "password=",
        "token=",
        "secret=",
        "private_key",
        "-----begin",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return Err(ProviderContractError::SecretLeak);
    }
    Ok(())
}

/// Outcome of validating a single progress event line emitted by a
/// publish provider on its stderr stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEventDecision {
    /// Event shape was valid and matched the active request envelope.
    /// Carries the bounded, redacted detail string (always bounded by
    /// [`PROGRESS_DETAIL_MAX`]).
    Accepted { detail: String },
    /// Event shape was wrong (not an object, wrong contract, missing
    /// field, malformed queue_id, …). The runtime's own reason is the
    /// payload so the operator can read what the provider claimed.
    Malformed { reason: String },
    /// Event was structurally fine but claimed a different
    /// operation_id / project_id / queue_id / contract than the
    /// active request — a provider protocol violation.
    Mismatched,
    /// Event type was something other than `publish.progress`. The
    /// line is ignored without surfacing a diagnostic so the transport
    /// stays quiet when a provider mixes informational lines.
    Ignored,
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

pub fn invoke_provider(
    entry: &ProviderEntry,
    request: &PublishProviderRequest,
    folder: &Path,
) -> Result<PublishProviderResponse, ForgeError> {
    let input = serde_json::to_vec(request).map_err(|error| ForgeError::PublishInvalid {
        reason: format!("cannot encode publish provider request: {error}"),
    })?;
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
        use std::io::Write;
        stdin
            .write_all(&input)
            .map_err(|error| ForgeError::PublishInvalid {
                reason: format!(
                    "cannot send request to publish provider `{}`: {error}",
                    entry.id
                ),
            })?;
    }
    let progress_reader = child.stderr.take().map(|stderr| {
        let provider_id = entry.id.clone();
        let operation_id = request.operation_id.clone();
        let project_id = request.project_id.clone();
        let queue_id = request.queue_id.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                if let Ok(event) = serde_json::from_str::<Value>(line.trim()) {
                    match classify_progress_event(
                        &event,
                        &operation_id,
                        &project_id,
                        queue_id.as_deref(),
                    ) {
                        ProgressEventDecision::Accepted { detail } => {
                            let phase = event
                                .get("phase")
                                .and_then(Value::as_str)
                                .unwrap_or("?");
                            let status = event
                                .get("status")
                                .and_then(Value::as_str)
                                .unwrap_or("?");
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
    if let Some(reader) = progress_reader {
        let _ = reader.join();
    }
    let response: PublishProviderResponse =
        serde_json::from_slice(&output.stdout).map_err(|error| ForgeError::PublishInvalid {
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
    if !output.status.success() {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "publish provider `{}` failed: status {}",
                entry.id, output.status
            ),
        });
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_publish_request_fixture() {
        let request = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "project_id": "alethefy",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "delivery-1",
            "folder": "/home/paul/code/alethefy",
            "dry_run": true
        }))
        .unwrap();
        assert_eq!(request.operation, ProviderOperation::Publish);
        assert!(request.dry_run);
    }

    #[test]
    fn rejects_wrong_contract() {
        let error = parse_request(json!({
            "contract": "wrong/0.1.0",
            "operation": "verify",
            "provider": "openpanel",
            "project_id": "demo",
            "revision": "abc",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(error, ProviderContractError::ContractMismatch(_)));
    }

    #[test]
    fn rejects_secret_like_response_evidence() {
        let response = PublishProviderResponse {
            contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
            provider: "openpanel".to_string(),
            operation_id: "delivery-1".to_string(),
            status: "failed".to_string(),
            health: "unknown".to_string(),
            evidence: vec!["token=leaked".to_string()],
            recovery: vec![],
            queue_id: None,
        };
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn disabled_provider_is_not_selectable() {
        let config = ProviderConfig {
            providers: vec![ProviderEntry {
                id: "jenkins".to_string(),
                command: PathBuf::from("jenkins-provider"),
                enabled: false,
            }],
        };
        assert!(select_provider(&config, "jenkins").is_err());
    }

    fn response_with(evidence: Vec<String>, recovery: Vec<String>) -> PublishProviderResponse {
        PublishProviderResponse {
            contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
            provider: "openpanel".to_string(),
            operation_id: "delivery-1".to_string(),
            status: "done".to_string(),
            health: "healthy".to_string(),
            evidence,
            recovery,
            queue_id: None,
        }
    }

    #[test]
    fn accepts_clean_response() {
        let response = response_with(
            vec!["runtime health check passed".to_string()],
            vec!["restart the runtime pod".to_string()],
        );
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn rejects_password_marker_in_evidence() {
        let response = response_with(vec!["password=hunter2hunter2".to_string()], vec![]);
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn rejects_token_marker_in_recovery() {
        let response = response_with(
            vec![],
            vec!["token=ghp_abcdefghijklmnopqrstuvwxyz0123456789".to_string()],
        );
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn rejects_pem_block_in_evidence() {
        let response = response_with(vec!["-----BEGIN RSA PRIVATE KEY-----".to_string()], vec![]);
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn rejects_private_key_marker_in_recovery() {
        let response = response_with(vec![], vec!["private_key=...".to_string()]);
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn rejects_response_with_wrong_contract() {
        let mut response = response_with(vec![], vec![]);
        response.contract = "forge-publish-provider/0.2.0".to_string();
        assert!(matches!(
            validate_response(&response),
            Err(ProviderContractError::ContractMismatch(_))
        ));
    }

    #[test]
    fn parse_request_rejects_missing_provider_field() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "project_id": "demo",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("provider")
        ));
    }

    #[test]
    fn parse_request_rejects_missing_project_id_field() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("project_id")
        ));
    }

    #[test]
    fn parse_request_rejects_missing_revision_field() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "project_id": "demo",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("revision")
        ));
    }

    #[test]
    fn parse_request_rejects_missing_operation_id_field() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "project_id": "demo",
            "revision": "0123456789abcdef0123456789abcdef01234567"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("operation_id")
        ));
    }

    #[test]
    fn parse_request_rejects_blank_provider_string() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "   ",
            "project_id": "demo",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("provider")
        ));
    }

    #[test]
    fn parse_request_rejects_non_object_payload() {
        let err = parse_request(json!("not an object")).unwrap_err();
        assert!(matches!(err, ProviderContractError::NotObject));
    }

    #[test]
    fn parse_request_supports_all_operations() {
        for op in ["capabilities", "preflight", "publish", "verify", "rollback"] {
            let request = parse_request(json!({
                "contract": PUBLISH_PROVIDER_CONTRACT,
                "operation": op,
                "provider": "openpanel",
                "project_id": "demo",
                "revision": "0123456789abcdef0123456789abcdef01234567",
                "operation_id": "delivery-1"
            }))
            .unwrap();
            assert_eq!(request.operation.as_str(), op);
        }
    }

    impl ProviderOperation {
        fn as_str(&self) -> &'static str {
            match self {
                ProviderOperation::Capabilities => "capabilities",
                ProviderOperation::Preflight => "preflight",
                ProviderOperation::Publish => "publish",
                ProviderOperation::Verify => "verify",
                ProviderOperation::Rollback => "rollback",
            }
        }
    }

    #[test]
    fn select_provider_returns_the_enabled_entry() {
        let config = ProviderConfig {
            providers: vec![
                ProviderEntry {
                    id: "openpanel".to_string(),
                    command: PathBuf::from("op"),
                    enabled: true,
                },
                ProviderEntry {
                    id: "jenkins".to_string(),
                    command: PathBuf::from("jk"),
                    enabled: false,
                },
            ],
        };
        let entry = select_provider(&config, "openpanel").unwrap();
        assert_eq!(entry.id, "openpanel");
        assert!(entry.enabled);
    }

    #[test]
    fn select_provider_refuses_unknown_id() {
        let config = ProviderConfig::default();
        let err = select_provider(&config, "missing").unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn load_config_refuses_missing_file() {
        let err = load_config(Path::new("/no/such/file.yaml")).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn load_config_refuses_invalid_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("providers.yaml");
        std::fs::write(&path, "providers: [\nunterminated").unwrap();
        let err = load_config(&path).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn load_config_round_trips_enabled_default() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("providers.yaml");
        // `enabled` is omitted to exercise the default-true serde path.
        std::fs::write(&path, "providers:\n  - id: openpanel\n    command: op\n").unwrap();
        let config = load_config(&path).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert!(config.providers[0].enabled);
    }

    #[test]
    fn validate_queue_id_accepts_alphanumeric_dash_underscore() {
        assert!(validate_queue_id("fleet-1").is_ok());
        assert!(validate_queue_id("Fleet_2026_09_27_abcdef01").is_ok());
        assert!(validate_queue_id("a").is_ok());
    }

    #[test]
    fn validate_queue_id_rejects_empty_oversized_and_unsafe() {
        assert_eq!(
            validate_queue_id(""),
            Err(ProviderContractError::QueueIdShape(String::new()))
        );
        let too_long = "x".repeat(QUEUE_ID_MAX + 1);
        assert!(matches!(
            validate_queue_id(&too_long),
            Err(ProviderContractError::QueueIdShape(_))
        ));
        assert!(matches!(
            validate_queue_id("fleet 1"),
            Err(ProviderContractError::QueueIdShape(_))
        ));
        assert!(matches!(
            validate_queue_id("fleet/1"),
            Err(ProviderContractError::QueueIdShape(_))
        ));
    }

    #[test]
    fn parse_request_carries_queue_id_through() {
        let request = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "jenkins",
            "project_id": "alpha",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "publish-alpha-ab",
            "queue_id": "fleet-1"
        }))
        .unwrap();
        assert_eq!(request.queue_id.as_deref(), Some("fleet-1"));
    }

    #[test]
    fn parse_request_rejects_malformed_queue_id() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "jenkins",
            "project_id": "alpha",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "publish-alpha-ab",
            "queue_id": "fleet 1"
        }))
        .unwrap_err();
        assert!(matches!(err, ProviderContractError::QueueIdShape(_)));
    }

    fn progress_event(
        queue_id: Option<&str>,
        operation_id: Option<&str>,
        project_id: Option<&str>,
        phase: Option<&str>,
        status: Option<&str>,
        detail: Option<&str>,
    ) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "event".to_string(),
            Value::String("publish.progress".to_string()),
        );
        obj.insert(
            "contract".to_string(),
            Value::String(PUBLISH_PROVIDER_CONTRACT.to_string()),
        );
        if let Some(q) = queue_id {
            obj.insert("queue_id".to_string(), Value::String(q.to_string()));
        }
        if let Some(o) = operation_id {
            obj.insert("operation_id".to_string(), Value::String(o.to_string()));
        }
        if let Some(p) = project_id {
            obj.insert("project_id".to_string(), Value::String(p.to_string()));
        }
        if let Some(ph) = phase {
            obj.insert("phase".to_string(), Value::String(ph.to_string()));
        }
        if let Some(s) = status {
            obj.insert("status".to_string(), Value::String(s.to_string()));
        }
        if let Some(d) = detail {
            obj.insert("detail".to_string(), Value::String(d.to_string()));
        }
        Value::Object(obj)
    }

    #[test]
    fn classify_progress_accepts_matching_event() {
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build-and-run"),
            Some("started"),
            Some("stage 1/5"),
        );
        match classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")) {
            ProgressEventDecision::Accepted { detail } => {
                assert_eq!(detail, "stage 1/5");
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn classify_progress_ignores_non_progress_event() {
        let event = json!({"event": "publish.note", "data": "noise"});
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Ignored
        );
    }

    #[test]
    fn classify_progress_rejects_wrong_contract() {
        let mut event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        event.as_object_mut().unwrap().insert(
            "contract".to_string(),
            Value::String("forge-publish-provider/0.2.0".to_string()),
        );
        assert!(matches!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Malformed { .. }
        ));
    }

    #[test]
    fn classify_progress_rejects_operation_id_mismatch() {
        let event = progress_event(
            Some("fleet-1"),
            Some("op-other"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_rejects_project_id_mismatch() {
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("beta"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_rejects_queue_id_mismatch() {
        let event = progress_event(
            Some("fleet-other"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_redacts_and_bounds_detail() {
        let long = "x".repeat(PROGRESS_DETAIL_MAX + 200);
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            Some(&format!("token=ghp_secret_in_detail {long}")),
        );
        match classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")) {
            ProgressEventDecision::Accepted { detail } => {
                assert!(detail.contains("[REDACTED]"));
                assert!(detail.ends_with('…'));
                assert!(detail.len() <= PROGRESS_DETAIL_MAX);
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn classify_progress_requires_queue_id_when_active() {
        // Active request carries queue_id but event omits it: mismatch.
        let event = progress_event(
            None,
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_rejects_event_with_queue_id_for_standalone_request() {
        // Standalone publish (no active queue_id) MUST NOT receive a
        // progress event claiming a queue_id; the provider is
        // spoofing queue membership.
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", None),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_rejects_missing_required_fields() {
        // Missing phase.
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            None,
            Some("started"),
            None,
        );
        assert!(matches!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Malformed { .. }
        ));
        // Missing status.
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            None,
            None,
        );
        assert!(matches!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Malformed { .. }
        ));
    }
}
