//! Semantic review: fields.

use crate::core::ForgeError;

/// Normalize a CLI-supplied `--set field=value` argument into a
/// pair of strings. Returns `SemanticInvalid` for a malformed
/// pair or an unknown field.
pub fn parse_field_pair(input: &str) -> Result<(String, String), ForgeError> {
    let (field, value) = input
        .split_once('=')
        .ok_or_else(|| ForgeError::SemanticInvalid {
            reason: format!("malformed --set argument `{input}`: expected FIELD=VALUE"),
        })?;
    let field = field.trim().to_string();
    let value = value.trim().to_string();
    if field.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: format!("malformed --set argument `{input}`: field is empty"),
        });
    }
    if value.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: format!("malformed --set argument `{input}`: value is empty"),
        });
    }
    if !is_supported_field(&field) {
        return Err(ForgeError::SemanticInvalid {
            reason: format!(
                "unknown semantic field `{field}`; expected current_value, suggested_value, kind, confidence, or provider"
            ),
        });
    }
    Ok((field, value))
}

fn is_supported_field(field: &str) -> bool {
    matches!(
        field,
        "current_value"
            | "suggested_value"
            | "kind"
            | "confidence"
            | "provider"
            | "evidence_path"
            | "evidence_revision"
            | "evidence_excerpt"
            | "note"
    )
}
