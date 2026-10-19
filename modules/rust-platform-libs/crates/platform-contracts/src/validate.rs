//! Compatibility validator.
//!
//! Mirrors the policy implemented by the `platform-contracts`
//! repository's Python validator:
//!
//! 1. The envelope MUST contain `contract`, `generated_at`, `payload`.
//! 2. The `contract` field MUST match `^platform\.[a-z0-9-]+/<semver>$`.
//! 3. The contract family MUST be known to the registry.
//! 4. The major version MUST be in the entry's
//!    `supported_major_versions`. Minor and patch values are
//!    tolerated, matching the consumer rule
//!    "consumers MUST keep accepting the previous minor of the same
//!    major".
//! 5. The payload MUST validate against the per-family JSON Schema
//!    (Draft 2020-12).
//! 6. The redaction policy rejects any field name that contains a
//!    substring listed in the registry's `secret_field_substrings`,
//!    at any depth, including inside arrays.
//! 7. The compatibility layer emits a `compatibility.security_field_added`
//!    warning when a payload adds a field under a security-sensitive
//!    namespace declared by the registry.
//!
//! The Rust validator is intentionally a strict subset of the Python
//! one; it does not invent new policy. Unknown additive non-security
//! fields are tolerated because the JSON Schemas declare
//! `additionalProperties: true` for every contract family in the
//! `platform-contracts` repository.

use std::fs;

use serde::{Deserialize, Serialize};

use crate::envelope::parse_contract_identifier;
use crate::registry::ContractRegistry;

/// Error code for a missing envelope field.
pub const CODE_ENVELOPE_MISSING: &str = "envelope.missing_field";
/// Error code for a top-level type violation.
pub const CODE_ENVELOPE_TYPE: &str = "envelope.type";
/// Error code for a malformed contract identifier.
pub const CODE_CONTRACT_FORMAT: &str = "contract.identifier_format";
/// Error code for an unknown contract family.
pub const CODE_CONTRACT_UNKNOWN: &str = "contract.unknown_family";
/// Error code for an unsupported major version.
pub const CODE_VERSION_UNSUPPORTED: &str = "version.unsupported_major";
/// Error code for a malformed version segment.
pub const CODE_VERSION_FORMAT: &str = "version.format";
/// Error code for a missing required payload field.
pub const CODE_PAYLOAD_MISSING: &str = "payload.missing_required";
/// Error code for a payload type violation.
pub const CODE_PAYLOAD_TYPE: &str = "payload.type";
/// Error code for an invalid enum value.
pub const CODE_PAYLOAD_ENUM: &str = "payload.enum";
/// Error code for a pattern violation.
pub const CODE_PAYLOAD_PATTERN: &str = "payload.pattern";
/// Error code for a format violation.
pub const CODE_PAYLOAD_FORMAT: &str = "payload.format";
/// Error code for a maxLength violation.
pub const CODE_PAYLOAD_MAX_LENGTH: &str = "payload.max_length";
/// Error code for a minLength violation.
pub const CODE_PAYLOAD_MIN_LENGTH: &str = "payload.min_length";
/// Error code for a redaction (secret-named field) violation.
pub const CODE_REDACTION_SECRET: &str = "redaction.secret_field";
/// Warning code for an additive field under a security-sensitive namespace.
pub const CODE_UNKNOWN_SECURITY_FIELD: &str = "compatibility.security_field_added";

/// A single, structured validation failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    /// Stable error code (e.g. `payload.missing_required`).
    pub code: String,
    /// JSON path of the failure (e.g. `$.payload.subject_id`).
    pub path: String,
    /// Human-readable message.
    pub message: String,
}

/// A non-fatal compatibility observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Stable warning code.
    pub code: String,
    /// JSON path of the warning.
    pub path: String,
    /// Human-readable message.
    pub message: String,
}

