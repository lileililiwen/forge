//! The workspace boundary declaration, read from the inspected project.
//!
//! `workspace-governance` owns the format (`schemas/project.schema.json`);
//! this module only reads it: the optional `interfaces[]`, `capabilities`
//! and `integrations` blocks of a project's `.project.json`. A declaration
//! is readable or refused — never half-read, and never treated as absent —
//! and nothing here fetches or writes anything, so the no-sibling
//! invariant holds.
//!
//! Extracted from `src/plugins/mod.rs` to keep that file under the
//! source-file-size cap.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::core::ForgeError;
use crate::publish::providers::ProviderEntry;

use super::{builtin_descriptor, PluginDescriptor, PluginKind, CAPABILITIES, INTERFACE_MECHANISMS};

/// One declared boundary, validated against the workspace schema's own
/// rules (`workspace-governance` `schemas/project.schema.json`). Forge
/// reads this block; it adds no field to the format, and it never writes
/// a declaration into another project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceEntry {
    pub id: String,
    pub provides: bool,
    pub consumes: bool,
    pub provider: Option<String>,
    pub mechanism: String,
    pub entry: String,
    pub contract: Option<String>,
    pub digest: Option<String>,
    pub evidence_ref: Option<String>,
}

/// The project's own statement about one plugin id.
///
/// `kind` and `capabilities` are `None`/empty when the declaration names
/// the id without stating them: an unstated field is not a claim, and
/// the ranks below fill it rather than being overridden by it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclarationClaim {
    pub kind: Option<PluginKind>,
    pub capabilities: BTreeSet<String>,
}

impl DeclarationClaim {
    /// The descriptor this claim contributes, with the kind and
    /// capabilities it does not state filled from the builtin table
    /// (manifest first, then the hardcoded map).
    ///
    /// Filling is exactly what keeps a declared row behaviourally equal
    /// to the builtin row for the same id: a declaration that names a
    /// boundary it does not describe changes the row's *provenance*, not
    /// its kind or capabilities. The description is **not** filled: the
    /// builtin table's own text says "builtin …", which would be a
    /// misattribution on a row the declaration answered, so a declared
    /// row carries no description rather than the builtin's.
    pub(super) fn filled(&self, id: &str) -> PluginDescriptor {
        let fallback = builtin_descriptor(id);
        PluginDescriptor {
            kind: self
                .kind
                .map(|kind| kind.as_str().to_string())
                .or_else(|| fallback.as_ref().and_then(|d| d.kind.clone())),
            capabilities: if self.capabilities.is_empty() {
                fallback
                    .as_ref()
                    .map(|d| d.capabilities.clone())
                    .unwrap_or_default()
            } else {
                self.capabilities.iter().cloned().collect()
            },
            description: None,
        }
    }
}

/// The inspected project's boundary declaration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Declaration {
    /// The file the declaration was read from, when one was present.
    pub path: Option<PathBuf>,
    /// Validated `interfaces[]`, in declaration order.
    pub interfaces: Vec<InterfaceEntry>,
    /// What the declaration states about each plugin id it names.
    pub claims: BTreeMap<String, DeclarationClaim>,
}

/// The file name a workspace declaration lives in, at the project root.
pub const DECLARATION_FILE: &str = ".project.json";

/// The keys the workspace schema accepts on an `interfaces[]` entry. An
/// entry carrying anything else is refused rather than partially read.
const INTERFACE_KEYS: [&str; 10] = [
    "id",
    "provides",
    "consumes",
    "provider",
    "mechanism",
    "entry",
    "contract",
    "digest",
    "required",
    "evidence_ref",
];

fn refuse(reason: String) -> ForgeError {
    ForgeError::PluginBoundaryRefused { reason }
}

/// Read and validate the declaration at `<project_dir>/.project.json`.
///
/// An absent file means "nothing declared": `plugins list` on a project
/// with no declaration behaves exactly as it did before this source
/// existed. A present but unparseable file is refused naming the file and
/// the parse error, and a malformed `interfaces` block is refused field
/// by field, so a boundary that cannot be read is never treated as a
/// boundary that was not declared.
///
/// The `capabilities` and `integrations` blocks are read defensively:
/// their shapes are the audit's business, and a shape Forge cannot read
/// simply names no plugin id here.
pub fn read_declaration(project_dir: &Path) -> Result<Declaration, ForgeError> {
    let path = project_dir.join(DECLARATION_FILE);
    if !path.is_file() {
        return Ok(Declaration::default());
    }
    let text = std::fs::read_to_string(&path).map_err(|error| {
        refuse(format!(
            "declaration-invalid: {} cannot be read: {error}",
            path.display()
        ))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
        refuse(format!(
            "declaration-invalid: {} cannot be parsed: {error}",
            path.display()
        ))
    })?;
    let interfaces = match value.get("interfaces") {
        None => Vec::new(),
        Some(raw) => validate_interfaces(raw, &path)?,
    };
    Ok(Declaration {
        claims: declaration_claims(&value, &interfaces),
        interfaces,
        path: Some(path),
    })
}

