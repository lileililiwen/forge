//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::constants::MCP_CONTRACT_VERSION;
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
use crate::doctor::{parse_target_level, run_doctor};
use crate::feature::{add_feature, remove_feature, upgrade_feature};
use crate::generate::{generate, normalize_explicit, GeneratedProject};
use crate::gitops::{commit_paths, push_ref, run_test, CommitOutcome, PushOutcome, TestOutcome};
use crate::import::{adopt_import, inspect_import};
use crate::policy::{run_driftwatch, DriftWatchConfig};
use crate::profile::inspect_profile;
use crate::registry::Registry;
use crate::spec::{
    apply_routing, ensure_single_project, generate_spec, list_specs, read_spec, route_finding,
    DoctorFindingInput, FindingSource, RoutingOutcome, SpecDraft, SpecGenerateOutcome,
    SpecListEntry, SpecRequest,
};
use crate::upgrade::plan_upgrade;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

use super::model::{rpc_code, McpRequest, McpRpcError, McpToolDescriptor, McpToolKind};
use super::params::{
    canonicalize_project, core_error, internal_error, observation_for, open_registry,
    optional_bool, optional_string, redact_policy, required_string, string_array,
    string_array_value, validate_params, validate_project_id_strict,
};
use super::tools::tool_registry;

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
        "list_projects" => mcp_list_projects(db_path, args),
        "inspect_project" => mcp_inspect_project(db_path, args),
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

/// `list_projects` — thin adapter over the shared Core catalog
/// service (`src/catalog/`). The tool accepts the same
/// filter/source/pagination parameters the CLI exposes, builds a
/// `CatalogQuery`, and serialises the resulting `CatalogPage` as
/// the `content` field of one MCP tool result. Filtering, ordering
/// and pagination live in Core; this handler adds no rule of its
/// own.
fn mcp_list_projects(db_path: &Path, args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    use crate::catalog;
    let selection = build_catalog_selection_from_mcp(args)?;
    let max_age = optional_i64(args, "max_age").unwrap_or(catalog::DEFAULT_MAX_AGE_SECONDS);
    catalog::validate_max_age(max_age).map_err(core_error)?;
    let limit = optional_usize(args, "limit").unwrap_or(catalog::DEFAULT_LIMIT);
    let cursor = optional_string(args, "cursor");
    let pairs = catalog_filter_pairs_from_mcp(args);
    let query = catalog::CatalogQuery::from_pairs(&pairs, limit, cursor)
        .map_err(core_error)?
        .normalize();
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: max_age,
        now: chrono::Utc::now(),
    });
    let mut page =
        catalog::apply(&bundle.records, &query, &bundle.observed_at).map_err(core_error)?;
    page.sources = bundle.statuses.clone();
    let value = serde_json::to_value(&page).map_err(|err| internal_error(err.to_string()))?;
    Ok(serde_json::json!({ "catalog": value }))
}

/// `inspect_project` — thin adapter over the Core catalog service
/// that returns every catalog record for the requested project id
/// across the selected sources. The transport adds no rule; the
/// record order and set are the Core's.
fn mcp_inspect_project(db_path: &Path, args: &Map<String, Value>) -> Result<Value, McpRpcError> {
    use crate::catalog;
    let target = required_string(args, "target")?;
    crate::core::validate_project_id(&target).map_err(|reason| {
        McpRpcError::new(
            rpc_code::TOOL_REFUSED,
            format!("invalid project id `{target}`: {reason}"),
        )
        .with_data(serde_json::json!({
            "code": "project-id-invalid",
            "message": format!("invalid project id: {reason}"),
        }))
    })?;
    let selection = build_catalog_selection_from_mcp(args)?;
    let max_age = optional_i64(args, "max_age").unwrap_or(catalog::DEFAULT_MAX_AGE_SECONDS);
    catalog::validate_max_age(max_age).map_err(core_error)?;
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: max_age,
        now: chrono::Utc::now(),
    });
    let records = catalog::inspect_records(&bundle, &target).map_err(core_error)?;
    let page = serde_json::json!({
        "contract": catalog::CATALOG_CONTRACT_VERSION,
        "project_id": target,
        "records": records,
    });
    Ok(serde_json::json!({ "catalog": page }))
}

fn build_catalog_selection_from_mcp(
    args: &Map<String, Value>,
) -> Result<crate::catalog::CatalogSourceSelection, McpRpcError> {
    use crate::catalog;
    let mut kinds: Vec<catalog::SourceKind> = Vec::new();
    let sources = match args.get("sources") {
        Some(value) => string_array_value("sources", value).unwrap_or_default(),
        None => Vec::new(),
    };
    if sources.is_empty() {
        kinds.push(catalog::SourceKind::Local);
    } else {
        for raw in &sources {
            let kind = catalog::SourceKind::parse(raw).ok_or_else(|| {
                McpRpcError::new(
                    rpc_code::INVALID_PARAMS,
                    format!(
                        "unknown source `{raw}`; expected one of \
                         local|git|workspace-registry|inventory|github"
                    ),
                )
            })?;
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    let workspace_registry = args
        .get("workspace_registry")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| crate::fleet::resolve_registry_path(None));
    let inventory = args
        .get("inventory")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| crate::publish::inventory::resolve_source(None));
    let git_repositories = match args.get("git_repositories") {
        Some(value) => string_array_value("git_repositories", value)
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        None => Vec::new(),
    };
    let github_repositories = match args.get("github_repositories") {
        Some(value) => string_array_value("github_repositories", value).unwrap_or_default(),
        None => Vec::new(),
    };
    Ok(catalog::CatalogSourceSelection {
        kinds,
        git_repositories,
        workspace_registry,
        inventory,
        github_repositories,
    })
}

