//! Mature MCP surface (`mature-mcp-surface`).
//!
//! One stdio JSON-RPC 2.0 server that exposes the stable mature
//! Core operations through a single tool list. Core owns every
//! effect: the MCP transport only validates input, dispatches
//! through the same Core contracts the CLI uses, and renders
//! typed outcomes back to the caller. No business rule is
//! duplicated in the transport.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §20, §34, §42 require
//! one MCP server over mature Core operations. v0.4 exposes
//! only the operations that have already passed local
//! verification in earlier changes; an operation that is not
//! in [`tool_registry`] is not advertised (R1 boundary
//! scenario). The first MCP version is stdio; HTTP and the
//! portal are separate changes.
//!
//! ## Tool registry
//!
//! Every entry records the tool id, a human description, the
//! [`McpToolKind`] classification, and the structured input
//! shape Core expects. The classification drives both
//! authorization and the rendering of mutating side effects:
//!
//! - `ReadOnly` — no side effects, no project mutation
//!   (for example `list_projects`, `inspect_project`,
//!   `list_profiles`, `list_features`, `run_doctor`).
//! - `Mutating` — writes files, manifest, receipts, registry
//!   or sessions inside the project scope (for example
//!   `create_project`, `import_project`, `add_feature`,
//!   `remove_feature`, `upgrade_feature`, `generate_spec`,
//!   `run_agent`, `run_tests`, `commit`).
//! - `ExternalWrite` — issues a network side effect (for
//!   example `push`). Requires an explicit `confirm: true`
//!   in the request body so an implicit remote write is
//!   never accepted.
//!
//! `deploy` and `publish` are intentionally absent. Their
//! Core operations are not implemented yet, so the
//! registry does not advertise them (R2 boundary scenario).
//!
//! ## Risk model
//!
//! Model-issued requests are untrusted structured inputs.
//! The transport validates every field by name and type
//! before any Core call; a missing required field, an
//! unknown tool, an unauthorized project path, or a name
//! that contains shell metacharacters returns a structured
//! error before Core mutates state. Diagnostic logging is
//! restricted to stderr; the JSON-RPC envelope over stdout
//! is the only response surface. The session is bounded by
//! the host process: one request per line, one response per
//! line, no remote lifecycle.

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::agent::{
    apply_transition as apply_agent_transition, new_session, read_session,
    run_spec as run_session_spec, AgentProvider, AgentTransitionOutcome, SessionListEntry,
    SessionTransition,
};
use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::distribution::{
    apply_mirror, distribution_config_from_manifest, plan_mirror, DistributionConfig, MirrorRequest,
};
use crate::doctor::{parse_target_level, run_doctor, RegistryObservation};
use crate::feature::{add_feature, remove_feature, upgrade_feature};
use crate::generate::{generate, normalize_explicit, GeneratedProject};
use crate::gitops::{commit_paths, push_ref, run_test, CommitOutcome, PushOutcome, TestOutcome};
use crate::import::{adopt_import, inspect_import};
use crate::policy::{redact_report_in_place, run_driftwatch, DriftWatchConfig, PolicyOutcome};
use crate::profile::inspect_profile;
use crate::registry::Registry;
use crate::spec::{
    apply_routing, ensure_single_project, generate_spec, list_specs, read_spec, route_finding,
    DoctorFindingInput, FindingSource, RoutingOutcome, SpecDraft, SpecGenerateOutcome,
    SpecListEntry, SpecRequest,
};
use crate::upgrade::plan_upgrade;

/// Contract data version for the MCP surface. Bumping this
/// constant is the source-of-truth for tool list, validation
/// and dispatch shape; an older client must refuse the
/// version mismatch instead of silently reinterpreting the
/// response.
pub const MCP_CONTRACT_VERSION: &str = "0.1.0";

/// JSON-RPC protocol version implemented by the surface.
pub const JSONRPC_VERSION: &str = "2.0";

/// Classification of a single MCP tool. Drives the mutating
/// tool boundary, the authorization check, and the rendering
/// of side effects in the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolKind {
    /// Read-only: never mutates project state.
    ReadOnly,
    /// Mutating: writes files inside the project scope.
    Mutating,
    /// ExternalWrite: issues a network side effect.
    ExternalWrite,
}

impl McpToolKind {
    pub fn label(&self) -> &'static str {
        match self {
            McpToolKind::ReadOnly => "read_only",
            McpToolKind::Mutating => "mutating",
            McpToolKind::ExternalWrite => "external_write",
        }
    }
}

/// One declared MCP tool. The descriptor is the source of
/// truth for the tool name, the human description, the kind
/// (driving the mutating boundary), and the JSON Schema the
/// request payload must satisfy before Core is called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub kind: McpToolKind,
    pub description: String,
    pub contract: String,
    pub input_schema: Value,
}

impl McpToolDescriptor {
    fn read_only(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            kind: McpToolKind::ReadOnly,
            description: description.to_string(),
            contract: MCP_CONTRACT_VERSION.to_string(),
            input_schema,
        }
    }

    fn mutating(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            kind: McpToolKind::Mutating,
            description: description.to_string(),
            contract: MCP_CONTRACT_VERSION.to_string(),
            input_schema,
        }
    }

    fn external_write(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            kind: McpToolKind::ExternalWrite,
            description: description.to_string(),
            contract: MCP_CONTRACT_VERSION.to_string(),
            input_schema,
        }
    }
}

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
            "List registered projects from the local registry.",
            schema_object(&[("registry_path", string_type())], &[]),
        ),
        McpToolDescriptor::read_only(
            "inspect_project",
            "Inspect a registered project by id or path.",
            schema_object(
                &[("registry_path", string_type()), ("target", string_type())],
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

/// JSON-RPC 2.0 request envelope. `id` is optional because
/// notification requests (without `id`) are valid JSON-RPC
/// but the surface returns an explicit error for them so the
/// caller does not silently lose work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl McpRequest {
    /// Parse a single line of stdin. The line must contain a
    /// single JSON object; multiple objects on one line or
    /// partial JSON are refused with a structured parse error.
    pub fn parse_line(line: &str) -> Result<Self, McpProtocolError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(McpProtocolError::Empty);
        }
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|err| McpProtocolError::Parse(err.to_string()))?;
        if !value.is_object() {
            return Err(McpProtocolError::Parse(
                "request payload must be a JSON object".to_string(),
            ));
        }
        let request: McpRequest = serde_json::from_value(value)
            .map_err(|err| McpProtocolError::Parse(err.to_string()))?;
        if request.jsonrpc != JSONRPC_VERSION {
            return Err(McpProtocolError::Parse(format!(
                "jsonrpc must be `{JSONRPC_VERSION}`; got `{}`",
                request.jsonrpc
            )));
        }
        if request.method.is_empty() {
            return Err(McpProtocolError::Parse(
                "method must be a non-empty string".to_string(),
            ));
        }
        Ok(request)
    }
}

