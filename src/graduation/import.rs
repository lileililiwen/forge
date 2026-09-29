//! How a validated artifact becomes a project.
//!
//! The read-only half is [`build_proposal`]: resolve the profile, the
//! identity and the destination, and refuse a conflict before anything
//! is created. The writing half is [`adopt_graduation`]: write the
//! minimal manifest and the receipt, register, and roll the files back
//! if registration does not complete.
//!
//! Nothing here scaffolds, deploys, publishes or approves a gate. The
//! receipt is the projection that survives: the *mapped brief* and the
//! *allowlisted source provenance*, plus the evidence **count**. The
//! original artifact bytes and every evidence excerpt stay in the
//! preview and never reach disk.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use super::{
    clean_actor, GraduationBrief, GraduationImport, GraduationSource, GRADUATION_CONTRACT_VERSION,
    GRADUATION_DIR,
};
use crate::core::{validate_project_id, ForgeError};
use crate::profile::inspect_profile;
use crate::registry::{ProjectRecord, Registry};

/// A validated, read-only destination and identity resolution. The
/// project exists only once [`adopt_graduation`] returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationProposal {
    pub id: String,
    pub name: String,
    pub profile: String,
    /// Canonical absolute destination path.
    pub destination: String,
}

/// Everything the operator is shown before a write. `proposal` is
/// `None` for the bare `preview` surface and `Some` for an `import`
/// dry run, which is the one difference between them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationPreview {
    pub artifact: String,
    pub import: GraduationImport,
    pub proposal: Option<GraduationProposal>,
}

/// The outcome of a confirmed import.
#[derive(Debug, Clone)]
pub struct GraduationAdoption {
    pub record: ProjectRecord,
    pub receipt_path: String,
}

/// The `experiment` block of the receipt: the summary, the validated
/// flag and the evidence **count** — never an excerpt.
#[derive(Debug, Serialize)]
struct GraduationReceiptExperiment {
    summary: String,
    validated: bool,
    evidence_count: usize,
}

/// The exact receipt shape. Every field here is on the persistence
/// allowlist; the closed struct is what proves no excerpt or extra key
/// can reach disk.
#[derive(Debug, Serialize)]
struct GraduationReceipt {
    contract: String,
    project_id: String,
    source: GraduationSource,
    brief: GraduationBrief,
    experiment: GraduationReceiptExperiment,
    imported_at: String,
    actor: String,
}

/// Validate the operator's choices and derive the project identity.
///
/// Read-only: no directory is created and no registry is touched. An
/// unknown or incompatible `--profile` is refused with the existing
/// `unknown-profile`/`incompatible-profile` codes, an invalid `--id`
/// with `graduation-conflict`, and a destination that already holds a
/// manifest with `graduation-conflict`.
pub fn build_proposal(
    import: &GraduationImport,
    profile: &str,
    destination: &Path,
    id_override: Option<&str>,
) -> Result<GraduationProposal, ForgeError> {
    inspect_profile(profile)?;
    let id = derive_id(import, destination, id_override)?;
    let destination = canonical_destination(destination)?;
    for manifest in [
        crate::core::manifest::CANONICAL_MANIFEST,
        crate::core::manifest::LEGACY_MANIFEST,
    ] {
        if Path::new(&destination).join(manifest).is_file() {
            return Err(ForgeError::GraduationConflict {
                reason: format!(
                    "destination '{destination}' already holds {manifest}; a graduation import \
                     never overwrites an existing project"
                ),
            });
        }
    }
    Ok(GraduationProposal {
        id,
        name: import.brief.title.clone(),
        profile: profile.to_string(),
        destination,
    })
}

