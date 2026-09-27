//! Workspace Governance metadata emission (`workspace-metadata-emission`).
//!
//! Generation writes a sibling-schema-compatible `.project.json`
//! (`schema_version: 1`) derived purely from (project id, profile
//! descriptor): the governance profile comes from the descriptor's
//! explicit mapping, `verification.command` is the same native test
//! command the profile's readiness matrix uses, `evidence_status` starts
//! `planned`, and deployment stays non-deployable. Profiles whose
//! descriptor declares no mapping produce no file and a printed note —
//! a guessed governance profile is refused because the sibling validates
//! its own vocabulary.
//!
//! The declaration is covered by an ownership receipt under
//! `.forge/workspace/`: the receipt records the generated content hash,
//! so a user edit makes the bytes diverge from the record and upgrades
//! meet the standard ownership-conflict refusal instead of silently
//! overwriting. A declaration without a Forge receipt (hand-written or
//! created by the sibling) is foreign content Forge never writes.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::profile::ProfileDescriptor;

/// Sibling-compatible declaration path (Workspace Governance discovers
/// projects through this marker).
pub const METADATA_PATH: &str = ".project.json";

/// Forge-managed ownership receipt for the declaration.
pub const RECEIPT_PATH: &str = ".forge/workspace/project.json.receipt";

/// Sibling declaration schema version this emission targets.
pub const SCHEMA_VERSION: u8 = 1;

/// Honesty rule: Forge never writes evidence it did not observe, so a
/// fresh declaration always starts `planned`.
const EVIDENCE_STATUS: &str = "planned";

/// Declared lifecycle for freshly generated projects.
const LIFECYCLE: &str = "active";

#[derive(Serialize)]
struct Declaration<'a> {
    schema_version: u8,
    id: &'a str,
    kind: &'a str,
    profile: &'a str,
    lifecycle: &'a str,
    verification: Verification<'a>,
    /// Emitted only when the profile descriptor declares capabilities
    /// it really implements; otherwise the key is absent entirely
    /// (never an empty or speculative block).
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<std::collections::BTreeMap<&'a str, CapabilityEntry<'a>>>,
    deployment: Deployment,
}

#[derive(Serialize)]
struct Verification<'a> {
    command: &'a str,
    /// Emitted only when the profile descriptor declares a gate runtime;
    /// no supported profile declares one today.
    #[serde(skip_serializing_if = "Option::is_none")]
    gate_runtime: Option<&'a str>,
    evidence_status: &'a str,
}

#[derive(Serialize)]
struct Deployment {
    deployable: bool,
    jenkins_job: Option<String>,
    compose_file: Option<String>,
}

/// One generated capability entry. A fresh project has intent but no
/// observed evidence, so the state is always `declared`: no finding is
/// claimed and no evidence reference is invented.
#[derive(Serialize)]
struct CapabilityEntry<'a> {
    owner: &'a str,
    evidence_state: &'static str,
}

/// Deterministic declaration text for one project, or `None` when the
/// descriptor carries the no-mapping sentinel.
pub fn declaration_text(id: &str, descriptor: &ProfileDescriptor) -> Option<String> {
    let mapping = descriptor.workspace.as_ref()?;
    let capabilities = if mapping.capabilities.is_empty() {
        None
    } else {
        Some(
            mapping
                .capabilities
                .iter()
                .map(|cap| {
                    (
                        cap.name.as_str(),
                        CapabilityEntry {
                            owner: cap.owner.as_str(),
                            evidence_state: "declared",
                        },
                    )
                })
                .collect(),
        )
    };
    let document = Declaration {
        schema_version: SCHEMA_VERSION,
        id,
        kind: &mapping.kind,
        profile: &mapping.governance_profile,
        lifecycle: LIFECYCLE,
        verification: Verification {
            command: &descriptor.test_command,
            gate_runtime: mapping.gate_runtime.as_deref(),
            evidence_status: EVIDENCE_STATUS,
        },
        capabilities,
        // Non-deployable defaults: a real target is a project edit later.
        deployment: Deployment {
            deployable: false,
            jenkins_job: None,
            compose_file: None,
        },
    };
    let text =
        serde_json::to_string_pretty(&document).expect("declaration is serializable by shape");
    Some(format!("{text}\n"))
}

/// Receipt bytes recording the owned declaration's hash. Pure function of
/// the declaration content, like feature receipts.
pub fn receipt_text(declaration: &str) -> String {
    format!(
        "# Forge workspace metadata ownership record (Forge-managed; manual edits block upgrades).\n\
         file: {METADATA_PATH}\n\
         sha256: {}\n",
        sha256_hex(declaration.as_bytes())
    )
}