/// Outcome of validating a single document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    /// True iff there are no [`ValidationError`] entries.
    pub valid: bool,
    /// Structured errors collected during validation.
    pub errors: Vec<ValidationError>,
    /// Structured warnings collected during validation.
    pub warnings: Vec<ValidationWarning>,
    /// Parsed contract family, if the envelope parsed successfully.
    pub contract_family: Option<String>,
    /// Parsed `(major, minor, patch)`, if the envelope parsed.
    pub contract_version: Option<(u32, u32, u32)>,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            contract_family: None,
            contract_version: None,
        }
    }

    fn add_error(&mut self, code: &str, path: &str, message: impl Into<String>) {
        self.errors.push(ValidationError {
            code: code.to_string(),
            path: path.to_string(),
            message: message.into(),
        });
        self.valid = false;
    }

    fn add_warning(&mut self, code: &str, path: &str, message: impl Into<String>) {
        self.warnings.push(ValidationWarning {
            code: code.to_string(),
            path: path.to_string(),
            message: message.into(),
        });
    }
}

/// Validate a top-level document. The document is expected to be the
/// serialized form of an [`crate::envelope::Envelope`] (a JSON object
/// with `contract`, `generated_at`, and `payload` fields).
///
/// # Errors
///
/// Returns `Err` only when the per-family JSON Schema cannot be read
/// or compiled; missing or invalid documents are reported in
/// [`ValidationResult::errors`] rather than as a `Result::Err`.
pub fn validate_document(
    document: &serde_json::Value,
    registry: &ContractRegistry,
) -> Result<ValidationResult, ValidateError> {
    let mut result = ValidationResult::new();

    let object = match document.as_object() {
        Some(object) => object,
        None => {
            result.add_error(
                CODE_ENVELOPE_TYPE,
                "$",
                format!(
                    "Top-level document must be an object, got {}",
                    type_name(document)
                ),
            );
            return Ok(result);
        }
    };

    for required in ["contract", "generated_at", "payload"] {
        if !object.contains_key(required) {
            result.add_error(
                CODE_ENVELOPE_MISSING,
                "$",
                format!("Document is missing required envelope field {required:?}"),
            );
        }
    }
    if !result.valid {
        return Ok(result);
    }

    let contract = match object.get("contract").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            result.add_error(
                CODE_CONTRACT_FORMAT,
                "$.contract",
                "Contract identifier must be a string",
            );
            return Ok(result);
        }
    };

    let (family, version) = match parse_contract_identifier(&contract) {
        Ok(parsed) => parsed,
        Err(err) => {
            result.add_error(CODE_CONTRACT_FORMAT, "$.contract", err.to_string());
            return Ok(result);
        }
    };
    result.contract_family = Some(family.clone());
    result.contract_version = Some((version.major, version.minor, version.patch));

    let entry = match registry.get(&family) {
        Some(e) => e,
        None => {
            result.add_error(
                CODE_CONTRACT_UNKNOWN,
                "$.contract",
                format!(
                    "Unknown contract family {family:?}; supported families: {}",
                    registry.contract_ids().collect::<Vec<_>>().join(", ")
                ),
            );
            scan_for_secrets(document, registry, &mut result, "$");
            return Ok(result);
        }
    };

    if !entry.supports_major(version.major) {
        result.add_error(
            CODE_VERSION_UNSUPPORTED,
            "$.contract",
            format!(
                "Contract {family:?} major version {} is not supported; supported majors: [{}]",
                version.major,
                entry
                    .supported_major_versions
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        );
    }

    let payload = match object.get("payload") {
        Some(serde_json::Value::Object(_)) => object.get("payload").unwrap(),
        Some(_) => {
            result.add_error(CODE_PAYLOAD_TYPE, "$.payload", "Payload must be an object");
            return Ok(result);
        }
        None => return Ok(result),
    };

    validate_payload_against_schema(payload, entry, registry, &mut result)?;
    scan_for_secrets(document, registry, &mut result, "$");
    scan_for_security_field_additions(payload, entry, &mut result, "$.payload");

    Ok(result)
}

