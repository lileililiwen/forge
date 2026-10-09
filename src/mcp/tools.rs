//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::{McpToolDescriptor, McpToolKind};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
/// Built-in list of mature MCP tools. The list is the
/// authoritative answer to "what does Forge advertise over
/// MCP right now"; an operation that does not appear here
/// is not exposed (R1 + R2 boundary scenarios). New tools
/// must be added here with the matching dispatch arm and a
/// matching contract test.
pub fn tool_registry() -> Vec<McpToolDescriptor> {
    vec![
        McpToolDescriptor::read_only(
            "list_projects",
            "List catalog records from the local registry and the selected sources.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("sources", array_of_strings()),
                    ("tags", array_of_strings()),
                    ("languages", array_of_strings()),
                    ("profiles", array_of_strings()),
                    ("lifecycles", array_of_strings()),
                    ("repositories", array_of_strings()),
                    ("ci", array_of_strings()),
                    ("compose", array_of_strings()),
                    ("evidence", array_of_strings()),
                    ("filters", array_of_strings()),
                    ("limit", "integer"),
                    ("cursor", string_type()),
                    ("max_age", "integer"),
                ],
                &[],
            ),
        ),
        McpToolDescriptor::read_only(
            "inspect_project",
            "Inspect every catalog record for a project id across the selected sources.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("target", string_type()),
                    ("sources", array_of_strings()),
                    ("max_age", "integer"),
                ],
                &["target"],
            ),
        ),
        McpToolDescriptor::read_only(
            "list_profiles",
            "List versioned MVP profile descriptors.",
            schema_object(&[], &[]),
        ),
        McpToolDescriptor::read_only(
            "inspect_profile",
            "Inspect one versioned profile descriptor by id.",
            schema_object(&[("id", string_type())], &["id"]),
        ),
        McpToolDescriptor::read_only(
            "list_features",
            "List the versioned feature catalog.",
            schema_object(&[], &[]),
        ),
        McpToolDescriptor::read_only(
            "run_doctor",
            "Run a read-only doctor assessment on a project directory.",
            schema_object(
                &[
                    ("path", string_type()),
                    ("target", string_type()),
                    ("registry_path", string_type()),
                ],
                &["path"],
            ),
        ),
        McpToolDescriptor::read_only(
            "run_governance",
            "Run the selected standalone or optional governance provider for a project.",
            schema_object(&[("path", string_type())], &["path"]),
        ),
        McpToolDescriptor::mutating(
            "create_project",
            "Create a new project deterministically from a pinned profile.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("path", string_type()),
                    ("profile", string_type()),
                    ("id", string_type()),
                    ("name", string_type()),
                    ("features", array_of_strings()),
                    ("verify_native", boolean_type()),
                ],
                &["path", "profile", "id"],
            ),
        ),
        McpToolDescriptor::mutating(
            "import_project",
            "Import an existing project; on `--accept` write the manifest and register.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("path", string_type()),
                    ("profile", string_type()),
                    ("id", string_type()),
                    ("accept", boolean_type()),
                ],
                &["path"],
            ),
        ),
        McpToolDescriptor::mutating(
            "add_feature",
            "Install a feature (plus missing dependencies) on a project.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("target", string_type()),
                    ("feature", string_type()),
                    ("version", string_type()),
                ],
                &["target", "feature"],
            ),
        ),
        McpToolDescriptor::mutating(
            "remove_feature",
            "Remove a feature from a project, blocking on reverse-dependents.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("target", string_type()),
                    ("feature", string_type()),
                ],
                &["target", "feature"],
            ),
        ),
        McpToolDescriptor::mutating(
            "upgrade_feature",
            "Upgrade a feature to the tested catalog version on a project.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("target", string_type()),
                    ("feature", string_type()),
                    ("version", string_type()),
                ],
                &["target", "feature"],
            ),
        ),
        McpToolDescriptor::mutating(
            "generate_spec",
            "Generate a bounded spec for the named finding set on a project.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("path", string_type()),
                    ("findings", array_of_strings()),
                    ("reason", string_type()),
                ],
                &["path", "findings"],
            ),
        ),
        McpToolDescriptor::mutating(
            "run_agent",
            "Record a managed agent session transition on a project.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("path", string_type()),
                    ("session", string_type()),
                    ("provider", string_type()),
                    ("spec", string_type()),
                    ("transition", string_type()),
                ],
                &["path", "session", "transition"],
            ),
        ),
        McpToolDescriptor::mutating(
            "run_tests",
            "Run the profile's native test command on the named project.",
            schema_object(
                &[("registry_path", string_type()), ("path", string_type())],
                &["path"],
            ),
        ),
        McpToolDescriptor::mutating(
            "commit",
            "Stage the listed paths and create one scoped commit.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("path", string_type()),
                    ("paths", array_of_strings()),
                    ("message", string_type()),
                ],
                &["path", "paths", "message"],
            ),
        ),
        McpToolDescriptor::external_write(
            "push",
            "Push the named ref to a remote; requires `confirm: true`.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("path", string_type()),
                    ("remote", string_type()),
                    ("ref_name", string_type()),
                    ("confirm", boolean_type()),
                ],
                &["path", "confirm"],
            ),
        ),
        McpToolDescriptor::external_write(
            "mirror_project",
            "Distribute the project's refs to the configured primary and one-way mirrors; requires `confirm: true`.",
            schema_object(
                &[
                    ("registry_path", string_type()),
                    ("path", string_type()),
                    ("refs", array_of_strings()),
                    ("confirm", boolean_type()),
                    ("retry_failed", boolean_type()),
                    ("dry_run", boolean_type()),
                ],
                &["path", "confirm"],
            ),
        ),
    ]
}

pub(super) fn schema_object(properties: &[(&str, &str)], required: &[&str]) -> Value {
    let mut props = Map::new();
    for (name, kind) in properties {
        props.insert(
            (*name).to_string(),
            serde_json::json!({"type": json_type_label(kind)}),
        );
    }
    serde_json::json!({
        "type": "object",
        "properties": Value::Object(props),
        "required": required,
    })
}

pub(super) fn string_type() -> &'static str {
    "string"
}

pub(super) fn boolean_type() -> &'static str {
    "boolean"
}

pub(super) fn array_of_strings() -> &'static str {
    "string[]"
}

/// Look up a tool descriptor by name. The surface advertises
/// only the tools returned by [`tool_registry`]; anything
/// else is rejected as a missing tool.
pub fn find_tool(name: &str) -> Option<McpToolDescriptor> {
    tool_registry().into_iter().find(|t| t.name == name)
}

/// Summarize the registry so a model can ask "is this
/// operation mature?" before issuing a request. The
/// summary is the same JSON shape the `tools/list` method
/// returns, but exposes a separate name for clarity in
/// tests.
pub fn registry_summary() -> BTreeMap<String, McpToolKind> {
    let mut out = BTreeMap::new();
    for t in tool_registry() {
        out.insert(t.name, t.kind);
    }
    out
}

fn json_type_label(kind: &str) -> &str {
    match kind {
        "string[]" => "array",
        _ => kind,
    }
}
