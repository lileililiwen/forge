//! Declaration preflight for the Gate boundary (`gate-declaration-migration`).
//!
//! Forge owns the *validating* read of the declarations Driftwatchdog and
//! workspace-governance own. [`preflight_gate_declarations`] is the only
//! constructor of [`GateDeclarationContext`]: it checks `<project>/forge.yaml`
//! (absence is a valid empty floor, presence must parse), parses the whole
//! `.project.json` and the one Gate manifest Driftwatchdog selects, resolves
//! each floor row's confined implementation path, and computes the pack
//! expectation — all before any runtime spawn, evidence load or write.
//!
//! Nothing here executes a sibling, searches a parent or environment
//! variable, or writes a file. A malformed declaration returns its typed
//! error and leaves the invocation marker, prior evidence, the operation
//! journal and sibling history byte-identical.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::manifest::{
    DeclaredImplementationSource, ImplementationRoot, Manifest, VerificationMeta,
};
use crate::core::ForgeError;
use crate::governance::WORKSPACE_GOVERNANCE_PROVIDER_ID;

use super::model::{GateCheckState, ImplementationSource, PackExpectation, PackExpectationSource};

/// The Gate manifests Driftwatchdog selects from, in its own order. Only
/// the first existing regular file is read.
const GATE_MANIFEST_CANDIDATES: [&str; 3] =
    ["gate.toml", ".driftwatch/gate.toml", ".ai-gate/gate.yaml"];
/// The workspace-owned declaration Forge reads whole.
const WORKSPACE_DECLARATION: &str = ".project.json";
/// The canonical Forge manifest.
const FORGE_MANIFEST: &str = "forge.yaml";

/// One floor row's preflighted resolution. Only rows that declare a
/// root/path pair get an entry; a pathless floor row is read straight
/// from `verification`.
#[derive(Debug, Clone)]
enum DeclaredPathResolution {
    /// The declared candidate exists as a confined regular file.
    Present { stable_path: String },
    /// The declared root, candidate or governance selection is missing,
    /// disabled, wrong-provider or not a regular file. The row keeps
    /// exactly one stable placeholder path.
    Absent { stable_path: String },
}

/// The validated declaration floor one Gate operation runs against.
///
/// The fields are private on purpose: only [`preflight_gate_declarations`]
/// may build one, so no runtime path, reader or transport can act on an
/// unchecked declaration.
#[derive(Debug)]
pub struct GateDeclarationContext {
    verification: Option<VerificationMeta>,
    pack_expectation: PackExpectation,
    path_resolutions: BTreeMap<String, DeclaredPathResolution>,
}

impl GateDeclarationContext {
    /// The pack identity the current declarations expect.
    pub fn pack_expectation(&self) -> &PackExpectation {
        &self.pack_expectation
    }

    /// The declared verification floor, when `forge.yaml` declared one.
    pub fn verification(&self) -> Option<&VerificationMeta> {
        self.verification.as_ref()
    }

    /// Attribute one runtime row. This is the ordered algorithm of design
    /// 5.3 and performs no declaration or filesystem read: an exact,
    /// case-sensitive floor id wins, otherwise the row state and the exact
    /// structured runtime source decide. Free text never participates.
    pub fn resolve_implementation_source(
        &self,
        id: &str,
        state: GateCheckState,
        runtime_source: Option<&str>,
    ) -> (ImplementationSource, Option<String>) {
        if let Some(check) = self
            .verification
            .as_ref()
            .and_then(|verification| verification.checks.iter().find(|check| check.id == id))
        {
            match &check.source {
                DeclaredImplementationSource::Exempt => {
                    return (ImplementationSource::Exempt, None)
                }
                DeclaredImplementationSource::PackDefault => {
                    return (ImplementationSource::PackDefault, None)
                }
                DeclaredImplementationSource::Native | DeclaredImplementationSource::Bound => {
                    let declared = match check.source {
                        DeclaredImplementationSource::Native => ImplementationSource::Native,
                        _ => ImplementationSource::Bound,
                    };
                    // `native`/`bound` without a pair: in-process or
                    // command implementations may have no single file.
                    if check.path.is_none() {
                        return (declared, None);
                    }
                    return match self.path_resolutions.get(id) {
                        Some(DeclaredPathResolution::Present { stable_path }) => {
                            (declared, Some(stable_path.clone()))
                        }
                        Some(DeclaredPathResolution::Absent { stable_path }) => {
                            (ImplementationSource::Absent, Some(stable_path.clone()))
                        }
                        // Unreachable: every root/path row is resolved
                        // during preflight. Report the declared word
                        // rather than inventing an absent row.
                        None => (declared, None),
                    };
                }
            }
        }
        match state {
            GateCheckState::Unbound => (ImplementationSource::Unbound, None),
            GateCheckState::NotApplicable => (ImplementationSource::Exempt, None),
            _ => match runtime_source {
                Some("adapter:project-runtime") => (ImplementationSource::Bound, None),
                Some("driftwatchdog") => (ImplementationSource::Native, None),
                _ => (ImplementationSource::Undeclared, None),
            },
        }
    }
}