/// Validate a bare payload object against a known contract family.
///
/// # Errors
///
/// Returns `Err` when the per-family JSON Schema cannot be read or
/// compiled.
pub fn validate_payload(
    payload: &serde_json::Value,
    family: &str,
    registry: &ContractRegistry,
) -> Result<ValidationResult, ValidateError> {
    let mut result = ValidationResult::new();
    result.contract_family = Some(family.to_string());

    let entry = match registry.get(family) {
        Some(e) => e,
        None => {
            result.add_error(
                CODE_CONTRACT_UNKNOWN,
                "$",
                format!("Unknown contract family {family:?}"),
            );
            return Ok(result);
        }
    };

    if !payload.is_object() {
        result.add_error(CODE_PAYLOAD_TYPE, "$", "Payload must be an object");
        return Ok(result);
    }

    validate_payload_against_schema(payload, entry, registry, &mut result)?;
    scan_for_secrets(payload, registry, &mut result, "$");
    scan_for_security_field_additions(payload, entry, &mut result, "$");
    Ok(result)
}

/// Error raised when the per-family schema cannot be loaded.
#[derive(Debug, thiserror::Error)]
pub enum ValidateError {
    /// I/O error reading the schema file.
    #[error("I/O error reading schema {path}: {source}")]
    Io {
        /// Path that was attempted.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// JSON parse error in the schema file.
    #[error("schema {path} is not valid JSON: {source}")]
    SchemaJson {
        /// Path that was attempted.
        path: String,
        /// Underlying parse error.
        #[source]
        source: serde_json::Error,
    },
    /// JSON Schema compilation error.
    #[error("schema {path} failed to compile: {message}")]
    SchemaCompile {
        /// Path that was attempted.
        path: String,
        /// Underlying compile error message.
        message: String,
    },
}

fn validate_payload_against_schema(
    payload: &serde_json::Value,
    entry: &crate::registry::ContractEntry,
    registry: &ContractRegistry,
    result: &mut ValidationResult,
) -> Result<(), ValidateError> {
    let schema_path = registry.root.join(&entry.schema);
    let path_str = schema_path.to_string_lossy().into_owned();

    let schema_text = fs::read_to_string(&schema_path).map_err(|source| ValidateError::Io {
        path: path_str.clone(),
        source,
    })?;
    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_text).map_err(|source| ValidateError::SchemaJson {
            path: path_str.clone(),
            source,
        })?;
    let compiled = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&schema_value)
        .map_err(|err| ValidateError::SchemaCompile {
            path: path_str.clone(),
            message: err.to_string(),
        })?;

    for error in compiled.iter_errors(payload) {
        let code = map_jsonschema_code(error.kind()).to_string();
        let pointer = error.instance_path().as_str();
        let dotted = pointer_to_dotted(pointer);
        let path = if dotted.is_empty() {
            "$.payload".to_string()
        } else {
            format!("$.payload{dotted}")
        };
        result.add_error(&code, &path, error.to_string());
    }
    Ok(())
}

/// Convert a JSON pointer (e.g. `/subject_id` or `/checks/0/name`)
/// into the dotted form used in error paths (e.g. `.subject_id` or
/// `.checks[0].name`).
fn pointer_to_dotted(pointer: &str) -> String {
    if pointer.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for segment in pointer.split('/').skip(1) {
        if let Ok(idx) = segment.parse::<u64>() {
            out.push('[');
            out.push_str(&idx.to_string());
            out.push(']');
        } else {
            out.push('.');
            out.push_str(segment);
        }
    }
    out
}