fn catalog_filter_pairs_from_mcp(args: &Map<String, Value>) -> Vec<String> {
    let mut pairs: Vec<String> = Vec::new();
    for (field, key) in [
        ("tags", "tag"),
        ("languages", "language"),
        ("profiles", "profile"),
        ("lifecycles", "lifecycle"),
        ("repositories", "repository"),
        ("ci", "ci"),
        ("compose", "compose"),
        ("evidence", "evidence"),
    ] {
        if let Some(value) = args.get(field) {
            for value in string_array_value(field, value).unwrap_or_default() {
                pairs.push(format!("{key}={value}"));
            }
        }
    }
    if let Some(value) = args.get("filters") {
        pairs.extend(string_array_value("filters", value).unwrap_or_default());
    }
    pairs
}

fn optional_i64(args: &Map<String, Value>, field: &str) -> Option<i64> {
    args.get(field).and_then(Value::as_i64)
}

fn optional_usize(args: &Map<String, Value>, field: &str) -> Option<usize> {
    args.get(field)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
}

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
        None,
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
    if transition == "run_spec" {
        let supervisor = if provider == crate::agent::SISYPHUSFY_SUPERVISOR {
            Some(crate::agent::SISYPHUSFY_SUPERVISOR)
        } else {
            match provider.as_str() {
                "opencode" | "codex" | "ariadex" => None,
                other => {
                    return Err(McpRpcError::new(
                        rpc_code::INVALID_PARAMS,
                        format!(
                            "unknown agent provider `{other}`; expected one of: opencode, codex, ariadex, sisyphusfy (run_spec only)"
                        ),
                    ));
                }
            }
        };
        return mcp_run_agent_run_spec(
            db_path,
            &canonical,
            &project_id,
            &session_id,
            supervisor,
            now,
        );
    }
    let provider_id = match provider.as_str() {
        "opencode" => AgentProvider::Opencode,
        "codex" => AgentProvider::Codex,
        "ariadex" => AgentProvider::Ariadex,
        other => {
            return Err(McpRpcError::new(
                rpc_code::INVALID_PARAMS,
                format!(
                    "unknown agent provider `{other}`; expected one of: opencode, codex, ariadex"
                ),
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
    supervisor: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Value, McpRpcError> {
    let outcome = run_session_spec(canonical, session_id, supervisor, now).map_err(core_error)?;
    let files = crate::agent::write_session(canonical, &outcome.session).map_err(core_error)?;
    let registry = open_registry(db_path)?;
    let verdict = outcome
        .verdict
        .clone()
        .unwrap_or_else(|| "done".to_string());
    let detail = format!(
        "mcp run_agent `{}` run_spec session `{}` -> `{}` (verdict `{}`)",
        project_id,
        session_id,
        outcome.state.label(),
        verdict
    );
    let _ = registry.record_operation("mcp", project_id, &verdict, &detail);
    Ok(serde_json::json!({
        "transition": agent_outcome_envelope(&outcome),
        "files_written": files,
    }))
}

/// Render an [`AgentTransitionOutcome`] as a JSON object
/// matching the CLI's `forge agent` envelope: `session_id`
/// and `project_id` are hoisted so a model can address the
/// session without navigating into a nested `session`
/// object, mirroring the CLI's `agent_outcome_output`. The
/// `verdict` key appears only for delegated run-spec
/// executions, mirroring the CLI envelope.
fn agent_outcome_envelope(outcome: &AgentTransitionOutcome) -> Value {
    let mut value = serde_json::json!({
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
    });
    if let Some(verdict) = &outcome.verdict {
        value["verdict"] = serde_json::Value::String(verdict.clone());
    }
    value
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

pub(super) fn mcp_mirror_project(args: &Map<String, Value>) -> Result<Value, McpRpcError> {
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

#[allow(dead_code)]
fn _assert_exports() {
    let _: fn(&Path) -> Result<Vec<SpecListEntry>, ForgeError> = list_specs_for;
    let _: fn(&Path, &str) -> Result<Option<SpecDraft>, ForgeError> = read_spec_for;
    let _: fn() -> Vec<McpToolDescriptor> = tool_registry;
    let _: fn(Option<&Path>, &McpRequest) -> Result<Value, McpRpcError> = dispatch;
}