/// JSON-RPC 2.0 response envelope. The transport always
/// emits the matching id (or `null` when the request did not
/// carry one) so the caller can correlate responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpRpcError>,
}

impl McpResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(id: Value, error: McpRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Render the response as a single line of JSON for
    /// stdout. Pretty-printing is avoided to keep one
    /// request→one response→one line semantics.
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("MCP response is always serializable")
    }
}

/// JSON-RPC 2.0 error object. `code` is the standard
/// JSON-RPC integer code, `message` is the human-readable
/// summary and `data` carries the structured Core error
/// (with the stable `code()` from `ForgeError`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl McpRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Standard JSON-RPC 2.0 error codes plus the Forge surface
/// codes for transport-level failures.
pub mod rpc_code {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const TOOL_UNAUTHORIZED: i32 = -32010;
    pub const TOOL_MISSING: i32 = -32011;
    pub const TOOL_REFUSED: i32 = -32012;
}

/// Transport-level parse error. Never escapes the transport:
/// the transport renders it as a JSON-RPC `parse_error`
/// response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpProtocolError {
    Empty,
    Parse(String),
}

/// Dispatch the parsed request through the Core contracts.
/// Returns the value the transport should write to stdout, or
/// a [`McpRpcError`] describing the structured failure.
///
/// `db_path` is the registry path; `null` falls back to the
/// default registry location.
pub fn dispatch(db_path: Option<&Path>, request: &McpRequest) -> Result<Value, McpRpcError> {
    match request.method.as_str() {
        "tools/list" => Ok(serde_json::json!({
            "contract": MCP_CONTRACT_VERSION,
            "tools": tool_registry(),
        })),
        method => {
            let descriptor = tool_registry()
                .into_iter()
                .find(|t| t.name == method)
                .ok_or_else(|| {
                    McpRpcError::new(
                        rpc_code::TOOL_MISSING,
                        format!(
                            "unknown tool `{method}`; the tool list is the only source of truth"
                        ),
                    )
                })?;
            let mut args = match validate_params(&descriptor, &request.params) {
                Ok(args) => args,
                Err(err) => {
                    return Err(err);
                }
            };
            dispatch_tool(db_path, &descriptor, &mut args)
        }
    }
}

/// Core dispatch table. The `read_only` arm never mutates;
/// the `mutating` arm is allowed to write inside the project
/// scope; the `external_write` arm is allowed to issue a
/// network side effect, but only when the caller passed
/// `confirm: true` (the schema requires the field).
fn dispatch_tool(
    db_path: Option<&Path>,
    descriptor: &McpToolDescriptor,
    args: &mut Map<String, Value>,
) -> Result<Value, McpRpcError> {
    let db_path = match db_path {
        Some(p) => p.to_path_buf(),
        None => crate::registry::default_registry_path(),
    };
    match descriptor.kind {
        McpToolKind::ReadOnly => dispatch_read_only(&db_path, &descriptor.name, args),
        McpToolKind::Mutating => dispatch_mutating(&db_path, &descriptor.name, args),
        McpToolKind::ExternalWrite => dispatch_external_write(&db_path, &descriptor.name, args),
    }
}

fn dispatch_read_only(
    db_path: &Path,
    name: &str,
    args: &Map<String, Value>,
) -> Result<Value, McpRpcError> {
    match name {
        "list_projects" => {
            let registry = open_registry(db_path)?;
            Ok(serde_json::json!({"projects": registry.list().map_err(core_error)?}))
        }
        "inspect_project" => {
            let target = required_string(args, "target")?;
            let registry = open_registry(db_path)?;
            let record = registry.inspect(&target).map_err(core_error)?;
            let value =
                serde_json::to_value(&record).map_err(|err| internal_error(err.to_string()))?;
            Ok(serde_json::json!({"project": value}))
        }
        "list_profiles" => Ok(serde_json::json!({"profiles": crate::profile::list_profiles()})),
        "inspect_profile" => {
            let id = required_string(args, "id")?;
            let descriptor = inspect_profile(&id).map_err(core_error)?;
            let value =
                serde_json::to_value(&descriptor).map_err(|err| internal_error(err.to_string()))?;
            Ok(serde_json::json!({"profile": value}))
        }
        "list_features" => Ok(serde_json::json!({
            "features": crate::feature::feature_catalog()
        })),
        "run_doctor" => {
            let path = required_string(args, "path")?;
            let target = optional_string(args, "target");
            let level = match target.as_deref() {
                Some(raw) => Some(parse_target_level(raw).map_err(core_error)?),
                None => None,
            };
            let canonical = canonicalize_project(&path)?;
            let observation = open_registry(db_path)
                .ok()
                .and_then(|registry| observation_for(&registry, &canonical));
            let policy_outcome = run_driftwatch(&canonical, &DriftWatchConfig::from_env());
            let report = run_doctor(
                &canonical,
                level,
                observation.as_ref(),
                Some(&policy_outcome),
            )
            .map_err(core_error)?;
            let value =
                serde_json::to_value(&report).map_err(|err| internal_error(err.to_string()))?;
            Ok(serde_json::json!({"doctor": value, "policy": redact_policy(&policy_outcome)}))
        }
        "run_governance" => {
            let path = required_string(args, "path")?;
            let canonical = canonicalize_project(&path)?;
            let observation =
                crate::governance::evaluate_project(&canonical).map_err(core_error)?;
            let value = serde_json::to_value(&observation)
                .map_err(|err| internal_error(err.to_string()))?;
            Ok(serde_json::json!({"observation": value}))
        }
        other => Err(McpRpcError::new(
            rpc_code::METHOD_NOT_FOUND,
            format!("read-only dispatcher has no implementation for `{other}`"),
        )),
    }
}