/// Validate every `interfaces[]` entry, refusing the first problem with
/// the field that carries it. An empty array is valid and silent.
fn validate_interfaces(
    raw: &serde_json::Value,
    path: &Path,
) -> Result<Vec<InterfaceEntry>, ForgeError> {
    let items = raw.as_array().ok_or_else(|| {
        refuse(format!(
            "declaration-invalid: {} `interfaces` must be an array of interface declarations",
            path.display()
        ))
    })?;
    let mut entries = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        entries.push(validate_interface(item, index, path)?);
    }
    Ok(entries)
}

/// Validate one `interfaces[]` entry, mirroring the workspace schema's
/// rules: the id grammar, the exclusive side flags, the entry shape, the
/// provider a consumption names, the closed mechanism set, the versioned
/// contract every mechanism except `sibling` names, and the digest a
/// `consume`/`observe`/`sibling` edge is pinned by.
fn validate_interface(
    item: &serde_json::Value,
    index: usize,
    path: &Path,
) -> Result<InterfaceEntry, ForgeError> {
    let shape = |problem: String| {
        refuse(format!(
            "declaration-invalid: {}: {problem}",
            path.display()
        ))
    };
    let object = item
        .as_object()
        .ok_or_else(|| shape(format!("interfaces[{index}] must be an object")))?;
    let id = object.get("id").and_then(|v| v.as_str()).unwrap_or("");
    // The field prefix names the interface once its id is lawful, and the
    // index otherwise, so a refusal always names something the operator
    // can find in the file.
    let where_ = if crate::core::validate_project_id(id).is_ok() {
        format!("interface `{id}`")
    } else {
        format!("interfaces[{index}]")
    };
    if crate::core::validate_project_id(id).is_err() {
        return Err(shape(format!(
            "{where_}.id must be a kebab-case identifier"
        )));
    }
    let unknown: Vec<&str> = object
        .keys()
        .filter(|key| !INTERFACE_KEYS.contains(&key.as_str()))
        .map(|key| key.as_str())
        .collect();
    if !unknown.is_empty() {
        return Err(shape(format!(
            "{where_} has unknown key(s): {}",
            unknown.join(", ")
        )));
    }
    let provides = object.get("provides").and_then(|v| v.as_bool());
    let consumes = object.get("consumes").and_then(|v| v.as_bool());
    if provides == Some(true) && consumes == Some(true) {
        return Err(shape(format!(
            "{where_} must declare exactly one of provides/consumes as true"
        )));
    }
    if provides != Some(true) && consumes != Some(true) {
        return Err(shape(format!(
            "{where_} must declare exactly one of provides/consumes as true"
        )));
    }
    let mechanism = object
        .get("mechanism")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if mechanism.is_empty() {
        return Err(shape(format!("{where_}.mechanism is required")));
    }
    if !INTERFACE_MECHANISMS.contains(&mechanism) {
        return Err(refuse(format!(
            "mechanism-unknown: {where_}.mechanism is `{mechanism}`; accepted mechanisms: {}",
            INTERFACE_MECHANISMS.join(", ")
        )));
    }
    let entry = object.get("entry").and_then(|v| v.as_str()).unwrap_or("");
    if entry.is_empty() {
        return Err(shape(format!("{where_}.entry is required")));
    }
    if entry.starts_with('/') {
        return Err(shape(format!(
            "{where_}.entry must not be an absolute path"
        )));
    }
    if mechanism == "observe" && has_path_escape(entry) {
        return Err(shape(format!(
            "{where_}.entry must be a repository-relative document path; `..` is refused and never normalized"
        )));
    }
    let provider = object.get("provider").and_then(|v| v.as_str());
    if consumes == Some(true) && crate::core::validate_project_id(provider.unwrap_or("")).is_err() {
        return Err(shape(format!(
            "{where_}.provider must be the kebab-case id of the project providing it"
        )));
    }
    let contract = object.get("contract").and_then(|v| v.as_str());
    if let Some(value) = contract {
        if !is_versioned_contract(value) {
            return Err(refuse(format!(
                "contract-unversioned: {where_}.contract is `{value}`; write \"<name>/<major>.<minor>.<patch>\""
            )));
        }
    } else if mechanism != "sibling" {
        return Err(refuse(format!(
            "contract-unversioned: {where_}.contract is required for mechanism `{mechanism}`; write \"<name>/<major>.<minor>.<patch>\" and never invent a version"
        )));
    }
    let digest = object.get("digest").and_then(|v| v.as_str());
    if let Some(value) = digest {
        if !is_interface_digest(value) {
            return Err(shape(format!(
                "{where_}.digest must be a sha256 digest of the form sha256:<64 hex>"
            )));
        }
    } else if mechanism != "invoke" {
        return Err(refuse(format!(
            "digest-missing: {where_}.digest is required for mechanism `{mechanism}`; a retained copy or an observed document is pinned by the sha256 of its bytes"
        )));
    }
    let evidence_ref = object.get("evidence_ref").and_then(|v| v.as_str());
    if let Some(value) = evidence_ref {
        if value.is_empty() || value.starts_with('/') || has_path_escape(value) {
            return Err(shape(format!(
                "{where_}.evidence_ref must be a repository-relative path"
            )));
        }
    }
    Ok(InterfaceEntry {
        id: id.to_string(),
        provides: provides == Some(true),
        consumes: consumes == Some(true),
        provider: provider.map(|value| value.to_string()),
        mechanism: mechanism.to_string(),
        entry: entry.to_string(),
        contract: contract.map(|value| value.to_string()),
        digest: digest.map(|value| value.to_string()),
        evidence_ref: evidence_ref.map(|value| value.to_string()),
    })
}

