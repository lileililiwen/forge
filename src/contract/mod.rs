use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::release::engine::read_release;

pub const CONTRACT_CONTRACT_VERSION: &str = "0.1.0";
pub const CONTRACT_PATTERN: &str =
    r"^platform\.[a-z][a-z0-9]*(?:-[a-z0-9]+)*\/[0-9]+\.[0-9]+\.[0-9]+$";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub sha256: String,
    /// Owning source for this file (`platform-contracts` or
    /// `workspace-governance`). Entries written before per-file
    /// provenance carry `None` and mean the manifest's top-level source.
    #[serde(default)]
    pub source: Option<String>,
    /// Source revision the file was vendored from (full commit SHA when
    /// the sync could resolve one, else `"unknown"`).
    #[serde(default)]
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractManifest {
    pub schema_version: u64,
    pub source: String,
    pub revision: String,
    pub synced_at: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone)]
pub struct ContractSpec {
    pub module: &'static str,
    pub constant: &'static str,
    pub discriminator: &'static str,
    pub version: &'static str,
    pub platform_family: Option<&'static str>,
    pub doc: &'static str,
}

pub const CONTRACTS: &[ContractSpec] = &[
    ContractSpec {
        module: "agent",
        constant: "AGENT_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/agent-runtime-workflows/spec.md",
    },
    ContractSpec {
        module: "analytics",
        constant: "ANALYTICS_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/external-planes-analytics/spec.md",
    },
    ContractSpec {
        module: "api",
        constant: "API_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/core-http-api/spec.md",
    },
    ContractSpec {
        module: "checker",
        constant: "CHECKER_CONTRACT_VERSION",
        discriminator: "forge-checker/0.1.0",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/external-checker-emission/spec.md",
    },
    ContractSpec {
        module: "component",
        constant: "COMPONENT_CATALOG_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/semantic-component-registry/spec.md",
    },
    ContractSpec {
        module: "deploy",
        constant: "DEPLOY_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/adapter-deployment/spec.md",
    },
    ContractSpec {
        module: "deploy",
        constant: "DEPLOY_EXECUTOR_CONTRACT",
        discriminator: "forge-deploy-executor/0.1.0",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/adapter-deployment/spec.md",
    },
    ContractSpec {
        module: "distribution",
        constant: "DISTRIBUTION_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/repository-distribution/spec.md",
    },
    ContractSpec {
        module: "docs",
        constant: "DOCS_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/documentation-translation/spec.md",
    },
    ContractSpec {
        module: "doctor",
        constant: "DOCTOR_POLICY_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/doctor-maturity-assessment/spec.md",
    },
    ContractSpec {
        module: "feature",
        constant: "FEATURE_CATALOG_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/feature-lifecycle/spec.md",
    },
    ContractSpec {
        module: "fleet",
        constant: "FLEET_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/fleet-registry-observation/spec.md",
    },
    ContractSpec {
        module: "gate",
        constant: "GATE_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: Some("platform.gate-result"),
        doc: "openspec/specs/gate-runtime-evidence/spec.md",
    },
    ContractSpec {
        module: "generate",
        constant: "GENERATOR_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/deterministic-project-generation/spec.md",
    },
    ContractSpec {
        module: "gitops",
        constant: "GITOPS_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/agent-runtime-workflows/spec.md",
    },
    ContractSpec {
        module: "governance",
        constant: "GOVERNANCE_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/governance-provider-contract/spec.md",
    },
    ContractSpec {
        module: "identity",
        constant: "IDENTITY_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/central-admin-identity/spec.md",
    },
    // The shared-layer kit registry is a versioned surface: its descriptor
    // contract, the .NET kit version it pins and the vendored token kit
    // version are all registered here, so none can be bumped without the
    // inventory noticing.
    ContractSpec {
        module: "kit",
        constant: "KIT_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/scaffold-prewires-shared-layer/spec.md",
    },
    ContractSpec {
        module: "kit",
        constant: "PLATFORM_PACKAGE_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/scaffold-prewires-shared-layer/spec.md",
    },
    ContractSpec {
        module: "kit",
        constant: "PLATFORM_UI_KIT_VERSION",
        discriminator: "",
        version: "0.2.0",
        platform_family: None,
        doc: "openspec/specs/scaffold-prewires-shared-layer/spec.md",
    },
    ContractSpec {
        module: "mcp",
        constant: "MCP_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/mature-mcp-surface/spec.md",
    },
    ContractSpec {
        module: "planner",
        constant: "PLANNER_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/validated-intent-planner/spec.md",
    },
    ContractSpec {
        module: "policy",
        constant: "POLICY_CONTRACT_VERSION",
        discriminator: "driftwatch-checker/0.1.0",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/quality-policy-integration/spec.md",
    },
    ContractSpec {
        module: "portal",
        constant: "PORTAL_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/control-plane-portal/spec.md",
    },
    ContractSpec {
        module: "procedure",
        constant: "PROCEDURE_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/ai-procedure-skills/spec.md",
    },
    ContractSpec {
        module: "provider",
        constant: "PROVIDER_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/external-planes-analytics/spec.md",
    },
    ContractSpec {
        module: "readiness",
        constant: "READINESS_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: Some("platform.readiness"),
        doc: "openspec/specs/profile-registry/spec.md",
    },
    ContractSpec {
        module: "release",
        constant: "RELEASE_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: Some("platform.release-evidence"),
        doc: "openspec/specs/release-publishing/spec.md",
    },
    ContractSpec {
        module: "spec",
        constant: "SPEC_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/specification-remediation/spec.md",
    },
    ContractSpec {
        module: "ui_pattern",
        constant: "UI_PATTERN_CATALOG_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/semantic-ui-patterns/spec.md",
    },
    ContractSpec {
        module: "upgrade",
        constant: "UPGRADE_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/project-upgrade-orchestration/spec.md",
    },
    ContractSpec {
        module: "contract",
        constant: "CONTRACT_CONTRACT_VERSION",
        discriminator: "",
        version: "0.1.0",
        platform_family: None,
        doc: "openspec/specs/platform-contract-consumption/spec.md",
    },
];

