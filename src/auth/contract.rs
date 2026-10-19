//! The client half of the `forge-identity-provider/1.0` contract.
//!
//! Core speaks the contract to a pack over standard input and standard output. The
//! contract is a versioned process boundary, so Core keeps its own copy of the wire
//! types instead of linking the provider module. Every response is validated before
//! it is trusted, and no response may carry a secret value.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The contract identifier Core and a pack agree on.
pub const CONTRACT_ID: &str = "forge-identity-provider/1.0";

/// The operation Core asks a pack to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOperation {
    /// Report what the provider can do for the project.
    Probe,
    /// Produce a side-effect free plan.
    Plan,
    /// Perform the planned change, bound to the plan digest.
    Apply,
    /// Report the current state of the provider population.
    Status,
    /// Stop the provider runtime for the project.
    Stop,
}

impl ProviderOperation {
    /// The wire name of the operation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Probe => "probe",
            Self::Plan => "plan",
            Self::Apply => "apply",
            Self::Status => "status",
            Self::Stop => "stop",
        }
    }
}

/// The terminal status of a provider operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    /// The operation completed and the population is usable.
    Ready,
    /// The operation cannot proceed without an owner decision.
    Blocked,
    /// The operation failed.
    Failed,
    /// The provider or its runtime is not reachable.
    Unavailable,
}

impl ProviderStatus {
    /// The wire name of the status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }
}

/// One change a plan will make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanChange {
    /// The change kind.
    pub kind: String,
    /// The file or provider object the change touches.
    pub target: String,
    /// A secret-free summary.
    pub summary: String,
}

/// The end-user population a provider created for one project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    /// The provider-scoped population identifier.
    pub population_id: String,
    /// The OIDC issuer of the population.
    pub issuer: String,
    /// The OIDC client id of the population.
    pub client_id: String,
    /// The callback path the provider redirects to.
    pub callback_path: String,
    /// The scopes the client requests.
    #[serde(default)]
    pub scopes: Vec<String>,
    /// The registration page of the population, when the provider has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_url: Option<String>,
}

/// A recovery action the owner can take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// A stable reason code.
    pub reason: String,
    /// A secret-free instruction.
    pub instruction: String,
}

/// One request line written to a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRequest {
    /// The contract identifier.
    pub contract: String,
    /// The provider pack that owns the request.
    pub provider_id: String,
    /// The caller-chosen operation identifier.
    pub operation_id: String,
    /// The project the operation belongs to.
    pub project_id: String,
    /// The project revision the operation is bound to.
    pub project_revision: String,
    /// The operation.
    pub operation: ProviderOperation,
    /// The operation input. Never carries a secret value.
    #[serde(default)]
    pub input: Value,
}

/// One response line read from a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    /// The contract identifier.
    pub contract: String,
    /// The provider pack that answered.
    pub provider_id: String,
    /// The operation identifier from the request.
    pub operation_id: String,
    /// The project from the request.
    pub project_id: String,
    /// The project revision from the request.
    pub project_revision: String,
    /// The operation that was answered.
    pub operation: ProviderOperation,
    /// The terminal status.
    pub status: ProviderStatus,
    /// A secret-free summary.
    #[serde(default)]
    pub summary: String,
    /// The digest of the plan, for `plan` and `apply`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_digest: Option<String>,
    /// The changes the plan will make.
    #[serde(default)]
    pub proposed_changes: Vec<PlanChange>,
    /// The population the operation produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population: Option<Population>,
    /// References to the secrets the population uses.
    #[serde(default)]
    pub secret_references: Vec<String>,
    /// The recovery actions of a blocked or failed operation.
    #[serde(default)]
    pub recovery: Vec<RecoveryAction>,
    /// Provider specific detail.
    #[serde(default)]
    pub details: Value,
}

/// The contract violations Core refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    /// The pack answered with a different contract.
    UnsupportedContract(String),
    /// The pack answered for another request.
    IdentityMismatch(String),
    /// The pack answered with secret material.
    SecretMaterial(String),
    /// The pack answered with an unsafe URL.
    UnsafeUrl(String),
    /// The pack answered without the plan digest the operation must carry.
    MissingPlanDigest,
    /// The pack could not be run, or did not answer.
    Unavailable(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContract(found) => {
                write!(f, "the provider answered with contract '{found}'")
            }
            Self::IdentityMismatch(reason) => {
                write!(f, "the provider answered for another request: {reason}")
            }
            Self::SecretMaterial(field) => {
                write!(
                    f,
                    "the provider answer carries secret material in '{field}'"
                )
            }
            Self::UnsafeUrl(url) => write!(f, "the provider answer carries an unsafe url '{url}'"),
            Self::MissingPlanDigest => {
                write!(f, "the provider answer carries no plan digest")
            }
            Self::Unavailable(reason) => write!(f, "the provider is unavailable: {reason}"),
        }
    }
}

impl std::error::Error for ContractError {}