/// The workspace declaration's pack identity, once the whole
/// `.project.json` has been read as JSON.
struct WorkspaceDeclaration {
    identity: Option<String>,
}

/// The one Gate manifest Driftwatchdog selects, with the pack identity it
/// declares.
struct SelectedGateManifest {
    relative: &'static str,
    identity: Option<String>,
}

/// Validate every current declaration for `project_root` and resolve the
/// floor's attribution. This is the only constructor of
/// [`GateDeclarationContext`] and it has no side effect.
pub fn preflight_gate_declarations(
    project_root: &Path,
) -> Result<GateDeclarationContext, ForgeError> {
    // 1 + 2: an absent `forge.yaml` is a valid empty floor; a present one
    // must load, and its `manifest-invalid` propagates rather than being
    // read as absence.
    let verification = load_verification_floor(project_root)?;
    // 3: every existing relevant declaration is parsed even when a
    // higher-precedence tier already answers.
    let workspace = read_workspace_declaration(project_root)?;
    let gate_manifest = select_gate_manifest(project_root)?;
    // 4: confinement per project-root row, then the stored governance
    // selection for governance-root rows only.
    let path_resolutions = resolve_floor_paths(project_root, verification.as_ref())?;
    // 5: the expectation comes from the already parsed documents.
    let pack_expectation = resolve_pack_expectation(
        verification.as_ref(),
        workspace.as_ref(),
        gate_manifest.as_ref(),
    );
    Ok(GateDeclarationContext {
        verification,
        pack_expectation,
        path_resolutions,
    })
}

/// Load the presence-preserving verification floor. `Manifest::parse`
/// owns the schema (explicit `null`/scalar/sequence and a mapping without
/// `checks` are `manifest-invalid`, not absence).
fn load_verification_floor(project_root: &Path) -> Result<Option<VerificationMeta>, ForgeError> {
    let path = project_root.join(FORGE_MANIFEST);
    if !path.is_file() {
        return Ok(None);
    }
    let (manifest, _path) = Manifest::load_from_dir(project_root, Some(Path::new(FORGE_MANIFEST)))?;
    Ok(manifest.verification)
}

/// Read the whole `.project.json` as JSON and validate the one field
/// Forge consumes. Malformed JSON, a non-object document, a non-mapping
/// `verification` and a non-string `verification.rule_pack` are
/// `gate-invalid` naming the relative path and the exact field.
fn read_workspace_declaration(
    project_root: &Path,
) -> Result<Option<WorkspaceDeclaration>, ForgeError> {
    let path = project_root.join(WORKSPACE_DECLARATION);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| {
        gate_invalid(
            WORKSPACE_DECLARATION,
            "document",
            &format!("cannot read: {err}"),
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
        gate_invalid(
            WORKSPACE_DECLARATION,
            "document",
            &format!("not valid JSON: {err}"),
        )
    })?;
    let object = value
        .as_object()
        .ok_or_else(|| gate_invalid(WORKSPACE_DECLARATION, "document", "must be a JSON object"))?;
    let identity = match object.get("verification") {
        None => None,
        Some(declaration) => {
            let verification = declaration.as_object().ok_or_else(|| {
                gate_invalid(WORKSPACE_DECLARATION, "verification", "must be a mapping")
            })?;
            match verification.get("rule_pack") {
                None => None,
                Some(pack) => {
                    let pack = pack.as_str().ok_or_else(|| {
                        gate_invalid(
                            WORKSPACE_DECLARATION,
                            "verification.rule_pack",
                            "must be a string",
                        )
                    })?;
                    non_empty(pack)
                }
            }
        }
    };
    Ok(Some(WorkspaceDeclaration { identity }))
}