#[derive(Debug, Clone)]
pub struct ContractUnmappable {
    pub family: String,
    pub value: String,
}

impl std::fmt::Display for ContractUnmappable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "value '{}' has no mapping in family '{}'",
            self.value, self.family
        )
    }
}

pub fn contracts_dir() -> std::path::PathBuf {
    if let Some(dir) =
        std::env::var_os("FORGE_CONTRACTS_DIR").filter(|v| !v.to_string_lossy().trim().is_empty())
    {
        return std::path::PathBuf::from(dir);
    }
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("contracts")
}

pub fn load_manifest() -> Result<ContractManifest, ForgeError> {
    let path = contracts_dir().join("manifest.json");
    let text = fs::read_to_string(&path).map_err(|_| ForgeError::ContractInvalid {
        reason: format!(
            "contracts/manifest.json missing or unreadable at {}",
            path.display()
        ),
    })?;
    serde_json::from_str(&text).map_err(|e| ForgeError::ContractInvalid {
        reason: format!("contracts/manifest.json invalid: {e}"),
    })
}

pub fn verify_manifest_digests() -> Result<(), String> {
    let manifest = load_manifest().map_err(|e| e.to_string())?;
    let base = contracts_dir();
    for entry in &manifest.files {
        let file = base.join(&entry.path);
        let bytes = fs::read(&file).map_err(|e| format!("{}: cannot read: {e}", entry.path))?;
        let actual = crate::release::hash_bytes(&bytes);
        if actual != entry.sha256 {
            return Err(format!(
                "{}: expected {} got {}",
                entry.path, entry.sha256, actual
            ));
        }
    }
    Ok(())
}

pub fn supported_families() -> Vec<&'static str> {
    vec![
        "platform.gate-result",
        "platform.readiness",
        "platform.release-evidence",
        "platform.capability",
        "platform.audit-event",
        "platform.job-outcome",
    ]
}

pub fn family_schema_path(family: &str) -> Option<&'static str> {
    match family {
        "platform.gate-result" => Some("schemas/gate-result.schema.json"),
        "platform.readiness" => Some("schemas/readiness.schema.json"),
        "platform.release-evidence" => Some("schemas/release-evidence.schema.json"),
        "platform.capability" => Some("schemas/capability.schema.json"),
        "platform.audit-event" => Some("schemas/audit-event.schema.json"),
        "platform.job-outcome" => Some("schemas/job-outcome.schema.json"),
        _ => None,
    }
}

