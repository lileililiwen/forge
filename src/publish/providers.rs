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
    Ok(request)
}

pub fn validate_response(response: &PublishProviderResponse) -> Result<(), ProviderContractError> {
    if response.contract != PUBLISH_PROVIDER_CONTRACT {
        return Err(ProviderContractError::ContractMismatch(
            response.contract.clone(),
        ));
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
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                if let Ok(event) = serde_json::from_str::<Value>(line.trim()) {
                    if event.get("event").and_then(Value::as_str) == Some("publish.progress") {
                        let project = event.get("project_id").and_then(Value::as_str).unwrap_or("?");
                        let phase = event.get("phase").and_then(Value::as_str).unwrap_or("?");
                        let status = event.get("status").and_then(Value::as_str).unwrap_or("?");
                        let detail = event.get("detail").and_then(Value::as_str).unwrap_or("");
                        eprintln!(
                            "forge publish provider={provider_id} project={project} phase={phase} status={status} {detail}"
                        );
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
}