/// Hex SHA-256 used by the receipt contract.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Staged files generation adds for a mapped profile: the declaration
/// plus its ownership receipt. Unmapped profiles stage nothing.
pub fn staged_files(id: &str, descriptor: &ProfileDescriptor) -> Vec<(String, String)> {
    let Some(declaration) = declaration_text(id, descriptor) else {
        return Vec::new();
    };
    vec![
        (METADATA_PATH.to_string(), declaration.clone()),
        (RECEIPT_PATH.to_string(), receipt_text(&declaration)),
    ]
}

/// Printed note when a mapped output was requested but the profile
/// declares no governance mapping.
pub fn omission_note(profile: &str) -> String {
    format!(
        "workspace metadata: profile '{profile}' declares no governance mapping; \
         {METADATA_PATH} omitted (never guessed)"
    )
}

/// What an upgrade may do with an existing declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataAction {
    /// Nothing to manage: no declaration, no Forge receipt (foreign
    /// content), no mapping for the current profile, or content already
    /// matches what Forge would write.
    Nothing,
    /// The unedited generated declaration is stale for the current
    /// manifest and descriptor; the upgrade rewrites both files.
    Refresh {
        declaration: String,
        receipt: String,
    },
    /// The declaration differs from the recorded generation content
    /// (user edits) while an upgrade would change it: refuse with the
    /// ownership-conflict code and preserve the edited file.
    Conflict,
}

/// Decide the upgrade action for the declaration in `dir`, given the
/// live manifest id and profile descriptor. Read-only.
pub fn plan_action(dir: &Path, id: &str, descriptor: &ProfileDescriptor) -> MetadataAction {
    let Ok(current) = fs::read(dir.join(METADATA_PATH)) else {
        return MetadataAction::Nothing;
    };
    let Ok(receipt) = fs::read_to_string(dir.join(RECEIPT_PATH)) else {
        // A declaration without a Forge receipt is foreign (sibling
        // `init_project.py` or hand-written): Forge never rewrites it.
        return MetadataAction::Nothing;
    };
    let Some(expected) = declaration_text(id, descriptor) else {
        // Current profile has no mapping: nothing can be honestly emitted
        // as a replacement, so the existing file is left untouched.
        return MetadataAction::Nothing;
    };
    if current == expected.as_bytes() {
        return MetadataAction::Nothing;
    }
    let recorded = recorded_hash(&receipt);
    if recorded.as_deref() == Some(sha256_hex(&current).as_str()) {
        MetadataAction::Refresh {
            declaration: expected.clone(),
            receipt: receipt_text(&expected),
        }
    } else {
        // Edited (or the receipt itself drifted): the upgrade would
        // change a file the user owns edits on.
        MetadataAction::Conflict
    }
}

fn recorded_hash(receipt: &str) -> Option<String> {
    let line = receipt.lines().find_map(|l| l.strip_prefix("sha256: "))?;
    let hex = line.trim();
    let valid = (hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| hex.to_ascii_lowercase());
    valid
}

/// True when the declaration bytes in `dir` are exactly what Forge
/// generated: a Forge receipt exists and its recorded hash matches the
/// current file. A missing receipt (foreign content) or a hash mismatch
/// (user edits) means the claim is not Forge-authored, so a broken
/// reference in it warns rather than fails.
pub(crate) fn declaration_is_forge_authored(dir: &Path, current: &[u8]) -> bool {
    let Ok(receipt) = fs::read_to_string(dir.join(RECEIPT_PATH)) else {
        return false;
    };
    recorded_hash(&receipt).as_deref() == Some(sha256_hex(current).as_str())
}