/// Accept a previewed proposal: write the minimal manifest and the
/// receipt, then register; roll the files back if any step fails.
pub fn adopt_graduation(
    registry: &mut Registry,
    proposal: &GraduationProposal,
    import: &GraduationImport,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<GraduationAdoption, ForgeError> {
    let destination = PathBuf::from(&proposal.destination);

    // Read-only identity pre-check, before anything is created.
    registry.check_identity_available(&proposal.id, &proposal.destination)?;

    // The manifest text is the existing minimal builder; it is parsed
    // here as a pre-write self-check so a bug cannot land a manifest
    // that the registry would reject. This runs before any filesystem
    // mutation.
    let text = crate::import::build_manifest_text(&proposal.id, &proposal.profile, None);
    crate::core::manifest::Manifest::parse(&destination.join("forge.yaml"), text.as_bytes())?;

    let actor = clean_actor(actor);
    let receipt = GraduationReceipt {
        contract: GRADUATION_CONTRACT_VERSION.to_string(),
        project_id: proposal.id.clone(),
        source: import.source.clone(),
        brief: import.brief.clone(),
        experiment: GraduationReceiptExperiment {
            summary: import.experiment.summary.clone(),
            validated: import.experiment.validated,
            evidence_count: import.experiment.evidence.len(),
        },
        imported_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
        actor,
    };
    let receipt_json =
        serde_json::to_string_pretty(&receipt).map_err(|err| ForgeError::GraduationConflict {
            reason: format!("graduation receipt is not serializable: {err}"),
        })?;

    // From here on, every failure path removes exactly what this
    // function wrote and (only when it created the destination) the
    // destination itself.
    let created_dir = !destination.exists();
    let forge_path = destination.join("forge.yaml");
    let receipt_dir = destination.join(GRADUATION_DIR).join(&proposal.id);
    let receipt_path = receipt_dir.join("import.json");

    if let Err(err) = fs::create_dir_all(&destination) {
        cleanup(&destination, &forge_path, &receipt_dir, created_dir);
        return Err(ForgeError::GraduationConflict {
            reason: format!(
                "destination '{}' could not be created: {err}; nothing was written",
                destination.display()
            ),
        });
    }
    if let Err(err) = fs::write(&forge_path, &text) {
        cleanup(&destination, &forge_path, &receipt_dir, created_dir);
        return Err(ForgeError::GraduationConflict {
            reason: format!(
                "manifest destination '{}' is unwritable: {err}; nothing was written",
                forge_path.display()
            ),
        });
    }
    if let Err(err) =
        fs::create_dir_all(&receipt_dir).and_then(|_| fs::write(&receipt_path, &receipt_json))
    {
        cleanup(&destination, &forge_path, &receipt_dir, created_dir);
        return Err(ForgeError::GraduationConflict {
            reason: format!(
                "receipt destination '{}' is unwritable: {err}; the manifest was removed",
                receipt_path.display()
            ),
        });
    }

    match registry.register(&destination, None) {
        Ok(record) => Ok(GraduationAdoption {
            record,
            receipt_path: receipt_path.display().to_string(),
        }),
        Err(err) => {
            cleanup(&destination, &forge_path, &receipt_dir, created_dir);
            Err(err)
        }
    }
}

fn cleanup(destination: &Path, forge_path: &Path, receipt_dir: &Path, created_dir: bool) {
    let _ = fs::remove_file(forge_path);
    let _ = fs::remove_dir_all(receipt_dir);
    // Remove the sidecar scaffolding only when it is now empty, so a
    // directory the operator already had is never destroyed.
    let _ = fs::remove_dir(destination.join(GRADUATION_DIR));
    let _ = fs::remove_dir(destination.join(".forge"));
    if created_dir {
        let _ = fs::remove_dir_all(destination);
    }
}

/// Derive the project id from `--id`, then the brief title, then the
/// destination basename. An explicit id is refused when it is not
/// kebab-case.
fn derive_id(
    import: &GraduationImport,
    destination: &Path,
    id_override: Option<&str>,
) -> Result<String, ForgeError> {
    if let Some(id) = id_override {
        return validate_project_id(id)
            .map(|_| id.to_string())
            .map_err(|reason| ForgeError::GraduationConflict {
                reason: format!("--id `{id}` {reason}"),
            });
    }
    let from_title = kebab(&import.brief.title);
    if validate_project_id(&from_title).is_ok() {
        return Ok(from_title);
    }
    let from_dir = destination
        .file_name()
        .and_then(|name| name.to_str())
        .map(kebab)
        .unwrap_or_default();
    if validate_project_id(&from_dir).is_ok() {
        return Ok(from_dir);
    }
    Err(ForgeError::GraduationConflict {
        reason: format!(
            "cannot derive a valid project id from the brief title '{}' or the destination '{}'; \
             re-run with --id <kebab-case-id>",
            import.brief.title,
            destination.display()
        ),
    })
}

/// Mirror of `crate::import::derive_project_id`'s kebab pattern.
fn kebab(raw: &str) -> String {
    let raw = raw.to_lowercase();
    let mut cleaned = String::with_capacity(raw.len());
    let mut prev_dash = true;
    for ch in raw.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            cleaned.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            cleaned.push('-');
            prev_dash = true;
        }
    }
    while cleaned.ends_with('-') {
        cleaned.pop();
    }
    cleaned
}

