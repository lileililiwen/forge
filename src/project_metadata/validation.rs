use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::core::ForgeError;

use super::invalid;

pub(super) fn validate_metadata_sources(
    declaration: &Value,
    manifest: Option<&Value>,
) -> Result<(), ForgeError> {
    let fail = |field: &str, expected: &str| {
        let field = if field.is_empty() {
            ".project.json".to_owned()
        } else {
            format!(".project.json.{field}")
        };
        invalid(format!("{field} must be {expected}"))
    };
    let validate_string =
        |object: &Map<String, Value>, field: &str, prefix: &str| -> Result<(), ForgeError> {
            if let Some(value) = object.get(field) {
                if !value.as_str().is_some_and(|value| !value.is_empty()) {
                    let field = if prefix.is_empty() {
                        field.to_owned()
                    } else {
                        format!("{prefix}.{field}")
                    };
                    return Err(fail(&field, "a non-empty string"));
                }
            }
            Ok(())
        };
    let validate_string_array =
        |object: &Map<String, Value>, field: &str| -> Result<(), ForgeError> {
            if let Some(value) = object.get(field) {
                let items = value
                    .as_array()
                    .ok_or_else(|| fail(&format!("discoverability.{field}"), "an array"))?;
                if items.is_empty()
                    || items
                        .iter()
                        .any(|item| !item.as_str().is_some_and(|item| !item.is_empty()))
                {
                    return Err(fail(
                        &format!("discoverability.{field}"),
                        "an array of non-empty strings",
                    ));
                }
            }
            Ok(())
        };

    let declaration = declaration
        .as_object()
        .ok_or_else(|| fail("", "an object"))?;
    for field in ["role", "stack", "profile", "lifecycle"] {
        validate_string(declaration, field, "")?;
    }
    if let Some(value) = declaration.get("discoverability") {
        let discoverability = value
            .as_object()
            .ok_or_else(|| fail("discoverability", "an object"))?;
        for field in ["description", "homepage"] {
            validate_string(discoverability, field, "discoverability")?;
        }
        for field in [
            "keywords",
            "topics",
            "audience",
            "use_cases",
            "search_phrases",
        ] {
            validate_string_array(discoverability, field)?;
        }
    }
    if let Some(value) = declaration.get("capabilities") {
        let capabilities = value
            .as_object()
            .ok_or_else(|| fail("capabilities", "an object"))?;
        for (id, value) in capabilities {
            let capability = value
                .as_object()
                .ok_or_else(|| fail(&format!("capabilities.{id}"), "an object"))?;
            for field in ["owner", "evidence_state"] {
                if !capability
                    .get(field)
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
                {
                    return Err(fail(
                        &format!("capabilities.{id}.{field}"),
                        "a non-empty string",
                    ));
                }
            }
            validate_string(capability, "evidence_ref", &format!("capabilities.{id}"))?;
        }
    }
    if let Some(value) = declaration.get("interfaces") {
        let interfaces = value
            .as_array()
            .ok_or_else(|| fail("interfaces", "an array"))?;
        let mut interface_ids = BTreeSet::new();
        for (index, value) in interfaces.iter().enumerate() {
            let field = format!("interfaces[{index}]");
            let interface = value.as_object().ok_or_else(|| fail(&field, "an object"))?;
            for required in ["id", "mechanism", "entry"] {
                if !interface
                    .get(required)
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
                {
                    return Err(fail(&format!("{field}.{required}"), "a non-empty string"));
                }
            }
            let id = interface.get("id").and_then(Value::as_str).unwrap();
            if !interface_ids.insert(id) {
                return Err(fail(&format!("{field}.id"), "unique within interfaces"));
            }
            for optional in ["contract", "provider", "digest", "evidence_ref"] {
                validate_string(interface, optional, &field)?;
            }
            for boolean in ["provides", "consumes", "required"] {
                if interface
                    .get(boolean)
                    .is_some_and(|value| !value.is_boolean())
                {
                    return Err(fail(&format!("{field}.{boolean}"), "a boolean"));
                }
            }
            let provides = interface.get("provides").and_then(Value::as_bool) == Some(true);
            let consumes = interface.get("consumes").and_then(Value::as_bool) == Some(true);
            if provides == consumes {
                return Err(fail(
                    &field,
                    "exactly one of provides or consumes to be true",
                ));
            }
        }
    }
    if let Some(value) = declaration.get("integrations") {
        let integrations = value
            .as_array()
            .ok_or_else(|| fail("integrations", "an array"))?;
        for (index, value) in integrations.iter().enumerate() {
            let field = format!("integrations[{index}]");
            let integration = value.as_object().ok_or_else(|| fail(&field, "an object"))?;
            if !integration
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty())
            {
                return Err(fail(&format!("{field}.id"), "a non-empty string"));
            }
            if !integration
                .get("required")
                .and_then(Value::as_bool)
                .is_some()
            {
                return Err(fail(&format!("{field}.required"), "a boolean"));
            }
        }
    }
    if let Some(manifest) = manifest {
        let manifest = manifest
            .as_object()
            .ok_or_else(|| invalid("forge.yaml must be an object"))?;
        if let Some(project) = manifest.get("project") {
            let project = project
                .as_object()
                .ok_or_else(|| invalid("forge.yaml project must be an object"))?;
            for field in ["name", "profile"] {
                if let Some(value) = project.get(field) {
                    if !value.as_str().is_some_and(|value| !value.is_empty()) {
                        return Err(invalid(format!(
                            "forge.yaml project.{field} must be a non-empty string"
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}