/// Validates a response against the request that produced it.
pub fn validate_response(
    request: &ProviderRequest,
    response: &ProviderResponse,
) -> Result<(), ContractError> {
    if response.contract != CONTRACT_ID {
        return Err(ContractError::UnsupportedContract(
            response.contract.clone(),
        ));
    }
    if response.provider_id != request.provider_id {
        return Err(ContractError::IdentityMismatch(format!(
            "provider '{}' answered for provider '{}'",
            response.provider_id, request.provider_id
        )));
    }
    if response.operation_id != request.operation_id {
        return Err(ContractError::IdentityMismatch(format!(
            "operation '{}' answered for operation '{}'",
            response.operation_id, request.operation_id
        )));
    }
    if response.project_id != request.project_id {
        return Err(ContractError::IdentityMismatch(format!(
            "project '{}' answered for project '{}'",
            response.project_id, request.project_id
        )));
    }
    if response.project_revision != request.project_revision {
        return Err(ContractError::IdentityMismatch(
            "the project revision changed between the request and the answer".to_string(),
        ));
    }
    if response.operation != request.operation {
        return Err(ContractError::IdentityMismatch(format!(
            "operation '{}' answered for operation '{}'",
            response.operation.as_str(),
            request.operation.as_str()
        )));
    }

    if looks_secretful(&response.summary) {
        return Err(ContractError::SecretMaterial("summary".to_string()));
    }
    for change in &response.proposed_changes {
        if looks_secretful(&change.summary) || looks_secretful(&change.target) {
            return Err(ContractError::SecretMaterial(
                "proposed_changes".to_string(),
            ));
        }
    }
    for reference in &response.secret_references {
        if !is_reference(reference) {
            return Err(ContractError::SecretMaterial(
                "secret_references".to_string(),
            ));
        }
    }
    for action in &response.recovery {
        if looks_secretful(&action.instruction) {
            return Err(ContractError::SecretMaterial("recovery".to_string()));
        }
    }

    if let Some(population) = &response.population {
        if population.population_id.trim().is_empty() {
            return Err(ContractError::IdentityMismatch(
                "the population carries no identifier".to_string(),
            ));
        }
        if population.client_id.trim().is_empty() {
            return Err(ContractError::IdentityMismatch(
                "the population carries no client id".to_string(),
            ));
        }
        if !population.callback_path.starts_with('/') {
            return Err(ContractError::IdentityMismatch(
                "the population carries a callback path that is not absolute".to_string(),
            ));
        }
        if !is_safe_url(&population.issuer) {
            return Err(ContractError::UnsafeUrl(population.issuer.clone()));
        }
        if let Some(registration) = &population.registration_url {
            if !is_safe_url(registration) {
                return Err(ContractError::UnsafeUrl(registration.clone()));
            }
            if !same_site(&population.issuer, registration) {
                return Err(ContractError::UnsafeUrl(registration.clone()));
            }
        }
    }

    if request.operation == ProviderOperation::Plan && response.plan_digest.is_none() {
        return Err(ContractError::MissingPlanDigest);
    }

    if let Ok(value) = serde_json::to_value(response) {
        scan_for_secret_fields(&value, "response")?;
    }

    Ok(())
}

/// Scans a raw provider answer for secret material.
///
/// The scan runs on the raw answer, before the typed response drops the fields
/// the contract does not define. A pack that sends a secret in an unknown field
/// is still refused.
pub fn scan_answer_for_secrets(value: &serde_json::Value) -> Result<(), ContractError> {
    scan_for_secret_fields(value, "response")
}

/// Walks a provider answer and refuses a field that carries secret material.
///
/// A reference is allowed, a value is not: the contract carries references only.
fn scan_for_secret_fields(value: &serde_json::Value, path: &str) -> Result<(), ContractError> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}.{key}");
                if looks_secretful(key) && !key.contains("references") {
                    let carries_a_value = match child {
                        serde_json::Value::String(text) => !is_reference(text),
                        serde_json::Value::Null => false,
                        _ => true,
                    };
                    if carries_a_value {
                        return Err(ContractError::SecretMaterial(child_path));
                    }
                }
                scan_for_secret_fields(child, &child_path)?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                scan_for_secret_fields(item, &format!("{path}[{index}]"))?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The substrings that mark a value as a secret.
pub const SECRET_MARKERS: &[&str] = &[
    "secret",
    "password",
    "passwd",
    "api_key",
    "apikey",
    "private_key",
    "client_secret",
    "access_token",
    "refresh_token",
    "bearer ",
    "authorization",
];

/// True when the text carries a secret marker.
pub fn looks_secretful(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    SECRET_MARKERS.iter().any(|marker| lowered.contains(marker))
}

fn is_reference(reference: &str) -> bool {
    reference.starts_with("env://")
        || reference.starts_with("file://")
        || reference.starts_with("secret://")
        || reference.starts_with("config://")
}

fn is_safe_url(url: &str) -> bool {
    if let Some(rest) = url.strip_prefix("https://") {
        return !rest.is_empty();
    }
    if let Some(rest) = url.strip_prefix("http://") {
        let host = rest.split('/').next().unwrap_or_default();
        let host = host.split(':').next().unwrap_or_default();
        return host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1";
    }
    false
}

fn same_site(issuer: &str, other: &str) -> bool {
    let host = |url: &str| -> Option<String> {
        let rest = url.split_once("://")?.1;
        let authority = rest.split('/').next()?;
        Some(authority.to_string())
    };
    match (host(issuer), host(other)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

/// Computes the digest of a plan.
pub fn plan_digest(
    project_id: &str,
    provider_id: &str,
    operation: &str,
    changes: &[PlanChange],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(provider_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(operation.as_bytes());
    hasher.update(b"\n");
    for change in changes {
        hasher.update(change.kind.as_bytes());
        hasher.update(b"\n");
        hasher.update(change.target.as_bytes());
        hasher.update(b"\n");
        hasher.update(change.summary.as_bytes());
        hasher.update(b"\n");
    }
    format!("sha256:{:x}", hasher.finalize())
}

/// The digest of a project revision, used to bind a plan to the project state.
pub fn project_revision(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}