/// Read the first existing Gate manifest in Driftwatchdog order and
/// validate the one field Forge consumes: root `rule_pack_version` for
/// TOML, root `rule_pack` for YAML. A document that does not parse, is
/// not a mapping, or carries a wrong-typed pack field is `gate-invalid`
/// naming the relative path and the exact field.
fn select_gate_manifest(project_root: &Path) -> Result<Option<SelectedGateManifest>, ForgeError> {
    for relative in GATE_MANIFEST_CANDIDATES {
        let path = project_root.join(relative);
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| gate_invalid(relative, "document", &format!("cannot read: {err}")))?;
        let identity = if relative.ends_with(".toml") {
            let value: toml::Value = toml::from_str(&text).map_err(|err| {
                gate_invalid(relative, "document", &format!("not valid TOML: {err}"))
            })?;
            match value
                .as_table()
                .and_then(|table| table.get("rule_pack_version"))
            {
                None => None,
                Some(toml::Value::String(pack)) => non_empty(pack),
                Some(_) => {
                    return Err(gate_invalid(
                        relative,
                        "rule_pack_version",
                        "must be a string",
                    ))
                }
            }
        } else {
            let value: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|err| {
                gate_invalid(relative, "document", &format!("not valid YAML: {err}"))
            })?;
            let mapping = value
                .as_mapping()
                .ok_or_else(|| gate_invalid(relative, "document", "must be a mapping"))?;
            match mapping.get(serde_yaml::Value::String("rule_pack".to_string())) {
                None => None,
                Some(serde_yaml::Value::String(pack)) => non_empty(pack),
                Some(_) => return Err(gate_invalid(relative, "rule_pack", "must be a string")),
            }
        };
        return Ok(Some(SelectedGateManifest { relative, identity }));
    }
    Ok(None)
}

/// Resolve every floor row that declares a root/path pair. Project-root
/// candidates are confined to the canonical project root (a symlink
/// escape is `manifest-invalid`); governance-root rows consult the stored
/// provider selection, loaded at most once and only when such a row
/// exists.
fn resolve_floor_paths(
    project_root: &Path,
    verification: Option<&VerificationMeta>,
) -> Result<BTreeMap<String, DeclaredPathResolution>, ForgeError> {
    let mut resolutions = BTreeMap::new();
    let Some(verification) = verification else {
        return Ok(resolutions);
    };

    for (index, check) in verification.checks.iter().enumerate() {
        let Some(path) = check.path.as_deref() else {
            continue;
        };
        if !matches!(check.root.as_ref(), Some(ImplementationRoot::Project)) {
            continue;
        }
        let stable_path = format!("<project>/{path}");
        let resolution = if confined_project_file(project_root, path, index)? {
            DeclaredPathResolution::Present { stable_path }
        } else {
            DeclaredPathResolution::Absent { stable_path }
        };
        resolutions.insert(check.id.clone(), resolution);
    }

    // Second pass: resolve governance-workspace floor paths using the stored
    // workspace root. Apply the same canonical containment check used for
    // project-root candidates so a governance candidate that escapes its
    // declared root through a symlink returns `manifest-invalid`.
    let mut governance_root: Option<Option<PathBuf>> = None;
    for (index, check) in verification.checks.iter().enumerate() {
        let Some(path) = check.path.as_deref() else {
            continue;
        };
        if !matches!(
            check.root.as_ref(),
            Some(ImplementationRoot::GovernanceWorkspace)
        ) {
            continue;
        }
        let stable_path = format!("<governance-workspace>/{path}");
        let root = match &governance_root {
            Some(root) => root.clone(),
            None => {
                let resolved = stored_governance_root(project_root)?;
                governance_root = Some(resolved.clone());
                resolved
            }
        };
        let prefix_map = project_root.join(FORGE_MANIFEST).display().to_string();
        let present = match &root {
            Some(root) => {
                confined_candidate_file(&root.join(path), root.as_path(), index, &prefix_map)
            }
            None => Ok(false),
        };
        let resolution = match present {
            Ok(true) => DeclaredPathResolution::Present { stable_path },
            _ => DeclaredPathResolution::Absent { stable_path },
        };
        resolutions.insert(check.id.clone(), resolution);
    }

    Ok(resolutions)
}

/// Whether an existing declared project-root candidate is a confined
/// regular file. A candidate whose canonical path leaves its canonical
/// declared root through a symlink is `manifest-invalid` naming the exact
/// `verification.checks[n].path`, never `absent`.
fn confined_project_file(
    project_root: &Path,
    relative: &str,
    index: usize,
) -> Result<bool, ForgeError> {
    let candidate = project_root.join(relative);
    if !candidate.exists() {
        return Ok(false);
    }
    let manifest = project_root.join(FORGE_MANIFEST).display().to_string();
    let canonical = candidate
        .canonicalize()
        .map_err(|err| ForgeError::ManifestInvalid {
            path: manifest.clone(),
            reason: format!(
                "verification.checks[{index}].path cannot be resolved: {relative}: {err}"
            ),
        })?;
    let root = project_root
        .canonicalize()
        .map_err(|err| ForgeError::ManifestInvalid {
            path: manifest.clone(),
            reason: format!(
                "verification.checks[{index}].path declared root cannot be resolved: {err}"
            ),
        })?;
    if !canonical.starts_with(&root) {
        return Err(ForgeError::ManifestInvalid {
            path: manifest,
            reason: format!(
                "verification.checks[{index}].path escapes its declared project root: {relative}",
            ),
        });
    }
    Ok(candidate.is_file())
}

