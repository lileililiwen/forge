use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::release::engine::read_release;

pub const CONTRACT_CONTRACT_VERSION: &str = "0.1.0";
pub const CONTRACT_PATTERN: &str =
    r"^platform\.[a-z][a-z0-9]*(?:-[a-z0-9]+)*\/[0-9]+\.[0-9]+\.[0-9]+$";

mod inventory;
pub use inventory::{ContractSpec, CONTRACTS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub sha256: String,
    /// Owner of this retained contract asset.
    #[serde(default)]
    pub source: Option<String>,
    /// Content revision for the Forge-maintained asset set.
    #[serde(default)]
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractManifest {
    pub schema_version: u64,
    pub source: String,
    pub revision: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformFamilySupport {
    pub family: String,
    pub status: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlatformFamilyCatalog {
    schema_version: u64,
    families: Vec<PlatformFamilySupport>,
}

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
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            let installed = bin_dir.join("../share/forge/contracts");
            if installed.is_dir() {
                return installed;
            }
        }
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
    if manifest.source != "forge" {
        return Err(format!(
            "contracts/manifest.json owner must be forge, got '{}'",
            manifest.source
        ));
    }
    let base = contracts_dir();
    let mut declared = std::collections::BTreeSet::new();
    let mut revision_rows = Vec::new();
    for entry in &manifest.files {
        let rel = Path::new(&entry.path);
        if rel.is_absolute()
            || rel
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(format!(
                "unsafe contract asset path '{}': refused",
                entry.path
            ));
        }
        if !declared.insert(entry.path.clone()) {
            return Err(format!(
                "duplicate contract asset path '{}': refused",
                entry.path
            ));
        }
        if entry.source.as_deref() != Some("forge") {
            return Err(format!("{}: asset owner must be forge", entry.path));
        }
        if entry.revision.as_deref() != Some(manifest.revision.as_str()) {
            return Err(format!(
                "{}: asset revision disagrees with manifest",
                entry.path
            ));
        }
        let file = base.join(&entry.path);
        let bytes = fs::read(&file).map_err(|e| format!("{}: cannot read: {e}", entry.path))?;
        let actual = crate::release::hash_bytes(&bytes);
        if actual != entry.sha256 {
            return Err(format!(
                "{}: expected {} got {}",
                entry.path, entry.sha256, actual
            ));
        }
        revision_rows.push((entry.path.clone(), entry.sha256.clone()));
    }
    revision_rows.sort();
    let revision_input = revision_rows
        .iter()
        .map(|(path, digest)| format!("{path}\t{digest}"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let revision = crate::release::hash_bytes(revision_input.as_bytes());
    if revision != manifest.revision {
        return Err(format!(
            "contracts/manifest.json revision mismatch: expected {}, computed {revision}",
            manifest.revision
        ));
    }
    let mut actual = std::collections::BTreeSet::new();
    collect_contract_assets(&base, &base, &mut actual)?;
    actual.remove("manifest.json");
    let declared_assets: std::collections::BTreeSet<_> = declared
        .iter()
        .filter(|path| path.as_str() != "manifest.json")
        .cloned()
        .collect();
    if actual != declared_assets {
        let unlisted: Vec<_> = actual.difference(&declared_assets).cloned().collect();
        let missing: Vec<_> = declared_assets.difference(&actual).cloned().collect();
        return Err(format!(
            "contract asset inventory mismatch; unlisted: {unlisted:?}; missing: {missing:?}"
        ));
    }
    Ok(())
}

fn collect_contract_assets(
    root: &Path,
    current: &Path,
    out: &mut std::collections::BTreeSet<String>,
) -> Result<(), String> {
    let entries = fs::read_dir(current).map_err(|e| {
        format!(
            "cannot enumerate contract assets at {}: {e}",
            current.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read contract asset entry: {e}"))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| format!("cannot inspect {}: {e}", path.display()))?;
        if kind.is_symlink() {
            return Err(format!(
                "contract asset symlink refused: {}",
                path.display()
            ));
        }
        if kind.is_dir() {
            collect_contract_assets(root, &path, out)?;
        } else if kind.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| format!("contract asset escaped root: {}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel);
        }
    }
    Ok(())
}

pub fn family_support_catalog() -> Result<Vec<PlatformFamilySupport>, String> {
    verify_manifest_digests()?;
    let path = contracts_dir().join("family-support.json");
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("cannot read family support catalog {}: {e}", path.display()))?;
    let doc: PlatformFamilyCatalog = serde_json::from_str(&text)
        .map_err(|e| format!("invalid family support catalog {}: {e}", path.display()))?;
    if doc.schema_version != 1 {
        return Err(format!("{} has unsupported schema_version", path.display()));
    }
    let mut names = std::collections::BTreeSet::new();
    for family in &doc.families {
        if !names.insert(family.family.as_str()) {
            return Err(format!(
                "{} duplicates family '{}'",
                path.display(),
                family.family
            ));
        }
        match family.status.as_str() {
            "supported" => {
                let schema = family.schema.as_deref().ok_or_else(|| {
                    format!(
                        "{} supported family '{}' has no schema",
                        path.display(),
                        family.family
                    )
                })?;
                if Path::new(schema).is_absolute()
                    || Path::new(schema)
                        .components()
                        .any(|part| !matches!(part, std::path::Component::Normal(_)))
                    || !contracts_dir().join(schema).is_file()
                {
                    return Err(format!(
                        "{} has unsafe or missing schema for '{}'",
                        path.display(),
                        family.family
                    ));
                }
                if family.version.as_deref().is_none_or(str::is_empty) {
                    return Err(format!(
                        "{} supported family '{}' has no version",
                        path.display(),
                        family.family
                    ));
                }
                let version = family.version.as_deref().expect("version checked above");
                if let Some(registered) = platform_family_version(&family.family) {
                    if version != registered {
                        return Err(format!(
                            "{} version for '{}' is {version}, expected {registered}",
                            path.display(),
                            family.family
                        ));
                    }
                    let major = version
                        .split('.')
                        .next()
                        .and_then(|value| value.parse::<u64>().ok())
                        .ok_or_else(|| {
                            format!("{} has invalid version '{version}'", path.display())
                        })?;
                    if !supported_majors(&family.family)?.contains(&major) {
                        return Err(format!(
                            "{} registry does not serve {} version {version}",
                            path.display(),
                            family.family
                        ));
                    }
                } else if family.family == crate::portfolio::share::MANIFEST_SCHEMA_FAMILY {
                    if version != crate::portfolio::share::MANIFEST_SCHEMA_VERSION {
                        return Err(format!(
                            "{} portfolio manifest version is {version}, expected {}",
                            path.display(),
                            crate::portfolio::share::MANIFEST_SCHEMA_VERSION
                        ));
                    }
                } else {
                    return Err(format!(
                        "{} supported family '{}' has no Forge producer or validator registration",
                        path.display(),
                        family.family
                    ));
                }
                if family.reason.is_some() {
                    return Err(format!(
                        "{} supported family '{}' cannot declare an unsupported reason",
                        path.display(),
                        family.family
                    ));
                }
            }
            "unsupported" => {
                if family.schema.is_some() || family.version.is_some() {
                    return Err(format!(
                        "{} unsupported family '{}' must not declare schema or version",
                        path.display(),
                        family.family
                    ));
                }
                if family.reason.as_deref().is_none_or(str::is_empty) {
                    return Err(format!(
                        "{} unsupported family '{}' has no reason",
                        path.display(),
                        family.family
                    ));
                }
            }
            other => {
                return Err(format!(
                    "{} family '{}' has invalid status '{other}'",
                    path.display(),
                    family.family
                ))
            }
        }
    }
    Ok(doc.families)
}

pub fn supported_families() -> Result<Vec<String>, String> {
    Ok(family_support_catalog()?
        .into_iter()
        .filter(|family| {
            family.status == "supported" && platform_family_version(&family.family).is_some()
        })
        .map(|family| family.family)
        .collect())
}

/// The wire version Forge emits for every family it can project onto,
/// registered as one reviewed table rather than interpolated at each call
/// site. The schema `$id` carries no version, so this table and the
/// envelope's `contract` discriminator are the only places a version is
/// expressed: a family bump is a table edit, and the
/// `registered_family_coverage` test proves no supported family is left
/// without a row.
pub const PLATFORM_FAMILY_VERSIONS: &[(&str, &str)] = &[
    ("platform.audit-event", "0.1.0"),
    ("platform.capability", "0.1.0"),
    ("platform.gate-result", "1.0.0"),
    ("platform.readiness", "0.1.0"),
    ("platform.release-evidence", "0.1.0"),
];

/// Fallback used only by [`build_envelope`] for a family nobody
/// registered. No Forge emission path reaches it — every family passed to
/// an `emit_*` helper is in `supported_families()` and therefore covered
/// by [`PLATFORM_FAMILY_VERSIONS`] — and the coverage test is what makes
/// that claim checkable rather than hopeful. It exists so the envelope
/// builder stays total instead of panicking on a caller mistake.
const UNREGISTERED_FAMILY_VERSION: &str = "0.1.0";

pub fn platform_family_version(family: &str) -> Option<&'static str> {
    PLATFORM_FAMILY_VERSIONS
        .iter()
        .find(|(name, _)| *name == family)
        .map(|(_, version)| *version)
}

pub fn family_schema_path(family: &str) -> Result<Option<String>, String> {
    Ok(family_support_catalog()?
        .into_iter()
        .find(|item| item.family == family && item.status == "supported")
        .map(|item| item.schema.expect("supported catalog row has schema")))
}

pub fn family_support(family: &str) -> Result<Option<PlatformFamilySupport>, String> {
    Ok(family_support_catalog()?
        .into_iter()
        .find(|item| item.family == family))
}

pub fn contract_discriminator(family: &str) -> Option<String> {
    platform_family_version(family).map(|version| format!("{family}/{version}"))
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

/// Project one persisted gate-evidence record onto the contract's root
/// `result`.
///
/// `platform.gate-result/1.0.0` gives Forge the value its own
/// `GateCheckState` exists for: `review_required` — "a human decision is
/// still open and no one has decided the outcome". `Unresolved` and
/// `Unknown` are precisely states Forge refused to classify, so reporting
/// them as `errored` ("a check could not be evaluated to a verdict")
/// asserted an evaluation failure that Forge never observed.
///
/// `Blocked` is decomposed rather than mapped flat because Forge's
/// aggregate is a superposition: `classify_aggregate` short-circuits on
/// the sibling's `blocked` bit before reading the status, so `Blocked`
/// covers both "a required check reported a failure" and "required reviews
/// are pending under `review_required_blocks`". The observed check rows
/// separate them. When no check row carries a failure, Forge has observed
/// no attempted failure at all while the gate did not proceed, so the one
/// claim 1.0.0 can make honestly is that no outcome was decided.
///
/// Every reachable record has an honest value, so the `Err` arm is
/// unreachable for this family today; the `Result` shape is kept so the
/// refusing mapping stays the interface, not so a future state has to be
/// invented first.
pub fn map_gate_result(
    evidence: &crate::gate::GateEvidence,
) -> Result<&'static str, ContractUnmappable> {
    use crate::gate::{GateAggregate, GateCheckState};
    match evidence.aggregate {
        GateAggregate::Passed => Ok("passed"),
        // A non-blocking failure is an attempted check that reported a
        // failure, which is what the 1.0.0 `failed` value states.
        GateAggregate::Failed => Ok("failed"),
        GateAggregate::Unknown => Ok("review_required"),
        GateAggregate::Blocked => {
            let failed = evidence
                .checks
                .iter()
                .any(|c| c.state == GateCheckState::Fail);
            if failed {
                Ok("failed")
            } else {
                // No row attributed a failure to the block, so the honest
                // claim is that the outcome is undecided — not that a
                // check failed and not that a check could not be
                // evaluated.
                Ok("review_required")
            }
        }
    }
}

/// Project one recorded check state onto the contract's
/// `checks[].status`. The 1.0.0 vocabulary is shared by `result` and
/// `checks[].status` and carries the same claim at both levels, so this
/// table and [`map_gate_result`] agree by construction.
///
/// `errored` is never produced: Forge does not run the checks, and its
/// gate plane never observes an adapter-level evaluation failure — an
/// unparseable document, a timeout or a spawn failure is
/// `GateOutcome::Unavailable`, which leaves no evidence and therefore
/// reaches no payload.
pub fn map_gate_check_state(
    state: crate::gate::GateCheckState,
) -> Result<&'static str, ContractUnmappable> {
    use crate::gate::GateCheckState;
    let mapped = match state {
        GateCheckState::Pass => "passed",
        GateCheckState::Fail => "failed",
        // `skipped` is "a check was proven not to apply; this is an
        // exemption", which is what NOT_APPLICABLE states. `Skip` keeps
        // the same rendering: the packaged runtime emits no SKIP status,
        // so the arm exists to keep the closed vocabulary closed.
        GateCheckState::Skip | GateCheckState::NotApplicable => "skipped",
        // No decision has been taken and no outcome was observed. The raw
        // status survives in the redacted, bounded `detail` string.
        GateCheckState::Unresolved => "review_required",
    };
    Ok(mapped)
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
    let contract = contract_discriminator(family)
        .unwrap_or_else(|| format!("{family}/{UNREGISTERED_FAMILY_VERSION}"));
    let generated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    serde_json::json!({
        "contract": contract,
        "generated_at": generated_at,
        "payload": payload,
    })
}

/// The majors a family still serves, read from the Forge-maintained registry
/// record. The registry is the authority the source publishes and
/// `verify_manifest_digests` pins; an unreadable, unparseable or
/// family-less registry is a refusal, never a skipped check.
fn supported_majors(family: &str) -> Result<Vec<u64>, String> {
    let path = contracts_dir().join("registry.json");
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let registry: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    let majors = registry
        .get("contracts")
        .and_then(|c| c.get(family))
        .and_then(|f| f.get("supported_major_versions"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            format!(
                "{} records no supported majors for '{family}'",
                path.display()
            )
        })?;
    let parsed: Vec<u64> = majors.iter().filter_map(|v| v.as_u64()).collect();
    if parsed.len() != majors.len() {
        return Err(format!(
            "{} declares a non-integer supported major for '{family}'",
            path.display()
        ));
    }
    Ok(parsed)
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
    let support = family_support(&family)?.ok_or_else(|| format!("unknown family '{family}'"))?;
    if support.status != "supported" {
        return Err(format!(
            "unsupported family '{family}': {}",
            support
                .reason
                .unwrap_or_else(|| "no support reason recorded".to_string())
        ));
    }
    let major = version
        .split('.')
        .next()
        .and_then(|m| m.parse::<u64>().ok())
        .ok_or_else(|| format!("contract '{contract}' has no parsable major version"))?;
    let majors = supported_majors(family)?;
    if !majors.contains(&major) {
        return Err(format!(
            "unsupported major for '{family}': {version} (the Forge registry serves {majors:?})"
        ));
    }
    doc.get("generated_at")
        .and_then(|v| v.as_str())
        .ok_or("missing generated_at")?;
    let payload = doc.get("payload").ok_or("missing payload")?;
    let subs = secret_field_substrings();
    if let Some(hit) = payload_contains_secret_field(payload, &subs) {
        return Err(format!("payload contains secret field '{hit}'"));
    }
    if let Some(schema_path) = family_schema_path(family)? {
        let schema_text = fs::read_to_string(contracts_dir().join(&schema_path))
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
    let result = map_gate_result(&evidence).map_err(|e| ForgeError::ContractInvalid {
        reason: e.to_string(),
    })?;
    let gate_id = format!("{}/{}", evidence.project_id, evidence.runtime);
    let mut checks = Vec::new();
    for c in &evidence.checks {
        let status = map_gate_check_state(c.state).map_err(|e| ForgeError::ContractInvalid {
            reason: e.to_string(),
        })?;
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