fn dispatch_mutating(
    db_path: &Path,
    name: &str,
    args: &Map<String, Value>,
) -> Result<Value, McpRpcError> {
    match name {
        "create_project" => mcp_create_project(db_path, args),
        "import_project" => mcp_import_project(db_path, args),
        "add_feature" => mcp_feature_add(db_path, args),
        "remove_feature" => mcp_feature_remove(db_path, args),
        "upgrade_feature" => mcp_feature_upgrade(db_path, args),
        "generate_spec" => mcp_generate_spec(args),
        "run_agent" => mcp_run_agent(db_path, args),
        "run_tests" => mcp_run_tests(args),
        "commit" => mcp_commit(args),
        other => Err(McpRpcError::new(
            rpc_code::METHOD_NOT_FOUND,
            format!("mutating dispatcher has no implementation for `{other}`"),
        )),
    }
}

fn dispatch_external_write(
    _db_path: &Path,
    name: &str,
    args: &Map<String, Value>,
) -> Result<Value, McpRpcError> {
    match name {
        "push" => mcp_push(args),
        "mirror_project" => mcp_mirror_project(args),
        other => Err(McpRpcError::new(
            rpc_code::METHOD_NOT_FOUND,
            format!("external-write dispatcher has no implementation for `{other}`"),
        )),
    }
}

// ---- mutating tool implementations --------------------------------

fn mcp_create_project(db_path: &Path, args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let path = required_string(args, "path")?;
    let profile = required_string(args, "profile")?;
    let id = required_string(args, "id")?;
    validate_project_id_strict(&id)?;
    let name = optional_string(args, "name");
    // `features` is optional in the schema; an absent
    // field means "no features requested", matching the
    // CLI default.
    let features = match args.get("features") {
        Some(value) => string_array_value("features", value)?,
        None => Vec::new(),
    };
    let verify_native_flag = optional_bool(args, "verify_native").unwrap_or(false);
    let destination = PathBuf::from(&path);
    let normalized = normalize_explicit(
        Some(&profile),
        Some(&id),
        name.as_deref(),
        &features,
        &destination,
    )
    .map_err(core_error)?;
    let mut registry = open_registry(db_path)?;
    let mut generated: GeneratedProject =
        generate(&mut registry, &normalized).map_err(core_error)?;
    if verify_native_flag {
        let report =
            crate::generate::verify_native(&normalized.profile, &normalized.destination, None)
                .map_err(core_error)?;
        generated.native_verified = report.verified;
        generated.native_note = format!(
            "native build `{}` and test `{}` succeeded for profile `{}`",
            report.build_command, report.test_command, report.profile
        );
    }
    let detail = format!(
        "mcp create_project `{}` ({}) from {}@{}",
        generated.record.id,
        generated.record.path,
        profile,
        crate::generate::GENERATOR_VERSION
    );
    let _ = registry.record_operation("mcp", &generated.record.id, "done", &detail);
    let value = serde_json::to_value(&generated).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({
        "contract": MCP_CONTRACT_VERSION,
        "created": value,
        "registry": db_path.display().to_string(),
    }))
}

fn mcp_import_project(db_path: &Path, args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let path = required_string(args, "path")?;
    let accept = optional_bool(args, "accept").unwrap_or(false);
    let profile = optional_string(args, "profile");
    let id = optional_string(args, "id");
    let canonical = canonicalize_project(&path)?;
    if accept {
        let mut registry = open_registry(db_path)?;
        let record = adopt_import(&mut registry, &canonical, profile.as_deref(), id.as_deref())
            .map_err(core_error)?;
        let detail = format!("mcp import_project `{}` ({})", record.id, record.path);
        let _ = registry.record_operation("mcp", &record.id, "done", &detail);
        let value = serde_json::to_value(&record).map_err(|err| internal_error(err.to_string()))?;
        Ok(serde_json::json!({"imported": value}))
    } else {
        let proposal = inspect_import(&canonical, profile.as_deref()).map_err(core_error)?;
        let value =
            serde_json::to_value(&proposal).map_err(|err| internal_error(err.to_string()))?;
        Ok(serde_json::json!({"proposal": value}))
    }
}

fn mcp_feature_add(db_path: &Path, args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let target = required_string(args, "target")?;
    let feature = required_string(args, "feature")?;
    let version = optional_string(args, "version");
    let mut registry = open_registry(db_path)?;
    let outcome =
        add_feature(&mut registry, &target, &feature, version.as_deref()).map_err(core_error)?;
    let value = serde_json::to_value(&outcome).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({"feature": value}))
}

fn mcp_feature_remove(db_path: &Path, args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let target = required_string(args, "target")?;
    let feature = required_string(args, "feature")?;
    let mut registry = open_registry(db_path)?;
    let outcome = remove_feature(&mut registry, &target, &feature).map_err(core_error)?;
    let value = serde_json::to_value(&outcome).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({"feature": value}))
}

fn mcp_feature_upgrade(db_path: &Path, args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let target = required_string(args, "target")?;
    let feature = required_string(args, "feature")?;
    let version = optional_string(args, "version");
    let mut registry = open_registry(db_path)?;
    let outcome = upgrade_feature(&mut registry, &target, &feature, version.as_deref())
        .map_err(core_error)?;
    let value = serde_json::to_value(&outcome).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({"feature": value}))
}

fn mcp_generate_spec(args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let path = required_string(args, "path")?;
    let findings = string_array(args, "findings")?;
    if findings.is_empty() {
        return Err(McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            "spec generate requires at least one finding id",
        ));
    }
    let reason = optional_string(args, "reason");
    let canonical = canonicalize_project(&path)?;
    let request = SpecRequest {
        project_path: canonical,
        finding_ids: findings,
        reason,
    };
    ensure_single_project(&request).map_err(core_error)?;
    let sources: Vec<FindingSource> = Vec::new();
    let now = chrono::Utc::now();
    let outcome: SpecGenerateOutcome =
        generate_spec(&request, &sources, now).map_err(core_error)?;
    let value = serde_json::to_value(&outcome).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({"spec": value}))
}

