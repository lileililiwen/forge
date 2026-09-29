//! `forge-delivery-hermora/0.1.0` adapter contract.
//!
//! The adapter is a local executable that receives the bounded
//! request envelope on stdin and answers a bounded response envelope
//! on stdout. Forge defines the wire contract; the adapter
//! implementation is owned by Hermora and lives outside this
//! repository.
//!
//! Adapter resolution order:
//!  1. `FORGE_HERMORA_BIN` (absolute path or executable on `PATH`)
//!  2. `forge-hermora-adapter` on `PATH`
//!
//! Wall-clock timeout: `FORGE_HERMORA_TIMEOUT_SECS` (default 60s,
//! clamped to `1..=600`).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant as TimeInstant};

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;

pub const HERMORA_CONTRACT: &str = "forge-delivery-hermora/0.1.0";

/// Adapter invocation timeout. A hostile or wedged adapter cannot
/// pin the CLI or API beyond the budget.
pub const HERMORA_TIMEOUT_SECS: u64 = 60;
pub const HERMORA_TIMEOUT_MIN_SECS: u64 = 1;
pub const HERMORA_TIMEOUT_MAX_SECS: u64 = 600;

/// Bounded request envelope written to the adapter's stdin.
#[derive(Debug, Clone, Serialize)]
pub struct HermoraRequest {
    pub contract: &'static str,
    pub operation: &'static str,
    pub project_id: String,
    pub environment: String,
    pub revision: String,
    pub deployment_url: String,
    /// Operator-supplied reference to an environment variable that
    /// holds the Hermora token. Never a token value.
    pub secret_ref: String,
}

/// Bounded response envelope read from the adapter's stdout.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HermoraResponse {
    pub contract: String,
    pub operation: String,
    /// Terminal status — the adapter must answer either
    /// `connected` or `failed`. Anything else is a contract
    /// violation and is refused without echoing the value.
    pub status: String,
    /// Hermora-side site id, present on success.
    #[serde(default)]
    pub site_id: Option<String>,
    /// Public URL the adapter enrolled, present on success.
    #[serde(default)]
    pub environment_url: Option<String>,
    /// Bounded reason string on failure (already scrubbed by the
    /// adapter).
    #[serde(default)]
    pub reason: Option<String>,
}

/// Outcome of one Hermora adapter invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HermoraOutcome {
    Connected {
        site_id: String,
        environment_url: String,
    },
    Failed {
        reason: String,
    },
    Unavailable {
        reason: String,
    },
}

impl HermoraOutcome {
    pub fn label(&self) -> &'static str {
        match self {
            HermoraOutcome::Connected { .. } => "connected",
            HermoraOutcome::Failed { .. } => "failed",
            HermoraOutcome::Unavailable { .. } => "unavailable",
        }
    }
}