/// Apply a [`MetadataAction::Refresh`]; other actions write nothing.
/// Returns the staged paths that changed.
pub fn apply_action(dir: &Path, action: &MetadataAction) -> Result<Vec<String>, std::io::Error> {
    let MetadataAction::Refresh {
        declaration,
        receipt,
    } = action
    else {
        return Ok(Vec::new());
    };
    fs::write(dir.join(METADATA_PATH), declaration)?;
    if let Some(parent) = dir.join(RECEIPT_PATH).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dir.join(RECEIPT_PATH), receipt)?;
    Ok(vec![METADATA_PATH.to_string(), RECEIPT_PATH.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{inspect_profile, WorkspaceMapping};
    use std::io::Write;

    fn request_descriptor(id: &str) -> ProfileDescriptor {
        inspect_profile(id).unwrap()
    }

    #[test]
    fn mapped_profile_emits_honest_declaration() {
        let text = declaration_text("demo-app", &request_descriptor("rust-web")).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["schema_version"], SCHEMA_VERSION);
        assert_eq!(value["id"], "demo-app");
        assert_eq!(value["kind"], "product");
        assert_eq!(value["profile"], "rust-product");
        assert_eq!(value["lifecycle"], "active");
        assert_eq!(value["verification"]["command"], "cargo test");
        assert_eq!(value["verification"]["evidence_status"], "planned");
        // Honesty: no gate runtime is declared, so the key is absent.
        assert!(
            value["verification"].get("gate_runtime").is_none(),
            "{text}"
        );
        assert_eq!(value["deployment"]["deployable"], false);
        assert!(value["deployment"]["jenkins_job"].is_null());
        assert!(value["deployment"]["compose_file"].is_null());
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn verification_command_equals_descriptor_test_command() {
        for id in [
            "aspnet-web",
            "flutter-app",
            "nextjs-web",
            "python-service",
            "react-web",
            "rust-web",
        ] {
            let descriptor = request_descriptor(id);
            let text = declaration_text(id, &descriptor).unwrap();
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(
                value["verification"]["command"].as_str().unwrap(),
                descriptor.test_command,
                "{id} must reuse the descriptor's native command"
            );
        }
    }

    #[test]
    fn declared_gate_runtime_is_emitted() {
        let mut descriptor = request_descriptor("rust-web");
        descriptor.workspace = Some(WorkspaceMapping {
            governance_profile: "rust-product".to_string(),
            kind: "product".to_string(),
            gate_runtime: Some("driftwatchdog".to_string()),
            capabilities: Vec::new(),
        });
        let text = declaration_text("demo", &descriptor).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["verification"]["gate_runtime"], "driftwatchdog");
    }

    #[test]
    fn descriptor_declared_capabilities_emit_as_declared_only() {
        use crate::profile::WorkspaceCapability;
        let mut descriptor = request_descriptor("rust-web");
        descriptor.workspace.as_mut().unwrap().capabilities = vec![
            WorkspaceCapability {
                name: "storage".to_string(),
                owner: "product".to_string(),
            },
            WorkspaceCapability {
                name: "jobs".to_string(),
                owner: "product".to_string(),
            },
        ];
        let text = declaration_text("demo", &descriptor).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let caps = value["capabilities"]
            .as_object()
            .expect("capabilities block");
        assert_eq!(caps.len(), 2, "{text}");
        assert_eq!(caps["storage"]["owner"], "product");
        assert_eq!(caps["storage"]["evidence_state"], "declared");
        assert!(caps["storage"].get("evidence_ref").is_none(), "{text}");
        assert_eq!(caps["jobs"]["evidence_state"], "declared");
    }

    #[test]
    fn descriptor_without_capabilities_emits_no_key() {
        for id in [
            "aspnet-web",
            "flutter-app",
            "nextjs-web",
            "python-service",
            "react-web",
            "rust-web",
        ] {
            let text = declaration_text(id, &request_descriptor(id)).unwrap();
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert!(
                value.get("capabilities").is_none(),
                "{id} must carry no speculative capabilities block: {text}"
            );
        }
    }

    #[test]
    fn unmapped_profile_emits_nothing_and_notes() {
        let mut descriptor = request_descriptor("rust-web");
        descriptor.workspace = None;
        assert!(declaration_text("demo", &descriptor).is_none());
        assert!(staged_files("demo", &descriptor).is_empty());
        let note = omission_note("rust-cli");
        assert!(note.contains("rust-cli"), "{note}");
        assert!(note.contains("no governance mapping"), "{note}");
        assert!(note.contains(".project.json"), "{note}");
    }

    #[test]
    fn emission_is_deterministic_and_host_free() {
        let descriptor = request_descriptor("nextjs-web");
        let a = declaration_text("store-app", &descriptor).unwrap();
        let b = declaration_text("store-app", &descriptor).unwrap();
        assert_eq!(a, b);
        // No machine-specific roots, host paths or timestamps.
        assert!(!a.contains('/'), "{a}");
        assert!(!a.contains(char::from(0)), "{a}");
        let now = chrono::Utc::now().to_rfc3339();
        assert!(!a.contains(&now[..8]), "{a}");
    }

    #[test]
    fn receipt_records_declaration_hash() {
        let descriptor = request_descriptor("python-service");
        let files = staged_files("svc", &descriptor);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, METADATA_PATH);
        assert_eq!(files[1].0, RECEIPT_PATH);
        let expected_hash = sha256_hex(files[0].1.as_bytes());
        assert!(files[1].1.contains(&expected_hash), "{}", files[1].1);
        assert_eq!(
            recorded_hash(&files[1].1).as_deref(),
            Some(expected_hash.as_str())
        );
    }

    fn write(dir: &Path, rel: &str, bytes: &[u8]) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(bytes).unwrap();
    }

    #[test]
    fn plan_action_nothing_without_declaration_or_receipt() {
        let tmp = tempfile::TempDir::new().unwrap();
        let descriptor = request_descriptor("rust-web");
        assert_eq!(
            plan_action(tmp.path(), "app", &descriptor),
            MetadataAction::Nothing
        );
        // Foreign declaration (no Forge receipt): never rewritten.
        let text = declaration_text("app", &descriptor).unwrap();
        write(tmp.path(), METADATA_PATH, text.as_bytes());
        assert_eq!(
            plan_action(tmp.path(), "app", &descriptor),
            MetadataAction::Nothing
        );
    }

    #[test]
    fn plan_action_refreshes_unedited_stale_content() {
        let tmp = tempfile::TempDir::new().unwrap();
        let descriptor = request_descriptor("rust-web");
        let text = declaration_text("app", &descriptor).unwrap();
        write(tmp.path(), METADATA_PATH, text.as_bytes());
        write(tmp.path(), RECEIPT_PATH, receipt_text(&text).as_bytes());
        // Current profile: nothing to do.
        assert_eq!(
            plan_action(tmp.path(), "app", &descriptor),
            MetadataAction::Nothing
        );
        // Manifest moved to another mapped profile: unedited content is
        // refreshed honestly from the new descriptor.
        let next = request_descriptor("aspnet-web");
        match plan_action(tmp.path(), "app", &next) {
            MetadataAction::Refresh {
                declaration,
                receipt,
            } => {
                assert!(declaration.contains("dotnet-product"), "{declaration}");
                assert_eq!(
                    recorded_hash(&receipt).as_deref(),
                    Some(sha256_hex(declaration.as_bytes()).as_str())
                );
            }
            other => panic!("expected refresh, got {other:?}"),
        }
    }

    #[test]
    fn plan_action_conflicts_on_user_edits() {
        let tmp = tempfile::TempDir::new().unwrap();
        let descriptor = request_descriptor("rust-web");
        let text = declaration_text("app", &descriptor).unwrap();
        write(tmp.path(), METADATA_PATH, text.as_bytes());
        write(tmp.path(), RECEIPT_PATH, receipt_text(&text).as_bytes());
        // User edits the declaration.
        let edited = text.replace("cargo test", "make it so");
        write(tmp.path(), METADATA_PATH, edited.as_bytes());
        // An upgrade would change the file: conflict, never overwrite.
        let next = request_descriptor("aspnet-web");
        assert_eq!(
            plan_action(tmp.path(), "app", &next),
            MetadataAction::Conflict
        );
        // Malformed receipts fail closed to a conflict, preserving the file.
        write(tmp.path(), RECEIPT_PATH, b"not a receipt\n");
        assert_eq!(
            plan_action(tmp.path(), "app", &descriptor),
            MetadataAction::Conflict
        );
    }

    #[test]
    fn apply_action_writes_refresh_only() {
        let tmp = tempfile::TempDir::new().unwrap();
        let descriptor = request_descriptor("rust-web");
        let text = declaration_text("app", &descriptor).unwrap();
        assert_eq!(
            apply_action(tmp.path(), &MetadataAction::Nothing).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            apply_action(tmp.path(), &MetadataAction::Conflict).unwrap(),
            Vec::<String>::new()
        );
        let action = MetadataAction::Refresh {
            declaration: text.clone(),
            receipt: receipt_text(&text),
        };
        let changed = apply_action(tmp.path(), &action).unwrap();
        assert_eq!(
            changed,
            vec![METADATA_PATH.to_string(), RECEIPT_PATH.to_string()]
        );
        assert_eq!(
            fs::read_to_string(tmp.path().join(METADATA_PATH)).unwrap(),
            text
        );
        // Conflict path preserves a user-edited file untouched.
        let edited = text.replace("cargo test", "mine");
        write(tmp.path(), METADATA_PATH, edited.as_bytes());
        apply_action(tmp.path(), &MetadataAction::Conflict).unwrap();
        assert_eq!(
            fs::read_to_string(tmp.path().join(METADATA_PATH)).unwrap(),
            edited
        );
    }
}
