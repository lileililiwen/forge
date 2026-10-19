//! Contract validation.
//!
//! Both halves of the contract are validated before they are trusted. A request or a
//! response that does not match the contract is refused, never partially applied.

use crate::redaction::looks_secretful;
use crate::request::{ProviderRequest, CONTRACT_ID, CONTRACT_MAJOR};
use crate::response::ProviderResponse;

/// The contract violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The contract identifier is not the one this build speaks.
    UnsupportedContract(String),
    /// A required field is empty.
    MissingField(&'static str),
    /// A field does not follow the required shape.
    InvalidField(&'static str, String),
    /// The response answered a different request.
    IdentityMismatch(String),
    /// The response carried a secret value.
    SecretMaterial(String),
    /// A URL is not acceptable.
    UnsafeUrl(String),
    /// The response is missing the plan digest it must carry.
    MissingPlanDigest,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContract(found) => {
                write!(
                    f,
                    "unsupported contract '{found}', expected '{CONTRACT_ID}'"
                )
            }
            Self::MissingField(field) => write!(f, "missing field '{field}'"),
            Self::InvalidField(field, reason) => write!(f, "invalid field '{field}': {reason}"),
            Self::IdentityMismatch(reason) => write!(f, "contract identity mismatch: {reason}"),
            Self::SecretMaterial(field) => {
                write!(f, "field '{field}' carries secret material")
            }
            Self::UnsafeUrl(url) => write!(f, "unsafe url '{url}'"),
            Self::MissingPlanDigest => write!(f, "the response carries no plan digest"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validates a request before it is sent to a pack.
pub fn validate_request(request: &ProviderRequest) -> Result<(), ValidationError> {
    check_contract(&request.contract)?;
    check_identifier("provider_id", &request.provider_id)?;
    check_identifier("operation_id", &request.operation_id)?;
    check_identifier("project_id", &request.project_id)?;
    if request.project_revision.trim().is_empty() {
        return Err(ValidationError::MissingField("project_revision"));
    }
    Ok(())
}

/// Validates a response against the request that produced it.
pub fn validate_response(
    request: &ProviderRequest,
    response: &ProviderResponse,
) -> Result<(), ValidationError> {
    check_contract(&response.contract)?;
    if response.provider_id != request.provider_id {
        return Err(ValidationError::IdentityMismatch(format!(
            "provider '{}' answered for provider '{}'",
            response.provider_id, request.provider_id
        )));
    }
    if response.operation_id != request.operation_id {
        return Err(ValidationError::IdentityMismatch(format!(
            "operation '{}' answered for operation '{}'",
            response.operation_id, request.operation_id
        )));
    }
    if response.project_id != request.project_id {
        return Err(ValidationError::IdentityMismatch(format!(
            "project '{}' answered for project '{}'",
            response.project_id, request.project_id
        )));
    }
    if response.project_revision != request.project_revision {
        return Err(ValidationError::IdentityMismatch(
            "the project revision changed between the request and the response".to_string(),
        ));
    }
    if response.operation != request.operation {
        return Err(ValidationError::IdentityMismatch(format!(
            "operation '{}' answered for operation '{}'",
            response.operation.as_str(),
            request.operation.as_str()
        )));
    }

    if looks_secretful(&response.summary) {
        return Err(ValidationError::SecretMaterial("summary".to_string()));
    }
    for change in &response.proposed_changes {
        if looks_secretful(&change.summary) || looks_secretful(&change.target) {
            return Err(ValidationError::SecretMaterial(
                "proposed_changes".to_string(),
            ));
        }
    }
    for reference in &response.secret_references {
        if !is_reference(reference) {
            return Err(ValidationError::SecretMaterial(
                "secret_references".to_string(),
            ));
        }
    }
    for action in &response.recovery {
        if looks_secretful(&action.instruction) {
            return Err(ValidationError::SecretMaterial("recovery".to_string()));
        }
    }

    if let Some(population) = &response.population {
        check_population(population)?;
    }

    if matches!(request.operation, crate::request::ProviderOperation::Plan)
        && response.plan_digest.is_none()
    {
        return Err(ValidationError::MissingPlanDigest);
    }

    Ok(())
}

fn check_population(population: &crate::response::Population) -> Result<(), ValidationError> {
    if population.population_id.trim().is_empty() {
        return Err(ValidationError::MissingField("population_id"));
    }
    if population.client_id.trim().is_empty() {
        return Err(ValidationError::MissingField("client_id"));
    }
    if !population.callback_path.starts_with('/') {
        return Err(ValidationError::InvalidField(
            "callback_path",
            "the callback path must be absolute".to_string(),
        ));
    }
    check_url("issuer", &population.issuer)?;
    if let Some(registration) = &population.registration_url {
        check_url("registration_url", registration)?;
        if !same_site(&population.issuer, registration) {
            return Err(ValidationError::UnsafeUrl(registration.clone()));
        }
    }
    Ok(())
}

fn check_contract(contract: &str) -> Result<(), ValidationError> {
    if contract != CONTRACT_ID {
        return Err(ValidationError::UnsupportedContract(contract.to_string()));
    }
    let _ = CONTRACT_MAJOR;
    Ok(())
}

fn check_identifier(field: &'static str, value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::MissingField(field));
    }
    if value.chars().any(char::is_whitespace) {
        return Err(ValidationError::InvalidField(
            field,
            "the value must not contain whitespace".to_string(),
        ));
    }
    Ok(())
}

fn is_reference(reference: &str) -> bool {
    reference.starts_with("env://")
        || reference.starts_with("file://")
        || reference.starts_with("secret://")
        || reference.starts_with("config://")
}

fn check_url(field: &'static str, url: &str) -> Result<(), ValidationError> {
    let parsed = url::Url::parse(url).map_err(|_| {
        ValidationError::InvalidField(field, "the value is not an absolute URL".to_string())
    })?;
    match parsed.scheme() {
        "https" => Ok(()),
        "http" if parsed.host_str().map(is_loopback).unwrap_or(false) => Ok(()),
        _ => Err(ValidationError::UnsafeUrl(url.to_string())),
    }
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

fn same_site(issuer: &str, other: &str) -> bool {
    let left = url::Url::parse(issuer).ok();
    let right = url::Url::parse(other).ok();
    match (left, right) {
        (Some(left), Some(right)) => {
            left.host_str() == right.host_str()
                && left.port_or_known_default() == right.port_or_known_default()
        }
        _ => false,
    }
}