/// Resolve the adapter binary path. Returns `None` when neither the
/// explicit override nor the canonical default is reachable, so the
/// caller can answer a typed `Unavailable` outcome without the
/// CLI claiming an empty success.
pub fn resolve_adapter(override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(raw) = override_path {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    if let Some(env) = std::env::var_os("FORGE_HERMORA_BIN") {
        let trimmed = env.to_string_lossy().trim().to_string();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    if let Some(found) = first_binary_on_path(&["forge-hermora-adapter"]) {
        return Some(PathBuf::from(found));
    }
    None
}

fn first_binary_on_path(candidates: &[&str]) -> Option<String> {
    let path_env = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path_env) {
        for candidate in candidates {
            let full = entry.join(candidate);
            if full.is_file() {
                return Some(full.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Read `FORGE_HERMORA_TIMEOUT_SECS`, clamped into the bounded
/// budget. A misconfigured environment value (zero, negative, or
/// over the ceiling) is silently clamped rather than raised —
/// operators running in a sandboxed shell get the slowest acceptable
/// adapter call rather than a refusal.
pub fn adapter_timeout() -> Duration {
    let secs = std::env::var("FORGE_HERMORA_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(HERMORA_TIMEOUT_SECS)
        .clamp(HERMORA_TIMEOUT_MIN_SECS, HERMORA_TIMEOUT_MAX_SECS);
    Duration::from_secs(secs)
}

/// Invoke the Hermora adapter. Never carries a Forge session, a
/// registry byte, or a credential value: the `secret_ref` is the
/// name of an environment variable the adapter reads itself.
pub fn invoke(
    adapter: &std::path::Path,
    request: &HermoraRequest,
    timeout: Duration,
) -> Result<HermoraOutcome, ForgeError> {
    let payload = serde_json::to_vec(request).map_err(|err| ForgeError::DeliveryUnavailable {
        reason: format!("hermora request could not be encoded: {err}"),
    })?;

    let mut child = Command::new(adapter)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| ForgeError::DeliveryUnavailable {
            reason: format!(
                "hermora adapter `{}` could not be started: {err}",
                adapter.display()
            ),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        // A broken pipe here means the adapter exited early; the
        // exit-status surface below reports the real cause rather
        // than the EOF.
        let _ = stdin.write_all(&payload);
    }

    let deadline = TimeInstant::now() + timeout;
    let output = loop {
        match child.try_wait() {
            Ok(Some(_)) => break child.wait_with_output(),
            Ok(None) => {
                if TimeInstant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ForgeError::DeliveryUnavailable {
                        reason: format!(
                            "hermora adapter `{}` exceeded its {}s budget; \
                             the approved deployment is retained and the \
                             onboarding attempt is retryable",
                            adapter.display(),
                            timeout.as_secs()
                        ),
                    });
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                let _ = child.kill();
                return Err(ForgeError::DeliveryUnavailable {
                    reason: format!(
                        "hermora adapter `{}` could not be waited on: {err}",
                        adapter.display()
                    ),
                });
            }
        }
    };

    let output = output.map_err(|err| ForgeError::DeliveryUnavailable {
        reason: format!(
            "hermora adapter `{}` produced no readable response: {err}",
            adapter.display()
        ),
    })?;

    if !output.status.success() {
        return Ok(HermoraOutcome::Unavailable {
            reason: format!(
                "hermora adapter exited with status {}",
                output.status.code().unwrap_or(-1)
            ),
        });
    }

    parse_response(&output.stdout).map_err(|reason| ForgeError::DeliveryInvalid {
        reason: format!("hermora adapter response: {reason}"),
    })
}

/// Parse one adapter response envelope. Bounded so a verbose
/// adapter cannot pin the writer.
pub fn parse_response(bytes: &[u8]) -> Result<HermoraOutcome, String> {
    const RESPONSE_BYTES_MAX: usize = 16 * 1024;
    if bytes.len() > RESPONSE_BYTES_MAX {
        return Err(format!(
            "hermora adapter response is larger than {RESPONSE_BYTES_MAX} bytes"
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "hermora adapter response is not valid UTF-8".to_string())?;
    let response: HermoraResponse = serde_json::from_str(text)
        .map_err(|err| format!("response is not the hermora contract JSON: {err}"))?;
    if response.contract != HERMORA_CONTRACT {
        return Err(format!(
            "hermora adapter answered contract `{}`; expected `{HERMORA_CONTRACT}`",
            response.contract
        ));
    }
    match response.status.as_str() {
        "connected" => {
            let site_id = response
                .site_id
                .ok_or_else(|| "connected response is missing `site_id`".to_string())?;
            let environment_url = response
                .environment_url
                .ok_or_else(|| "connected response is missing `environment_url`".to_string())?;
            if site_id.is_empty() || site_id.len() > 256 {
                return Err(format!(
                    "hermora site_id `{}` is empty or longer than 256 chars",
                    site_id
                ));
            }
            if environment_url.is_empty() || environment_url.len() > 2048 {
                return Err(
                    "hermora environment_url is empty or longer than 2048 chars".to_string()
                );
            }
            if crate::semantic::proposal::looks_like_credential(&site_id)
                || crate::semantic::proposal::looks_like_credential(&environment_url)
            {
                return Err(
                    "hermora response carried a credential-shaped value; refusing without echoing"
                        .to_string(),
                );
            }
            Ok(HermoraOutcome::Connected {
                site_id,
                environment_url,
            })
        }
        "failed" => {
            let reason = response
                .reason
                .ok_or_else(|| "failed response is missing `reason`".to_string())?;
            if reason.len() > 512 {
                return Err(format!(
                    "hermora failure reason is {} chars; bound is 512",
                    reason.len()
                ));
            }
            Ok(HermoraOutcome::Failed { reason })
        }
        other => Err(format!(
            "hermora response status `{other}` is not one of `connected`/`failed`"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> HermoraRequest {
        HermoraRequest {
            contract: HERMORA_CONTRACT,
            operation: "register",
            project_id: "alpha".to_string(),
            environment: "production".to_string(),
            revision: "0123456789abcdef0123456789abcdef01234567".to_string(),
            deployment_url: "https://alpha.example.com".to_string(),
            secret_ref: "env:HERMORA_TOKEN_ALPHA".to_string(),
        }
    }

    #[test]
    fn parses_a_connected_response() {
        let body = br#"{"contract":"forge-delivery-hermora/0.1.0","operation":"register","status":"connected","site_id":"site_abc","environment_url":"https://alpha.hermora.example"}"#;
        let outcome = parse_response(body).expect("parse");
        assert_eq!(
            outcome,
            HermoraOutcome::Connected {
                site_id: "site_abc".to_string(),
                environment_url: "https://alpha.hermora.example".to_string(),
            }
        );
    }

    #[test]
    fn parses_a_failed_response_with_reason() {
        let body = br#"{"contract":"forge-delivery-hermora/0.1.0","operation":"register","status":"failed","reason":"site already enrolled"}"#;
        let outcome = parse_response(body).expect("parse");
        assert_eq!(
            outcome,
            HermoraOutcome::Failed {
                reason: "site already enrolled".to_string()
            }
        );
    }

    #[test]
    fn rejects_a_wrong_contract_envelope() {
        let body = br#"{"contract":"something/0.1.0","operation":"register","status":"connected","site_id":"a","environment_url":"b"}"#;
        let err = parse_response(body).unwrap_err();
        assert!(err.contains("hermora adapter answered contract"));
    }

    #[test]
    fn rejects_an_unknown_status_label() {
        let body = br#"{"contract":"forge-delivery-hermora/0.1.0","operation":"register","status":"pending"}"#;
        let err = parse_response(body).unwrap_err();
        assert!(err.contains("not one of"));
    }

    #[test]
    fn rejects_a_credential_shaped_site_id() {
        let body = br#"{"contract":"forge-delivery-hermora/0.1.0","operation":"register","status":"connected","site_id":"ghp_xxxx","environment_url":"https://a"}"#;
        let err = parse_response(body).unwrap_err();
        assert!(err.contains("credential-shaped"));
    }

    #[test]
    fn rejects_an_oversized_response() {
        let body = vec![b'x'; 17 * 1024];
        let err = parse_response(&body).unwrap_err();
        assert!(err.contains("larger than"));
    }

    #[test]
    fn timeout_is_clamped_into_the_bounded_budget() {
        // Zero / negative / over the ceiling all clamp.
        std::env::set_var("FORGE_HERMORA_TIMEOUT_SECS", "0");
        assert_eq!(adapter_timeout().as_secs(), HERMORA_TIMEOUT_MIN_SECS);
        std::env::set_var("FORGE_HERMORA_TIMEOUT_SECS", "100000");
        assert_eq!(adapter_timeout().as_secs(), HERMORA_TIMEOUT_MAX_SECS);
        std::env::remove_var("FORGE_HERMORA_TIMEOUT_SECS");
        assert_eq!(adapter_timeout().as_secs(), HERMORA_TIMEOUT_SECS);
    }

    #[test]
    fn request_envelope_is_serializable() {
        let payload = serde_json::to_string(&request()).expect("encode");
        assert!(payload.contains("forge-delivery-hermora/0.1.0"));
        assert!(payload.contains("register"));
        assert!(payload.contains("env:HERMORA_TOKEN_ALPHA"));
    }
}