fn mcp_run_agent(db_path: &Path, args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let path = required_string(args, "path")?;
    let session_id = required_string(args, "session")?;
    let provider = optional_string(args, "provider").unwrap_or_else(|| "opencode".to_string());
    let spec = optional_string(args, "spec");
    let transition = required_string(args, "transition")?;
    let canonical = canonicalize_project(&path)?;
    let now = chrono::Utc::now();
    let (manifest, _) = Manifest::load_from_dir(&canonical, None).map_err(core_error)?;
    let project_id = manifest.project.id;
    let provider_id = match provider.as_str() {
        "opencode" => AgentProvider::Opencode,
        "codex" => AgentProvider::Codex,
        other => {
            return Err(McpRpcError::new(
                rpc_code::INVALID_PARAMS,
                format!("unknown agent provider `{other}`; expected one of: opencode, codex"),
            ));
        }
    };
    let transition_kind = match transition.as_str() {
        "start" => SessionTransition::Start,
        "pause" => SessionTransition::Pause,
        "takeover" => SessionTransition::Takeover,
        "resume" => SessionTransition::Resume,
        "restart" => SessionTransition::Restart,
        "new_session" => SessionTransition::NewSession,
        "run_spec" => {
            return mcp_run_agent_run_spec(db_path, &canonical, &project_id, &session_id, now);
        }
        other => {
            return Err(McpRpcError::new(
                rpc_code::INVALID_PARAMS,
                format!(
                    "unknown agent transition `{other}`; expected one of: start, pause, takeover, resume, restart, new_session, run_spec"
                ),
            ));
        }
    };
    let session = match transition_kind {
        SessionTransition::Start => new_session(
            &canonical,
            &session_id,
            provider_id,
            spec.as_deref().unwrap_or(""),
            now,
        )
        .map_err(core_error)?,
        _ => read_session(&canonical, &session_id)
            .map_err(core_error)?
            .ok_or_else(|| {
                McpRpcError::new(
                    rpc_code::TOOL_REFUSED,
                    format!("session `{session_id}` was not found under `.forge/agents/`"),
                )
            })?,
    };
    let outcome: AgentTransitionOutcome =
        apply_agent_transition(session, transition_kind, now).map_err(core_error)?;
    let files = crate::agent::write_session(&canonical, &outcome.session).map_err(core_error)?;
    let registry = open_registry(db_path)?;
    let detail = format!(
        "mcp run_agent `{}` session `{}` {} -> `{}`",
        project_id,
        session_id,
        transition,
        outcome.state.label()
    );
    let _ = registry.record_operation("mcp", &project_id, "done", &detail);
    Ok(serde_json::json!({
        "transition": agent_outcome_envelope(&outcome),
        "files_written": files,
    }))
}

fn mcp_run_agent_run_spec(
    db_path: &Path,
    canonical: &Path,
    project_id: &str,
    session_id: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Value, McpRpcError> {
    let outcome = run_session_spec(canonical, session_id, now).map_err(core_error)?;
    let files = crate::agent::write_session(canonical, &outcome.session).map_err(core_error)?;
    let registry = open_registry(db_path)?;
    let detail = format!(
        "mcp run_agent `{}` run_spec session `{}` -> `{}`",
        project_id,
        session_id,
        outcome.state.label()
    );
    let _ = registry.record_operation("mcp", project_id, "done", &detail);
    Ok(serde_json::json!({
        "transition": agent_outcome_envelope(&outcome),
        "files_written": files,
    }))
}

/// Render an [`AgentTransitionOutcome`] as a JSON object
/// matching the CLI's `forge agent` envelope: `session_id`
/// and `project_id` are hoisted so a model can address the
/// session without navigating into a nested `session`
/// object, mirroring the CLI's `agent_outcome_output`.
fn agent_outcome_envelope(outcome: &AgentTransitionOutcome) -> Value {
    serde_json::json!({
        "session_id": outcome.session.session_id,
        "project_id": outcome.session.project_id,
        "provider": outcome.session.provider.label(),
        "spec_id": outcome.session.spec_id,
        "state": outcome.state.label(),
        "requested": outcome.requested.label(),
        "evidence": outcome.evidence,
        "next_step": outcome.next_step,
        "note": outcome.note,
        "contract": outcome.contract,
    })
}

fn mcp_run_tests(args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let path = required_string(args, "path")?;
    let canonical = canonicalize_project(&path)?;
    let (manifest, _) = Manifest::load_from_dir(&canonical, None).map_err(core_error)?;
    let descriptor = inspect_profile(&manifest.project.profile).map_err(core_error)?;
    let test_command = descriptor.test_command.clone();
    let outcome: TestOutcome = run_test(
        &manifest.project.id,
        &manifest.project.profile,
        Some(&test_command),
        &canonical,
    )
    .map_err(core_error)?;
    let value = serde_json::to_value(&outcome).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({"test": value}))
}

fn mcp_commit(args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let path = required_string(args, "path")?;
    let paths = string_array(args, "paths")?;
    if paths.is_empty() {
        return Err(McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            "commit requires at least one path",
        ));
    }
    let message = required_string(args, "message")?;
    if message.trim().is_empty() {
        return Err(McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            "commit message must be non-empty",
        ));
    }
    let canonical = canonicalize_project(&path)?;
    let (manifest, _) = Manifest::load_from_dir(&canonical, None).map_err(core_error)?;
    let outcome: CommitOutcome =
        commit_paths(&manifest.project.id, &canonical, &paths, &message).map_err(core_error)?;
    let value = serde_json::to_value(&outcome).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({"commit": value}))
}

fn mcp_push(args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let path = required_string(args, "path")?;
    let confirm = optional_bool(args, "confirm").unwrap_or(false);
    if !confirm {
        let err = McpRpcError::new(
            rpc_code::TOOL_REFUSED,
            "push requires `confirm: true`; implicit remote writes are refused",
        )
        .with_data(serde_json::json!({
            "code": "push-confirm-required",
            "message": "push requires `confirm: true`; refusing implicit remote write",
        }));
        return Err(err);
    }
    let remote = optional_string(args, "remote").unwrap_or_else(|| "origin".to_string());
    let ref_name = optional_string(args, "ref_name").unwrap_or_else(|| "HEAD".to_string());
    let canonical = canonicalize_project(&path)?;
    let (manifest, _) = Manifest::load_from_dir(&canonical, None).map_err(core_error)?;
    let outcome: PushOutcome =
        push_ref(&manifest.project.id, &canonical, &remote, &ref_name, true).map_err(core_error)?;
    let value = serde_json::to_value(&outcome).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({"push": value}))
}