/// A contract identifier with its version:
/// `"<name>/<major>.<minor>.<patch>"`, the schema's own rule.
fn is_versioned_contract(value: &str) -> bool {
    let Some((name, version)) = value.rsplit_once('/') else {
        return false;
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '@' | '/' | '-'))
    {
        return false;
    }
    let mut parts = version.split('.');
    let digits = |part: Option<&str>| {
        part.map(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(false)
    };
    digits(parts.next()) && digits(parts.next()) && digits(parts.next()) && parts.next().is_none()
}

/// A digest is `sha256:<64 hex>`, the schema's own form.
fn is_interface_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .map(|hex| {
            hex.len() == 64
                && hex
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        })
        .unwrap_or(false)
}

/// Whether a declared path escapes its repository root. The workspace
/// schema refuses an escaping path and never normalizes it into a
/// portable form; neither does Forge.
fn has_path_escape(value: &str) -> bool {
    value.split('/').any(|segment| segment == "..")
}

/// What the declaration states about every plugin id it names.
///
/// Three blocks name a boundary's other side, and all three are read:
/// an `interfaces[]` entry names its own id and, when it is a
/// consumption, the `provider` it consumes from; a `capabilities` entry
/// names its `owner`; an `integrations[]` entry names its `id`. The
/// closed capability vocabulary is the plugin vocabulary, so a
/// `capabilities` name outside it is not a plugin capability claim and
/// never becomes one — the workspace's own capability names (`identity`,
/// `storage`, …) stay their own vocabulary.
fn declaration_claims(
    value: &serde_json::Value,
    interfaces: &[InterfaceEntry],
) -> BTreeMap<String, DeclarationClaim> {
    let mut claims: BTreeMap<String, DeclarationClaim> = BTreeMap::new();
    for entry in interfaces {
        claims.entry(entry.id.clone()).or_default();
        if entry.consumes {
            if let Some(provider) = &entry.provider {
                claims.entry(provider.clone()).or_default();
            }
        }
    }
    if let Some(declared) = value.get("capabilities").and_then(|v| v.as_object()) {
        for (name, record) in declared {
            if !CAPABILITIES.contains(&name.as_str()) {
                continue;
            }
            let Some(owner) = record.get("owner").and_then(|v| v.as_str()) else {
                continue;
            };
            let claim = claims.entry(owner.to_string()).or_default();
            if let Some(kind) = PluginKind::parse(name) {
                if claim.kind.is_none() {
                    claim.kind = Some(kind);
                }
            }
            claim.capabilities.insert(name.clone());
        }
    }
    if let Some(integrations) = value.get("integrations").and_then(|v| v.as_array()) {
        for record in integrations {
            if let Some(id) = record.get("id").and_then(|v| v.as_str()) {
                claims.entry(id.to_string()).or_default();
            }
        }
    }
    claims
}

/// Refuse a configured descriptor that contradicts what the project
/// declares for the same id.
///
/// An operator descriptor is deliberate local configuration and a
/// provider declaration is a deliberate statement; when they disagree,
/// one of them is wrong and only a human can say which, so the registry
/// chooses neither. Only *stated* values are compared: an unstated field
/// is filled from the ranks below, never conflicted with.
pub(super) fn check_descriptor_against_declaration(
    entry: &ProviderEntry,
    descriptor: &PluginDescriptor,
    claim: &DeclarationClaim,
) -> Result<(), ForgeError> {
    let mut conflicts = Vec::new();
    if let (Some(declared_kind), Some(configured_kind)) = (
        claim.kind,
        descriptor.kind.as_deref().and_then(PluginKind::parse),
    ) {
        if declared_kind != configured_kind {
            conflicts.push(format!(
                "kind: declared `{}` by the declaration, configured `{}` by the descriptor",
                declared_kind.as_str(),
                configured_kind.as_str()
            ));
        }
    }
    let configured: BTreeSet<String> = descriptor
        .capabilities
        .iter()
        .filter(|capability| CAPABILITIES.contains(&capability.as_str()))
        .cloned()
        .collect();
    if !claim.capabilities.is_empty() && !configured.is_empty() && claim.capabilities != configured
    {
        conflicts.push(format!(
            "capabilities: declared [{}] by the declaration, configured [{}] by the descriptor",
            claim
                .capabilities
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            configured.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    if conflicts.is_empty() {
        return Ok(());
    }
    Err(refuse(format!(
        "definition-conflict: plugin `{}` is described twice and the two disagree ({}); one of them is wrong and Forge will not choose, so no row is rendered for this id",
        entry.id,
        conflicts.join("; ")
    )))
}