/// Generic confined-candidate check: given an absolute candidate path and its
/// declared root, verifies the canonical candidate stays within the declared
/// root through a symlink-safe containment test. Returns true if it is a
/// regular file, false otherwise, or manifest-invalid when the candidate
/// escapes the declared root. The index identifies the check row and
/// path_name names the declaring manifest for error messages.
fn confined_candidate_file(
    candidate: &Path,
    root: &Path,
    index: usize,
    path_name: &str,
) -> Result<bool, ForgeError> {
    if !candidate.exists() {
        return Ok(false);
    }
    let manifest = path_name.to_string();
    let canonical = candidate
        .canonicalize()
        .map_err(|err| ForgeError::ManifestInvalid {
            path: manifest.clone(),
            reason: format!(
                "verification.checks[{index}] cannot be resolved: {}: {err}",
                candidate.display()
            ),
        })?;
    let declared_root = root
        .canonicalize()
        .map_err(|err| ForgeError::ManifestInvalid {
            path: manifest.clone(),
            reason: format!("cannot resolve declared root {}: {err}", root.display()),
        })?;
    if !canonical.starts_with(&declared_root) {
        return Err(ForgeError::ManifestInvalid {
            path: manifest,
            reason: format!(
                "verification.checks[{index}] escapes its declared workspace root: {}",
                candidate.display()
            ),
        });
    }
    Ok(candidate.is_file())
}

/// The absolute `workspace_root` of the stored, enabled
/// `workspace-governance` selection. Only that stored field answers: there
/// is no environment, parent, adapter-path or sibling fallback. A malformed
/// stored config propagates `governance-invalid`; an unset or unusable
/// selection is simply no root.
fn stored_governance_root(project_root: &Path) -> Result<Option<PathBuf>, ForgeError> {
    let config = crate::governance::load_config(project_root)?;
    Ok(config
        .provider
        .as_ref()
        .filter(|provider| {
            provider.provider == WORKSPACE_GOVERNANCE_PROVIDER_ID && provider.enabled
        })
        .and_then(|provider| provider.workspace_root.as_deref())
        .filter(|root| !root.trim().is_empty())
        .map(PathBuf::from))
}

/// The first non-empty pack identity, in the required precedence, with
/// the tier that answered and its relative provenance file.
fn resolve_pack_expectation(
    verification: Option<&VerificationMeta>,
    workspace: Option<&WorkspaceDeclaration>,
    gate_manifest: Option<&SelectedGateManifest>,
) -> PackExpectation {
    if let Some(identity) = verification
        .and_then(|verification| verification.rule_pack.as_deref())
        .and_then(non_empty)
    {
        return PackExpectation {
            identity: Some(identity),
            source: PackExpectationSource::ForgeManifest,
            provenance: Some(FORGE_MANIFEST.to_string()),
        };
    }
    if let Some(identity) = workspace.and_then(|workspace| workspace.identity.clone()) {
        return PackExpectation {
            identity: Some(identity),
            source: PackExpectationSource::WorkspaceDeclaration,
            provenance: Some(WORKSPACE_DECLARATION.to_string()),
        };
    }
    if let Some(manifest) = gate_manifest {
        if let Some(identity) = manifest.identity.clone() {
            return PackExpectation {
                identity: Some(identity),
                source: PackExpectationSource::GateManifest,
                provenance: Some(manifest.relative.to_string()),
            };
        }
    }
    PackExpectation {
        identity: None,
        source: PackExpectationSource::Undeclared,
        provenance: None,
    }
}

/// A declaration field answers only with a non-empty value; a
/// whitespace-only declaration falls through to the next tier.
fn non_empty(raw: &str) -> Option<String> {
    if raw.trim().is_empty() {
        None
    } else {
        Some(raw.to_string())
    }
}

fn gate_invalid(relative: &str, field: &str, detail: &str) -> ForgeError {
    ForgeError::GateInvalid {
        reason: format!("invalid {relative}: {field} {detail}"),
    }
}