pub fn contract_discriminator(family: &str) -> Option<String> {
    Some(format!("{family}/0.1.0"))
}

pub fn secret_field_substrings() -> Vec<String> {
    let path = contracts_dir().join("vocabulary/secret-field-substrings.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(&text).unwrap_or_default()
}

pub fn payload_contains_secret_field(
    payload: &serde_json::Value,
    substrings: &[String],
) -> Option<String> {
    fn walk(value: &serde_json::Value, substrings: &[String], path: &str) -> Option<String> {
        match value {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let lower = k.to_ascii_lowercase();
                    for sub in substrings {
                        if lower.contains(&sub.to_ascii_lowercase()) {
                            return Some(if path.is_empty() {
                                k.clone()
                            } else {
                                format!("{path}.{k}")
                            });
                        }
                    }
                    let child_path = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    if let Some(hit) = walk(v, substrings, &child_path) {
                        return Some(hit);
                    }
                }
                None
            }
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    let child_path = format!("{path}[{i}]");
                    if let Some(hit) = walk(item, substrings, &child_path) {
                        return Some(hit);
                    }
                }
                None
            }
            _ => None,
        }
    }
    walk(payload, substrings, "")
}

pub fn map_gate_aggregate(
    agg: crate::gate::GateAggregate,
) -> Result<&'static str, ContractUnmappable> {
    match agg {
        crate::gate::GateAggregate::Passed => Ok("passed"),
        crate::gate::GateAggregate::Blocked => Ok("failed"),
        crate::gate::GateAggregate::Failed => Ok("errored"),
        crate::gate::GateAggregate::Unknown => Ok("errored"),
    }
}

pub fn map_readiness_status(
    status: crate::readiness::ReadinessStatus,
) -> Result<&'static str, ContractUnmappable> {
    match status {
        crate::readiness::ReadinessStatus::Passed => Ok("ready"),
        crate::readiness::ReadinessStatus::Failed => Ok("not_ready"),
        crate::readiness::ReadinessStatus::Unverified => Err(ContractUnmappable {
            family: "platform.readiness".to_string(),
            value: "unverified".to_string(),
        }),
    }
}

pub fn map_audit_outcome(state: &str) -> Result<&'static str, ContractUnmappable> {
    match state {
        "done" => Ok("success"),
        "failed" => Ok("failure"),
        "blocked" | "rejected" => Ok("denied"),
        other => Err(ContractUnmappable {
            family: "platform.audit-event".to_string(),
            value: other.to_string(),
        }),
    }
}

pub fn map_release_evidence_kind(_stages: &[String]) -> &'static str {
    "dry_run"
}

pub fn build_envelope(family: &str, payload: serde_json::Value) -> serde_json::Value {
    let contract = contract_discriminator(family).unwrap_or_else(|| format!("{family}/0.1.0"));
    let generated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    serde_json::json!({
        "contract": contract,
        "generated_at": generated_at,
        "payload": payload,
    })
}

pub fn validate_envelope(doc: &serde_json::Value) -> Result<(), String> {
    let contract = doc
        .get("contract")
        .and_then(|v| v.as_str())
        .ok_or("missing contract")?;
    let re_ok = contract.starts_with("platform.") && contract.contains('/');
    if !re_ok {
        return Err(format!(
            "contract '{contract}' does not match platform.* pattern"
        ));
    }
    let family = contract.split('/').next().unwrap_or("");
    let version = contract.split('/').nth(1).unwrap_or("");
    if !supported_families().contains(&family) {
        return Err(format!("unsupported family '{family}'"));
    }
    if !version.starts_with("0.") {
        return Err(format!("unsupported major for '{family}': {version}"));
    }
    doc.get("generated_at")
        .and_then(|v| v.as_str())
        .ok_or("missing generated_at")?;
    let payload = doc.get("payload").ok_or("missing payload")?;
    let subs = secret_field_substrings();
    if let Some(hit) = payload_contains_secret_field(payload, &subs) {
        return Err(format!("payload contains secret field '{hit}'"));
    }
    if let Some(schema_path) = family_schema_path(family) {
        let schema_text = fs::read_to_string(contracts_dir().join(schema_path))
            .map_err(|e| format!("cannot read schema {schema_path}: {e}"))?;
        let schema: serde_json::Value =
            serde_json::from_str(&schema_text).map_err(|e| format!("invalid schema: {e}"))?;
        validate_payload_against_schema(payload, &schema)?;
    }
    Ok(())
}