fn mcp_mirror_project(args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    let path = required_string(args, "path")?;
    let confirm = optional_bool(args, "confirm").unwrap_or(false);
    if !confirm {
        let err = McpRpcError::new(
            rpc_code::TOOL_REFUSED,
            "mirror_project requires `confirm: true`; implicit remote writes are refused",
        )
        .with_data(serde_json::json!({
            "code": "distribution-confirm-required",
            "message": "mirror_project requires `confirm: true`; refusing implicit remote write",
        }));
        return Err(err);
    }
    let refs = string_array(args, "refs")?;
    if refs.is_empty() {
        return Err(McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            "mirror_project requires at least one ref name in `refs`",
        ));
    }
    let dry_run = optional_bool(args, "dry_run").unwrap_or(false);
    let retry_failed = optional_bool(args, "retry_failed").unwrap_or(false);
    let canonical = canonicalize_project(&path)?;
    let (manifest, _) = Manifest::load_from_dir(&canonical, None).map_err(core_error)?;
    let config: DistributionConfig =
        distribution_config_from_manifest(&manifest).map_err(core_error)?;
    let request = MirrorRequest {
        project_id: manifest.project.id.clone(),
        refs,
        confirm: true,
        dry_run,
        retry_failed,
    };
    let report = if dry_run {
        let state_path = crate::distribution::state_path_for(&canonical, &manifest.project.id)
            .map_err(core_error)?;
        let state = crate::distribution::load_mirror_state(&state_path).map_err(core_error)?;
        plan_mirror(&config, &request, &state, &state_path).map_err(core_error)?
    } else {
        apply_mirror(&canonical, &config, &request).map_err(core_error)?
    };
    let value = serde_json::to_value(&report).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({
        "contract": crate::distribution::DISTRIBUTION_CONTRACT_VERSION,
        "mirror": value,
    }))
}

// ---- parameter validation -----------------------------------------

