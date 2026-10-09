//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::{rpc_code, McpRpcError, McpToolDescriptor};
use crate::core::ForgeError;
use crate::doctor::RegistryObservation;
use crate::policy::{redact_report_in_place, PolicyOutcome};
use crate::registry::Registry;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
pub(super) fn validate_params(
    descriptor: &McpToolDescriptor,
    params: &Value,
) -> Result<Map<String, Value>, McpRpcError> {
    if params.is_null() {
        if schema_requires_any(&descriptor.input_schema) {
            return Err(McpRpcError::new(
                rpc_code::INVALID_PARAMS,
                format!("tool `{}` requires params", descriptor.name),
            ));
        }
        return Ok(Map::new());
    }
    let object = params.as_object().ok_or_else(|| {
        McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            format!("params for `{}` must be a JSON object", descriptor.name),
        )
    })?;
    let required = schema_required_fields(&descriptor.input_schema);
    for field in &required {
        if !object.contains_key(field) {
            return Err(McpRpcError::new(
                rpc_code::INVALID_PARAMS,
                format!("tool `{}` requires field `{field}`", descriptor.name),
            ));
        }
    }
    for (field, value) in object.iter() {
        let expected = schema_field_type(&descriptor.input_schema, field);
        if !value_matches(value, expected) {
            return Err(McpRpcError::new(
                rpc_code::INVALID_PARAMS,
                format!(
                    "field `{field}` of `{}` has the wrong type; expected {expected}",
                    descriptor.name
                ),
            ));
        }
    }
    Ok(object.clone())
}

fn value_matches(value: &Value, expected: &str) -> bool {
    match expected {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "string[]" => value
            .as_array()
            .is_some_and(|arr| arr.iter().all(|v| v.is_string())),
        "any" | "unknown" => true,
        _ => true,
    }
}

fn schema_field_type<'a>(schema: &'a Value, field: &str) -> &'a str {
    let props = schema.get("properties").and_then(|v| v.as_object());
    if let Some(props) = props {
        if let Some(field_schema) = props.get(field) {
            return field_schema
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
        }
    }
    "unknown"
}

fn schema_required_fields(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn schema_requires_any(schema: &Value) -> bool {
    !schema_required_fields(schema).is_empty()
}

pub(super) fn required_string(
    args: &Map<String, Value>,
    field: &str,
) -> Result<String, McpRpcError> {
    let value = args.get(field).ok_or_else(|| {
        McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            format!("missing required field `{field}`"),
        )
    })?;
    value.as_str().map(|s| s.to_string()).ok_or_else(|| {
        McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            format!("field `{field}` must be a string"),
        )
    })
}

pub(super) fn optional_string(args: &Map<String, Value>, field: &str) -> Option<String> {
    args.get(field)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

pub(super) fn optional_bool(args: &Map<String, Value>, field: &str) -> Option<bool> {
    args.get(field).and_then(|v| v.as_bool())
}

pub(super) fn string_array(
    args: &Map<String, Value>,
    field: &str,
) -> Result<Vec<String>, McpRpcError> {
    let value = args.get(field).ok_or_else(|| {
        McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            format!("missing required field `{field}`"),
        )
    })?;
    string_array_value(field, value)
}

pub(super) fn string_array_value(field: &str, value: &Value) -> Result<Vec<String>, McpRpcError> {
    let arr = value.as_array().ok_or_else(|| {
        McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            format!("field `{field}` must be an array"),
        )
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let s = v.as_str().ok_or_else(|| {
            McpRpcError::new(
                rpc_code::INVALID_PARAMS,
                format!("field `{field}` must be an array of strings"),
            )
        })?;
        out.push(s.to_string());
    }
    Ok(out)
}

pub(super) fn validate_project_id_strict(id: &str) -> Result<(), McpRpcError> {
    crate::core::validate_project_id(id).map_err(|reason| {
        McpRpcError::new(
            rpc_code::TOOL_REFUSED,
            format!(
                "invalid project id `{id}`: {reason}; treated as literal data, never executed as shell code"
            ),
        )
        .with_data(serde_json::json!({
            "code": "project-id-invalid",
            "message": format!("invalid project id: {reason}; treated as literal data"),
        }))
    })
}

pub(super) fn canonicalize_project(path: &str) -> Result<PathBuf, McpRpcError> {
    let candidate = Path::new(path);
    if !candidate.is_dir() {
        return Err(McpRpcError::new(
            rpc_code::TOOL_REFUSED,
            format!("project path `{path}` is not a directory"),
        ));
    }
    candidate.canonicalize().map_err(|_| {
        McpRpcError::new(
            rpc_code::TOOL_REFUSED,
            format!("project path `{path}` could not be canonicalized"),
        )
    })
}

pub(super) fn open_registry(db_path: &Path) -> Result<Registry, McpRpcError> {
    Registry::open(db_path).map_err(core_error)
}

pub(super) fn observation_for(registry: &Registry, path: &Path) -> Option<RegistryObservation> {
    let canonical = path.canonicalize().ok()?.display().to_string();
    let record = registry
        .list()
        .ok()?
        .into_iter()
        .find(|p| p.path == canonical)?;
    Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at),
    })
}

pub(super) fn core_error(err: ForgeError) -> McpRpcError {
    let data = serde_json::json!({
        "code": err.code(),
        "message": err.to_string(),
    });
    let code = match err.code() {
        "unknown-project" | "unknown-profile" | "unknown-feature" => rpc_code::INVALID_PARAMS,
        "path-unavailable" | "manifest-not-found" | "manifest-invalid" | "spec-invalid" => {
            rpc_code::INVALID_PARAMS
        }
        "catalog-invalid" => rpc_code::INVALID_PARAMS,
        "push-confirm-required" => rpc_code::TOOL_REFUSED,
        other if other.starts_with("agent-") => rpc_code::TOOL_REFUSED,
        _ => rpc_code::INTERNAL_ERROR,
    };
    McpRpcError::new(code, err.to_string()).with_data(data)
}

pub(super) fn internal_error(message: String) -> McpRpcError {
    McpRpcError::new(rpc_code::INTERNAL_ERROR, message)
}

pub(super) fn redact_policy(outcome: &PolicyOutcome) -> Value {
    match outcome {
        PolicyOutcome::Reported(report) => {
            let mut sanitized = report.clone();
            redact_report_in_place(&mut sanitized);
            serde_json::to_value(&sanitized).unwrap_or_else(|_| serde_json::json!({}))
        }
        PolicyOutcome::Unavailable { reason } => {
            serde_json::json!({"unavailable": reason})
        }
    }
}