fn validate_payload_against_schema(
    payload: &serde_json::Value,
    schema: &serde_json::Value,
) -> Result<(), String> {
    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        for key in required {
            if let Some(k) = key.as_str() {
                if payload.get(k).is_none() {
                    return Err(format!("payload missing required field '{k}'"));
                }
            }
        }
    }
    if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
        if let Some(payload_obj) = payload.as_object() {
            for (key, val) in payload_obj {
                if let Some(prop_schema) = props.get(key) {
                    if let Some(enum_vals) = prop_schema.get("enum").and_then(|v| v.as_array()) {
                        if let Some(s) = val.as_str() {
                            let allowed: Vec<&str> =
                                enum_vals.iter().filter_map(|v| v.as_str()).collect();
                            if !allowed.contains(&s) {
                                return Err(format!(
                                    "field '{key}' value '{s}' not in enum {allowed:?}"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn emit_gate_result(
    project_dir: &Path,
    _project_id: &str,
) -> Result<serde_json::Value, ForgeError> {
    let evidence = crate::gate::load_latest_evidence(project_dir)
        .map_err(|e| ForgeError::ContractInvalid {
            reason: e.to_string(),
        })?
        .ok_or_else(|| ForgeError::ContractInvalid {
            reason: "no gate evidence for this project".to_string(),
        })?;
    let result =
        map_gate_aggregate(evidence.aggregate).map_err(|e| ForgeError::ContractInvalid {
            reason: e.to_string(),
        })?;
    let gate_id = format!("{}/{}", evidence.project_id, evidence.runtime);
    let mut checks = Vec::new();
    for c in &evidence.checks {
        let status = match c.state {
            crate::gate::GateCheckState::Pass => "passed",
            crate::gate::GateCheckState::Fail => "failed",
            crate::gate::GateCheckState::Skip => "skipped",
            crate::gate::GateCheckState::NotApplicable => "skipped",
            crate::gate::GateCheckState::Unresolved => "errored",
        };
        let mut entry = serde_json::json!({"check_id": c.id, "status": status});
        if let Some(note) = &c.note {
            entry["detail"] = serde_json::Value::String(redact_credentials(note));
        }
        checks.push(entry);
    }
    let payload = serde_json::json!({
        "gate_id": gate_id,
        "pipeline": evidence.runtime,
        "evaluated_at": evidence.observed_at,
        "result": result,
        "subject_ref": evidence.revision,
        "checks": checks,
    });
    let subs = secret_field_substrings();
    if let Some(hit) = payload_contains_secret_field(&payload, &subs) {
        return Err(ForgeError::ContractInvalid {
            reason: format!("payload contains secret field '{hit}'"),
        });
    }
    Ok(build_envelope("platform.gate-result", payload))
}

pub fn emit_readiness(
    project_dir: &Path,
    profile: Option<&str>,
) -> Result<Vec<serde_json::Value>, ForgeError> {
    let rows = if let Some(pid) = profile {
        vec![crate::readiness::run_profile_row(pid, None).map_err(|e| {
            ForgeError::ContractInvalid {
                reason: e.to_string(),
            }
        })?]
    } else {
        crate::readiness::run_matrix(&[])
            .map_err(|e| ForgeError::ContractInvalid {
                reason: e.to_string(),
            })?
            .rows
    };
    let mut out = Vec::new();
    for row in rows {
        let status = map_readiness_status(row.result).map_err(|e| ForgeError::ContractInvalid {
            reason: e.to_string(),
        })?;
        let mut checks = Vec::new();
        if let Some(b) = &row.build {
            checks.push(serde_json::json!({"name": "build", "status": if b.success { status } else { "not_ready" }}));
        }
        if let Some(t) = &row.test {
            checks.push(serde_json::json!({"name": "test", "status": if t.success { status } else { "not_ready" }}));
        }
        let payload = serde_json::json!({
            "service": row.profile,
            "status": status,
            "checked_at": row.generated_at,
            "checks": checks,
        });
        let subs = secret_field_substrings();
        if let Some(hit) = payload_contains_secret_field(&payload, &subs) {
            return Err(ForgeError::ContractInvalid {
                reason: format!("payload contains secret field '{hit}'"),
            });
        }
        out.push(build_envelope("platform.readiness", payload));
    }
    let _ = project_dir;
    Ok(out)
}

pub fn emit_release_evidence(
    project_dir: &Path,
    project_id: &str,
    release_id: &str,
) -> Result<serde_json::Value, ForgeError> {
    let state = read_release(project_dir, project_id, release_id)
        .map_err(|e| ForgeError::ContractInvalid {
            reason: e.to_string(),
        })?
        .ok_or_else(|| ForgeError::ContractInvalid {
            reason: format!("no release state for '{release_id}'"),
        })?;
    let payload = serde_json::json!({
        "release_id": state.identity.id,
        "service": project_id,
        "version": state.identity.version,
        "released_at": state.last_run_at,
        "evidence_kind": "dry_run",
    });
    let subs = secret_field_substrings();
    if let Some(hit) = payload_contains_secret_field(&payload, &subs) {
        return Err(ForgeError::ContractInvalid {
            reason: format!("payload contains secret field '{hit}'"),
        });
    }
    Ok(build_envelope("platform.release-evidence", payload))
}

pub fn emit_capability(
    _project_dir: &Path,
    project_id: &str,
) -> Result<Option<serde_json::Value>, ForgeError> {
    let path = _project_dir.join(".project.json");
    let text = fs::read_to_string(&path).map_err(|_| ForgeError::ContractInvalid {
        reason: "no .project.json capabilities for this project".to_string(),
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| ForgeError::ContractInvalid {
            reason: format!(".project.json invalid: {e}"),
        })?;
    let caps = match value.get("capabilities") {
        Some(v) if v.is_object() || v.is_array() => v,
        _ => return Ok(None),
    };
    let cap_id = caps
        .get("capability_id")
        .and_then(|v| v.as_str())
        .unwrap_or(project_id);
    let payload = serde_json::json!({
        "capability_id": cap_id,
        "service": project_id,
        "version": "0.1.0",
        "declared_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "stability": "experimental",
    });
    let subs = secret_field_substrings();
    if let Some(hit) = payload_contains_secret_field(&payload, &subs) {
        return Err(ForgeError::ContractInvalid {
            reason: format!("payload contains secret field '{hit}'"),
        });
    }
    Ok(Some(build_envelope("platform.capability", payload)))
}

pub fn emit_audit_events(
    registry_path: &Path,
    limit: usize,
) -> Result<Vec<serde_json::Value>, ForgeError> {
    let registry = crate::registry::Registry::open(registry_path)?;
    let entries = registry.recent_operations(limit)?;
    let mut out = Vec::new();
    let mut skipped = 0usize;
    for entry in entries {
        let outcome = match map_audit_outcome(&entry.state) {
            Ok(o) => o,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let category = match entry.kind.as_str() {
            "release" | "deploy" | "mirror" | "distribution" | "docs" => "lifecycle",
            "identity" | "api" => "authentication",
            k if k.contains("governance") => "configuration",
            _ => "lifecycle",
        };
        let payload = serde_json::json!({
            "event_id": format!("evt-{}", entry.op_id),
            "occurred_at": entry.finished_at.as_deref().unwrap_or(&entry.started_at),
            "category": category,
            "action": format!("{}.{}", entry.kind, entry.state),
            "outcome": outcome,
            "actor": {"subject_id": entry.project_id, "subject_type": "service"},
            "tenant_id": "default",
        });
        let subs = secret_field_substrings();
        if payload_contains_secret_field(&payload, &subs).is_some() {
            skipped += 1;
            continue;
        }
        out.push(build_envelope("platform.audit-event", payload));
    }
    let _ = skipped;
    Ok(out)
}

#[cfg(test)]
mod contract_tests;