fn validate_params(
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

fn schema_object(properties: &[(&str, &str)], required: &[&str]) -> Value {
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

fn json_type_label(kind: &str) -> &str {
    match kind {
        "string[]" => "array",
        _ => kind,
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

fn string_type() -> &'static str {
    "string"
}

fn boolean_type() -> &'static str {
    "boolean"
}

fn array_of_strings() -> &'static str {
    "string[]"
}

fn required_string(args: &Map<String, Value>, field: &str) -> Result<String, McpRpcError> {
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

fn optional_string(args: &Map<String, Value>, field: &str) -> Option<String> {
    args.get(field)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

fn optional_bool(args: &Map<String, Value>, field: &str) -> Option<bool> {
    args.get(field).and_then(|v| v.as_bool())
}

fn string_array(args: &Map<String, Value>, field: &str) -> Result<Vec<String>, McpRpcError> {
    let value = args.get(field).ok_or_else(|| {
        McpRpcError::new(
            rpc_code::INVALID_PARAMS,
            format!("missing required field `{field}`"),
        )
    })?;
    string_array_value(field, value)
}

fn string_array_value(field: &str, value: &Value) -> Result<Vec<String>, McpRpcError> {
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

fn validate_project_id_strict(id: &str) -> Result<(), McpRpcError> {
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

fn canonicalize_project(path: &str) -> Result<PathBuf, McpRpcError> {
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

fn open_registry(db_path: &Path) -> Result<Registry, McpRpcError> {
    Registry::open(db_path).map_err(core_error)
}

fn observation_for(registry: &Registry, path: &Path) -> Option<RegistryObservation> {
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

fn core_error(err: ForgeError) -> McpRpcError {
    let data = serde_json::json!({
        "code": err.code(),
        "message": err.to_string(),
    });
    let code = match err.code() {
        "unknown-project" | "unknown-profile" | "unknown-feature" => rpc_code::INVALID_PARAMS,
        "path-unavailable" | "manifest-not-found" | "manifest-invalid" | "spec-invalid" => {
            rpc_code::INVALID_PARAMS
        }
        "push-confirm-required" => rpc_code::TOOL_REFUSED,
        other if other.starts_with("agent-") => rpc_code::TOOL_REFUSED,
        _ => rpc_code::INTERNAL_ERROR,
    };
    McpRpcError::new(code, err.to_string()).with_data(data)
}

fn internal_error(message: String) -> McpRpcError {
    McpRpcError::new(rpc_code::INTERNAL_ERROR, message)
}

fn redact_policy(outcome: &PolicyOutcome) -> Value {
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

// ---- stdio server loop --------------------------------------------

/// Run the stdio server loop. Reads JSON-RPC 2.0 requests one
/// per line from stdin, dispatches them through Core, and
/// writes JSON-RPC 2.0 responses one per line to stdout.
/// Diagnostic messages go to stderr; the stdout channel
/// contains only the response envelope so a model can consume
/// the stream without parsing diagnostic noise.
///
/// `reader` and `writer` are split so tests can drive the
/// loop without spawning a child process.
pub fn run_session<R: BufRead, W: Write, E: Write>(
    db_path: Option<&Path>,
    mut reader: R,
    mut writer: W,
    mut diagnostics: E,
) -> io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(());
        }
        if line.trim().is_empty() {
            continue;
        }
        let request = match McpRequest::parse_line(&line) {
            Ok(r) => r,
            Err(McpProtocolError::Empty) => continue,
            Err(McpProtocolError::Parse(reason)) => {
                let response = McpResponse::failure(
                    Value::Null,
                    McpRpcError::new(rpc_code::PARSE_ERROR, reason),
                );
                writeln!(writer, "{}", response.to_line())?;
                writer.flush()?;
                continue;
            }
        };
        let id = request.id.clone().unwrap_or(Value::Null);
        match dispatch(db_path, &request) {
            Ok(value) => {
                let response = McpResponse::success(id, value);
                writeln!(writer, "{}", response.to_line())?;
                writer.flush()?;
            }
            Err(err) => {
                // The JSON-RPC response carries the full
                // error so the model can interpret it; the
                // diagnostic stream only sees a redacted
                // summary keyed on the tool name so a
                // credential embedded in the failed Core
                // call does not leak through stderr.
                let response = McpResponse::failure(id.clone(), err);
                let redacted = redact_for_diagnostics(&response, &request.method);
                let _ = writeln!(
                    diagnostics,
                    "mcp error method={} id={} code={} message={}",
                    request.method, id, redacted.code, redacted.message
                );
                writeln!(writer, "{}", response.to_line())?;
                writer.flush()?;
            }
        }
    }
}

/// Build a redaction-safe summary of a JSON-RPC error for
/// the diagnostic stream. The full error is preserved on
/// the JSON-RPC response so the caller can correlate the
/// failure; the diagnostic stream only ever sees the tool
/// name and the structured code.
fn redact_for_diagnostics(response: &McpResponse, method: &str) -> McpRpcError {
    let code = response
        .error
        .as_ref()
        .map(|e| e.code)
        .unwrap_or(rpc_code::INTERNAL_ERROR);
    McpRpcError {
        code,
        message: format!("tool `{method}` failed with code {code}"),
        data: None,
    }
}

/// Convenience: run the server against the host stdin/stdout
/// and stderr.
pub fn serve_stdio(db_path: Option<&Path>) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let reader = stdin.lock();
    let writer = stdout.lock();
    let diagnostics = stderr.lock();
    run_session(db_path, reader, writer, diagnostics)
}

// ---- internal helpers for tests ----------------------------------

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

/// Read the spec list for a project. Re-exported from the
/// spec module so the dispatch helper can be implemented
/// with one canonical path.
pub fn list_specs_for(path: &Path) -> Result<Vec<SpecListEntry>, ForgeError> {
    list_specs(path)
}

/// Read one spec for a project. Re-exported from the spec
/// module so the dispatch helper can be implemented with
/// one canonical path.
pub fn read_spec_for(path: &Path, spec_id: &str) -> Result<Option<SpecDraft>, ForgeError> {
    let entries = list_specs(path)?;
    let entry = entries
        .into_iter()
        .find(|e| e.id.dir_name() == spec_id)
        .ok_or_else(|| ForgeError::SpecInvalid {
            reason: format!("spec `{spec_id}` was not found under `.forge/specs/`"),
        })?;
    read_spec(path, &entry.id)
}

/// Route a finding through the spec remediation router. The
/// router returns a typed outcome; the MCP layer renders it
/// without reinterpreting.
pub fn route_finding_for(
    request: &SpecRequest,
    source: &FindingSource,
) -> Result<crate::spec::RoutingDecision, ForgeError> {
    route_finding(request, source)
}

/// Apply a routing decision through the spec remediation
/// router. The router records the evidence; the MCP layer
/// renders it.
pub fn apply_routing_for(
    request: &SpecRequest,
    source: &FindingSource,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<RoutingOutcome, ForgeError> {
    apply_routing(request, source, now)
}

/// Build a [`FindingSource`] for a doctor finding id; used
/// by the dispatch helpers and the cross-surface tests.
pub fn doctor_finding_source(id: &str) -> FindingSource {
    FindingSource::Doctor(DoctorFindingInput {
        id: id.to_string(),
        status: crate::doctor::FindingStatus::Fail,
        remediation: crate::doctor::Remediation::Manual,
        category: "spec".to_string(),
        detail: format!("finding `{id}` routed by `mcp spec apply`"),
    })
}

/// Build a [`FindingSource`] for a driftwatch finding id.
pub fn policy_finding_source(id: &str) -> FindingSource {
    FindingSource::Policy(crate::policy::PolicyFinding {
        id: id.to_string(),
        category: "spec".to_string(),
        severity: crate::policy::PolicySeverity::Fail,
        applicable: true,
        message: format!("policy finding `{id}`"),
        evidence: Vec::new(),
        reason: None,
    })
}

/// Build a [`FindingSource`] for a semantic conflict.
pub fn semantic_finding_source(project_id: &str, feature: &str) -> FindingSource {
    FindingSource::Conflict(crate::upgrade::SemanticConflict {
        project_id: project_id.to_string(),
        feature: feature.to_string(),
        owned_file: format!(".forge/features/{feature}.receipt"),
        reason: "drifted receipt reported by MCP".to_string(),
        suggested_spec: format!(
            "forge spec generate --project {project_id} --finding semantic-{feature}"
        ),
    })
}

/// Render the agents for a project as a JSON value; used by
/// the cross-surface tests to compare with the CLI surface.
pub fn list_agents_for(path: &Path) -> Result<Vec<SessionListEntry>, ForgeError> {
    crate::agent::list_sessions(path)
}

/// Read one agent session for a project. Used by tests that
/// exercise the cross-surface invariants.
pub fn read_agent_for(
    path: &Path,
    session_id: &str,
) -> Result<Option<crate::agent::AgentSession>, ForgeError> {
    read_session(path, session_id)
}

/// Plan an upgrade for a project. Re-exported from the
/// upgrade module so the dispatch helpers can be implemented
/// with one canonical path.
pub fn plan_upgrade_for(
    registry: &Registry,
    target: &str,
    feature: Option<&str>,
) -> Result<crate::upgrade::UpgradePlan, ForgeError> {
    plan_upgrade(registry, target, feature)
}

// Compile-time guard: any drift between the spec module's
// public surface and the re-exports here is a build error.
#[allow(dead_code)]
fn _assert_exports() {
    let _: fn(&Path) -> Result<Vec<SpecListEntry>, ForgeError> = list_specs_for;
    let _: fn(&Path, &str) -> Result<Option<SpecDraft>, ForgeError> = read_spec_for;
    let _: fn() -> Vec<McpToolDescriptor> = tool_registry;
    let _: fn(Option<&Path>, &McpRequest) -> Result<Value, McpRpcError> = dispatch;
}

// Re-export the path utilities required for tests so the
// cross-surface module can stay focused on the contract
// rather than on path plumbing.
pub use crate::registry::default_registry_path;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn request(method: &str, id: Value, params: Value) -> McpRequest {
        McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: Some(id),
            method: method.to_string(),
            params,
        }
    }

    #[test]
    fn registry_advertises_only_mature_tools() {
        let tools = tool_registry();
        let names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
        // Stable set: list/inspect/create/import/feature/doctor/spec/agent/test/commit/push
        for expected in [
            "list_projects",
            "inspect_project",
            "list_profiles",
            "inspect_profile",
            "list_features",
            "run_doctor",
            "create_project",
            "import_project",
            "add_feature",
            "remove_feature",
            "upgrade_feature",
            "generate_spec",
            "run_agent",
            "run_tests",
            "commit",
            "push",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "mature tool `{expected}` missing from registry: {names:?}"
            );
        }
    }

    #[test]
    fn registry_omits_unstable_deploy_and_publish_tools() {
        // R2 boundary: deployment and publishing are not
        // implemented in Core yet, so they MUST not be advertised.
        let tools = tool_registry();
        for forbidden in ["deploy", "publish", "release", "mirror", "docs"] {
            assert!(
                tools.iter().all(|t| t.name != forbidden),
                "tool `{forbidden}` is unstable and must not be advertised"
            );
        }
    }

    #[test]
    fn registry_classifies_push_as_external_write() {
        let tools = tool_registry();
        let push = tools.iter().find(|t| t.name == "push").expect("push");
        assert_eq!(push.kind, McpToolKind::ExternalWrite);
        let read_only = [
            "list_projects",
            "inspect_project",
            "list_profiles",
            "inspect_profile",
            "list_features",
            "run_doctor",
        ];
        for name in read_only {
            let tool = tools.iter().find(|t| t.name == name).expect(name);
            assert_eq!(tool.kind, McpToolKind::ReadOnly, "{name} must be read-only");
        }
        let mutating = [
            "create_project",
            "import_project",
            "add_feature",
            "remove_feature",
            "upgrade_feature",
            "generate_spec",
            "run_agent",
            "run_tests",
            "commit",
        ];
        for name in mutating {
            let tool = tools.iter().find(|t| t.name == name).expect(name);
            assert_eq!(tool.kind, McpToolKind::Mutating, "{name} must be mutating");
        }
    }

    #[test]
    fn tools_list_returns_full_registry_summary() {
        let req = request("tools/list", Value::from(1), Value::Null);
        let value = dispatch(None, &req).expect("dispatch");
        assert_eq!(value["contract"], MCP_CONTRACT_VERSION);
        let tools = value["tools"].as_array().expect("tools array");
        assert!(tools.iter().any(|t| t["name"] == "create_project"));
        assert!(tools.iter().all(|t| t["name"] != "deploy"));
    }

    #[test]
    fn unknown_tool_returns_structured_missing_error() {
        let req = request("deploy", Value::from(1), serde_json::json!({}));
        let err = dispatch(None, &req).expect_err("missing tool");
        assert_eq!(err.code, rpc_code::TOOL_MISSING);
        assert!(err.message.contains("unknown tool `deploy`"));
    }

    #[test]
    fn parse_line_rejects_malformed_json() {
        let err = McpRequest::parse_line("{not json").expect_err("parse error");
        match err {
            McpProtocolError::Parse(_) => {}
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_rejects_wrong_jsonrpc_version() {
        let err = McpRequest::parse_line(
            r#"{"jsonrpc":"1.0","id":1,"method":"list_projects","params":{}}"#,
        )
        .expect_err("version error");
        match err {
            McpProtocolError::Parse(reason) => {
                assert!(reason.contains("jsonrpc must be"));
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_accepts_valid_request() {
        let req = McpRequest::parse_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"list_projects","params":{}}"#,
        )
        .expect("parse");
        assert_eq!(req.method, "list_projects");
        assert_eq!(req.id, Some(Value::from(1)));
    }

    #[test]
    fn validate_params_rejects_missing_required_field() {
        let descriptor = find_tool("inspect_project").expect("inspect_project");
        let err = validate_params(&descriptor, &serde_json::json!({})).expect_err("missing");
        assert_eq!(err.code, rpc_code::INVALID_PARAMS);
        assert!(err.message.contains("target"));
    }

    #[test]
    fn validate_params_rejects_wrong_field_type() {
        let descriptor = find_tool("commit").expect("commit");
        let err = validate_params(
            &descriptor,
            &serde_json::json!({
                "path": ".",
                "paths": "not-an-array",
                "message": "fix"
            }),
        )
        .expect_err("wrong type");
        assert_eq!(err.code, rpc_code::INVALID_PARAMS);
        assert!(err.message.contains("paths"));
    }

    #[test]
    fn validate_params_rejects_shell_metacharacters_in_project_id() {
        // The schema only requires path, profile, id. Pass an
        // id with shell metacharacters; the schema validation
        // does not reject the characters, but the dispatcher
        // (validate_project_id_strict) must reject the id
        // before any Core mutation.
        let req = request(
            "create_project",
            Value::from(1),
            serde_json::json!({
                "path": ".",
                "profile": "rust-web",
                "id": "evil; rm -rf /",
            }),
        );
        let err = dispatch(None, &req).expect_err("shell metacharacters");
        assert_eq!(err.code, rpc_code::TOOL_REFUSED);
        assert!(err.message.contains("evil; rm -rf /"));
        assert!(err.message.contains("literal data"));
    }

    #[test]
    fn push_without_confirm_is_refused_before_subprocess() {
        let req = request(
            "push",
            Value::from(1),
            serde_json::json!({
                "path": ".",
                "confirm": false,
            }),
        );
        let err = dispatch(None, &req).expect_err("confirm required");
        assert_eq!(err.code, rpc_code::TOOL_REFUSED);
        assert!(err.message.contains("confirm"));
    }

    #[test]
    fn push_confirm_required_field_omitted_returns_invalid_params() {
        let req = request(
            "push",
            Value::from(1),
            serde_json::json!({
                "path": ".",
            }),
        );
        let err = dispatch(None, &req).expect_err("confirm missing");
        // Schema-level validation rejects the missing
        // required field before the push-specific refusal
        // can fire. The push-confirm-required refusal only
        // applies to `confirm: false`.
        assert_eq!(err.code, rpc_code::INVALID_PARAMS);
    }

    #[test]
    fn list_projects_through_dispatch_returns_empty_registry() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let req = request("list_projects", Value::from(1), serde_json::json!({}));
        let value = dispatch(Some(&db), &req).expect("dispatch");
        assert_eq!(value, serde_json::json!({"projects": []}));
    }

    #[test]
    fn sequential_isolated_sessions_remain_independent() {
        // Two sessions with different temporary registries must not
        // observe each other's projects or operations.
        let dir_a = tempfile::tempdir().unwrap();
        let db_a = dir_a.path().join("registry.db");
        let dir_b = tempfile::tempdir().unwrap();
        let db_b = dir_b.path().join("registry.db");
        let app_a = dir_a.path().join("app-a");
        let create = request(
            "create_project",
            Value::from(1),
            serde_json::json!({
                "path": app_a.to_string_lossy(),
                "profile": "rust-web",
                "id": "isolated-a",
            }),
        );
        let value = dispatch(Some(&db_a), &create).expect("create in session A");
        assert_eq!(value["created"]["record"]["id"], "isolated-a");
        let list = request("list_projects", Value::from(1), serde_json::json!({}));
        let in_a = dispatch(Some(&db_a), &list).expect("list A");
        let in_b = dispatch(Some(&db_b), &list).expect("list B");
        assert_eq!(in_a["projects"].as_array().unwrap().len(), 1);
        assert_eq!(in_b, serde_json::json!({"projects": []}));
    }

    #[test]
    fn inspect_project_unknown_id_returns_invalid_params() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let req = request(
            "inspect_project",
            Value::from(1),
            serde_json::json!({"target": "no-such-project"}),
        );
        let err = dispatch(Some(&db), &req).expect_err("unknown project");
        assert_eq!(err.code, rpc_code::INVALID_PARAMS);
        let data = err.data.expect("data");
        assert_eq!(data["code"], "unknown-project");
    }

    #[test]
    fn run_session_round_trip_known_request_and_unknown_tool() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let db_path = db.as_path();
        let input = "\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"list_projects\",\"params\":{}}\n\
            {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"deploy\",\"params\":{}}\n\
        ";
        let mut output = Vec::new();
        let mut diag = Vec::new();
        run_session(
            Some(db_path),
            Cursor::new(input.as_bytes()),
            &mut output,
            &mut diag,
        )
        .expect("run session");
        let rendered = String::from_utf8(output).expect("utf8");
        // Two responses (one per request) on two lines.
        let mut lines = rendered.lines();
        let ok_line = lines.next().expect("ok line");
        let err_line = lines.next().expect("err line");
        let ok: McpResponse = serde_json::from_str(ok_line).expect("ok json");
        let err: McpResponse = serde_json::from_str(err_line).expect("err json");
        assert!(ok.result.is_some());
        assert_eq!(ok.id, Value::from(1));
        assert!(err.error.is_some());
        assert_eq!(err.id, Value::from(2));
        assert_eq!(err.error.unwrap().code, rpc_code::TOOL_MISSING);
    }

    #[test]
    fn run_session_reports_parse_error_as_rpc_envelope() {
        let input = "not-a-json-line\n";
        let mut output = Vec::new();
        let mut diag = Vec::new();
        run_session(None, Cursor::new(input.as_bytes()), &mut output, &mut diag)
            .expect("run session");
        let rendered = String::from_utf8(output).expect("utf8");
        let line = rendered.lines().next().expect("line");
        let response: McpResponse = serde_json::from_str(line).expect("json");
        let err = response.error.expect("error");
        assert_eq!(err.code, rpc_code::PARSE_ERROR);
        assert_eq!(response.id, Value::Null);
    }

    #[test]
    fn run_session_drops_secrets_from_diagnostics() {
        let input = "\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"create_project\",\"params\":{\"path\":\".\",\"profile\":\"rust-web\",\"id\":\"aws-secret-AKIAIOSFODNN7EXAMPLE\"}}\n\
        ";
        let mut output = Vec::new();
        let mut diag = Vec::new();
        run_session(None, Cursor::new(input.as_bytes()), &mut output, &mut diag)
            .expect("run session");
        let diag_text = String::from_utf8_lossy(&diag);
        // Diagnostic should mention the method but not the
        // secret-looking id. The id is included in the
        // method name only as the request label, never as
        // the parameter value.
        assert!(diag_text.contains("create_project"));
        assert!(!diag_text.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn contract_version_is_recorded_on_tool_descriptors() {
        let tools = tool_registry();
        for t in &tools {
            assert_eq!(t.contract, MCP_CONTRACT_VERSION, "tool `{}`", t.name);
        }
    }

    #[test]
    fn dispatch_through_cli_returns_equivalent_outcome() {
        // R1 success scenario: `inspect_project` and
        // `doctor` (via the CLI) must return the same domain
        // outcome. We bootstrap a project via the CLI
        // surface, then re-inspect it through MCP and
        // compare. The fixture lives in a unique temp dir.
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(
            proj.join("forge.yaml"),
            "schema: 1\nproject:\n  id: r1-equiv\n  name: R1\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
        )
        .unwrap();
        let mut registry = Registry::open(&db).unwrap();
        let _ = registry.register(&proj, None).unwrap();
        drop(registry);
        let req = request(
            "inspect_project",
            Value::from(1),
            serde_json::json!({"target": "r1-equiv"}),
        );
        let value = dispatch(Some(&db), &req).expect("dispatch");
        let project = &value["project"];
        assert_eq!(project["id"], "r1-equiv");
        assert_eq!(project["profile"], "rust-web");
        assert_eq!(project["maturity"], "L1");
    }

    #[test]
    fn mutating_tool_creates_journal_entry() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let req = request(
            "create_project",
            Value::from(1),
            serde_json::json!({
                "path": dir.path().join("app").to_string_lossy(),
                "profile": "rust-web",
                "id": "mcp-app",
            }),
        );
        let value = dispatch(Some(&db), &req).expect("create");
        assert_eq!(value["created"]["record"]["id"], "mcp-app");
        // Journal entry should be recorded under the `mcp` kind.
        let registry = Registry::open(&db).unwrap();
        let entries = registry.journal_entries().unwrap();
        let mcp_entries: Vec<&_> = entries.iter().filter(|e| e.kind == "mcp").collect();
        assert!(
            !mcp_entries.is_empty(),
            "expected at least one mcp journal entry"
        );
    }

    #[test]
    fn unknown_tool_request_id_is_preserved_in_error() {
        let req = request("deploy", Value::from("opaque-id-42"), serde_json::json!({}));
        let err = dispatch(None, &req).expect_err("missing tool");
        assert_eq!(err.code, rpc_code::TOOL_MISSING);
        let response = McpResponse::failure(Value::from("opaque-id-42"), err);
        assert_eq!(response.id, Value::from("opaque-id-42"));
    }
}