fn map_jsonschema_code(kind: &jsonschema::error::ValidationErrorKind) -> &'static str {
    use jsonschema::error::ValidationErrorKind;
    match kind {
        ValidationErrorKind::Required { .. } => CODE_PAYLOAD_MISSING,
        ValidationErrorKind::Enum { .. } => CODE_PAYLOAD_ENUM,
        ValidationErrorKind::Pattern { .. } | ValidationErrorKind::RegexEngineFailure { .. } => {
            CODE_PAYLOAD_PATTERN
        }
        ValidationErrorKind::Format { .. } => CODE_PAYLOAD_FORMAT,
        ValidationErrorKind::Maximum { .. }
        | ValidationErrorKind::Minimum { .. }
        | ValidationErrorKind::ExclusiveMaximum { .. }
        | ValidationErrorKind::ExclusiveMinimum { .. } => CODE_PAYLOAD_FORMAT,
        ValidationErrorKind::MaxLength { .. } => CODE_PAYLOAD_MAX_LENGTH,
        ValidationErrorKind::MinLength { .. } => CODE_PAYLOAD_MIN_LENGTH,
        ValidationErrorKind::Type { .. } => CODE_PAYLOAD_TYPE,
        _ => "payload.other",
    }
}

fn scan_for_secrets(
    document: &serde_json::Value,
    registry: &ContractRegistry,
    result: &mut ValidationResult,
    current_path: &str,
) {
    let needles = registry.secret_substrings();
    match document {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let child_path = if current_path == "$" {
                    format!("$.{key}")
                } else {
                    format!("{current_path}.{key}")
                };
                let lowered = key.to_lowercase();
                if let Some(needle) = needles.iter().find(|n| lowered.contains(n.as_str())) {
                    result.add_error(
                        CODE_REDACTION_SECRET,
                        &child_path,
                        format!(
                            "Field {key:?} contains secret substring {needle:?}; remove the value or rename the field"
                        ),
                    );
                }
                scan_for_secrets(value, registry, result, &child_path);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, value) in items.iter().enumerate() {
                let child_path = format!("{current_path}[{index}]");
                scan_for_secrets(value, registry, result, &child_path);
            }
        }
        _ => {}
    }
}

fn scan_for_security_field_additions(
    payload: &serde_json::Value,
    entry: &crate::registry::ContractEntry,
    result: &mut ValidationResult,
    current_path: &str,
) {
    if entry.security_sensitive_fields.is_empty() {
        return;
    }
    let security = &entry.security_sensitive_fields;
    for path in iter_dotted_paths(payload, "") {
        if security.iter().any(|s| s == &path) {
            continue;
        }
        if is_under_security_namespace(&path, security) {
            result.add_warning(
                CODE_UNKNOWN_SECURITY_FIELD,
                &format!("{current_path}.{path}"),
                format!(
                    "Field {path:?} is not declared by the schema but lives under a security-sensitive namespace; document the compatibility fixture before relying on this field"
                ),
            );
        }
    }
}

fn is_under_security_namespace(path: &str, security_paths: &[String]) -> bool {
    if security_paths.is_empty() {
        return false;
    }
    let parts: Vec<&str> = path.split('.').collect();
    for i in (1..=parts.len()).rev() {
        let prefix = parts[..i].join(".");
        for sec in security_paths {
            if sec == &prefix || sec.starts_with(&format!("{prefix}.")) {
                return true;
            }
        }
    }
    let sec_segments: std::collections::HashSet<String> = security_paths
        .iter()
        .map(|p| p.split('.').next_back().unwrap_or(p).to_lowercase())
        .collect();
    let new_segments: std::collections::HashSet<String> =
        parts.iter().map(|p| p.to_lowercase()).collect();
    !sec_segments.is_disjoint(&new_segments)
}

fn iter_dotted_paths(value: &serde_json::Value, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let serde_json::Value::Object(map) = value {
        for (key, child) in map {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            if child.is_object() {
                out.extend(iter_dotted_paths(child, &path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

fn type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_to_dotted_converts() {
        assert_eq!(pointer_to_dotted(""), "");
        assert_eq!(pointer_to_dotted("/subject_id"), ".subject_id");
        assert_eq!(pointer_to_dotted("/checks/0/name"), ".checks[0].name");
    }
}