/// Canonical absolute destination. A missing parent is
/// `path-unavailable`; an existing directory is canonicalized directly
/// so the registry stores the same path it would for any other import.
fn canonical_destination(destination: &Path) -> Result<String, ForgeError> {
    if destination.exists() {
        return destination
            .canonicalize()
            .map(|path| path.display().to_string())
            .map_err(|_| ForgeError::PathUnavailable {
                path: destination.display().to_string(),
            });
    }
    let parent = match destination.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    if !parent.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: destination.display().to_string(),
        });
    }
    let file_name = destination
        .file_name()
        .ok_or_else(|| ForgeError::PathUnavailable {
            path: destination.display().to_string(),
        })?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: destination.display().to_string(),
        })?;
    Ok(canonical_parent.join(file_name).display().to_string())
}

/// Render a preview for human CLI output.
pub fn render_preview_human(preview: &GraduationPreview) -> String {
    let import = &preview.import;
    let mut lines = vec![
        format!("Graduation import preview ({GRADUATION_CONTRACT_VERSION})"),
        format!(
            "source: {} hypora_project={} hypora_revision={} graduated_at={}",
            import.source.contract,
            import.source.hypora_project_id,
            import.source.hypora_revision,
            import.source.graduated_at,
        ),
        "brief:".to_string(),
        format!("  title: {}", import.brief.title),
        format!("  problem: {}", import.brief.problem),
        format!("  audience: {}", import.brief.audience),
        format!("  solution: {}", import.brief.solution),
        format!("  requirements ({}):", import.brief.requirements.len()),
    ];
    for requirement in &import.brief.requirements {
        lines.push(format!("    - {requirement}"));
    }
    lines.push(format!(
        "  success metrics ({}):",
        import.brief.success_metrics.len()
    ));
    for metric in &import.brief.success_metrics {
        lines.push(format!(
            "    - {} target {} window {}",
            metric.name, metric.target, metric.window
        ));
    }
    lines.push(format!(
        "experiment: validated={} evidence={}",
        import.experiment.validated,
        import.experiment.evidence.len()
    ));
    for evidence in &import.experiment.evidence {
        lines.push(format!(
            "  - {} {}: {}",
            evidence.kind, evidence.observed_at, evidence.excerpt
        ));
    }
    match &preview.proposal {
        Some(proposal) => {
            lines.push("proposed project:".to_string());
            lines.push(format!("  id: {}", proposal.id));
            lines.push(format!("  name: {}", proposal.name));
            lines.push(format!("  profile: {}", proposal.profile));
            lines.push(format!("  destination: {}", proposal.destination));
            lines.push("No files were written. Re-run with --confirm to import.".to_string());
        }
        None => lines.push(
            "No files were written. Run `forge graduation import <ARTIFACT> --path <DIR> \
             --profile <PROFILE> --confirm` to create a project."
                .to_string(),
        ),
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graduation::{parse_artifact, validate_graduation};
    use serde_json::json;
    use std::path::Path;
    use tempfile::TempDir;

    fn artifact_text() -> String {
        json!({
            "contract": "platform.idea-graduation/0.1.0",
            "hypora_project_id": "prj_01H",
            "hypora_revision": "rev-2026-09-20-3",
            "graduated_at": "2026-09-20T00:00:00Z",
            "brief": {
                "title": "My Cool Idea!",
                "problem": "A problem.",
                "audience": "Students.",
                "solution": "A solution.",
                "requirements": ["R1."],
                "success_metrics": [ { "name": "m", "target": ">= 1", "window": "30d" } ],
            },
            "experiment": {
                "summary": "A summary.",
                "validated": true,
                "evidence": [ { "kind": "probe", "excerpt": "Excerpt.", "observed_at": "2026-09-18T00:00:00Z" } ],
            },
        })
        .to_string()
    }

    fn import() -> GraduationImport {
        validate_graduation(&parse_artifact(&artifact_text()).unwrap()).unwrap()
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-20T12:34:56Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn the_id_is_derived_from_the_title_then_overridden_by_id() {
        let import = import();
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("some-dir");
        let derived = build_proposal(&import, "rust-web", &dest, None).unwrap();
        assert_eq!(derived.id, "my-cool-idea");
        assert_eq!(derived.name, "My Cool Idea!");
        assert!(
            derived.destination.ends_with("some-dir"),
            "{}",
            derived.destination
        );
        assert!(
            Path::new(&derived.destination).is_absolute(),
            "{}",
            derived.destination
        );
        let overridden = build_proposal(&import, "rust-web", &dest, Some("explicit-id")).unwrap();
        assert_eq!(overridden.id, "explicit-id");
        let err = build_proposal(&import, "rust-web", &dest, Some("Bad_ID")).unwrap_err();
        assert_eq!(err.code(), "graduation-conflict");
    }

    #[test]
    fn an_unknown_profile_is_refused_read_only() {
        let import = import();
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("dest");
        let err = build_proposal(&import, "not-a-profile", &dest, None).unwrap_err();
        assert_eq!(err.code(), "unknown-profile");
        assert!(
            !dest.exists(),
            "no directory may be created by build_proposal"
        );
    }

    #[test]
    fn a_destination_with_a_manifest_is_a_conflict() {
        let import = import();
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("dest");
        std::fs::create_dir(&dest).unwrap();
        std::fs::write(dest.join("forge.yaml"), "schema: 1\n").unwrap();
        let err = build_proposal(&import, "rust-web", &dest, None).unwrap_err();
        assert_eq!(err.code(), "graduation-conflict");
        assert_eq!(
            std::fs::read_to_string(dest.join("forge.yaml")).unwrap(),
            "schema: 1\n"
        );
    }

    #[test]
    fn a_missing_destination_parent_is_path_unavailable() {
        let import = import();
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("missing").join("dest");
        let err = build_proposal(&import, "rust-web", &dest, None).unwrap_err();
        assert_eq!(err.code(), "path-unavailable");
    }

    #[test]
    fn adopt_writes_manifest_and_receipt_then_registers() {
        let import = import();
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("registry.db");
        let dest = tmp.path().join("my-idea");
        let mut registry = Registry::open(&db).unwrap();
        let proposal = build_proposal(&import, "rust-web", &dest, Some("my-idea")).unwrap();
        let adoption = adopt_graduation(&mut registry, &proposal, &import, "ops", now()).unwrap();
        assert_eq!(adoption.record.id, "my-idea");
        assert_eq!(adoption.record.profile, "rust-web");
        assert!(dest.join("forge.yaml").is_file());
        assert!(Path::new(&adoption.receipt_path).is_file());
        assert_eq!(registry.inspect("my-idea").unwrap().id, "my-idea");
    }

    #[test]
    fn a_registration_failure_leaves_no_manifest_and_no_receipt() {
        let import = import();
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("registry.db");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        // A legacy manifest next to the one adopt writes makes the
        // registry's manifest resolution ambiguous after the writes.
        std::fs::write(dest.join("platform.yaml"), "schema: 1\n").unwrap();
        let mut registry = Registry::open(&db).unwrap();
        let proposal = GraduationProposal {
            id: "my-idea".to_string(),
            name: "My Cool Idea!".to_string(),
            profile: "rust-web".to_string(),
            destination: dest.canonicalize().unwrap().display().to_string(),
        };
        let err = adopt_graduation(&mut registry, &proposal, &import, "ops", now()).unwrap_err();
        assert_eq!(err.code(), "ambiguous-manifest");
        assert!(
            !dest.join("forge.yaml").exists(),
            "manifest must be rolled back"
        );
        assert!(
            !dest.join(GRADUATION_DIR).exists(),
            "receipt must be rolled back"
        );
        assert!(
            dest.join("platform.yaml").is_file(),
            "the pre-existing file must be untouched"
        );
        assert!(registry.list().unwrap().is_empty());
    }

    #[test]
    fn the_receipt_is_under_the_project_root_and_round_trips() {
        let import = import();
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("registry.db");
        let dest = tmp.path().join("dest");
        let mut registry = Registry::open(&db).unwrap();
        let proposal = build_proposal(&import, "rust-web", &dest, Some("my-idea")).unwrap();
        let adoption = adopt_graduation(&mut registry, &proposal, &import, "ops", now()).unwrap();
        let receipt = adoption.receipt_path.clone();
        assert!(
            receipt.ends_with(".forge/graduation/my-idea/import.json"),
            "{receipt}"
        );
        let text = std::fs::read_to_string(&receipt).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["contract"], GRADUATION_CONTRACT_VERSION);
        assert_eq!(value["project_id"], "my-idea");
        assert_eq!(value["experiment"]["evidence_count"], 1);
        assert_eq!(value["imported_at"], "2026-09-20T12:34:56Z");
        assert_eq!(value["actor"], "ops");
        // Re-adopting at a second destination round-trips the same brief.
        let dest2 = tmp.path().join("dest2");
        let proposal2 = build_proposal(&import, "rust-web", &dest2, Some("my-idea-two")).unwrap();
        let adoption2 = adopt_graduation(&mut registry, &proposal2, &import, "ops", now()).unwrap();
        let text2 = std::fs::read_to_string(adoption2.receipt_path).unwrap();
        let value2: serde_json::Value = serde_json::from_str(&text2).unwrap();
        assert_eq!(value["brief"], value2["brief"]);
        assert_eq!(value["source"], value2["source"]);
    }

    #[test]
    fn the_receipt_carries_no_evidence_excerpt_and_no_extra_key() {
        let import = import();
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("registry.db");
        let dest = tmp.path().join("dest");
        let mut registry = Registry::open(&db).unwrap();
        let proposal = build_proposal(&import, "rust-web", &dest, Some("my-idea")).unwrap();
        let adoption = adopt_graduation(&mut registry, &proposal, &import, "ops", now()).unwrap();
        let text = std::fs::read_to_string(&adoption.receipt_path).unwrap();
        assert!(
            !text.contains("Excerpt."),
            "excerpt must never be persisted"
        );
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let top: Vec<&String> = value.as_object().unwrap().keys().collect();
        let mut top: Vec<String> = top.into_iter().cloned().collect();
        top.sort();
        assert_eq!(
            top,
            vec![
                "actor",
                "brief",
                "contract",
                "experiment",
                "imported_at",
                "project_id",
                "source",
            ]
        );
        let experiment = value["experiment"].as_object().unwrap();
        let mut keys: Vec<String> = experiment.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec!["evidence_count", "summary", "validated"]);
    }
}
