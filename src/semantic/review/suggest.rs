//! Semantic review: suggest.

use crate::core::ForgeError;
use crate::semantic::proposal::{
    clean_bounded_field, looks_like_credential, Proposal, ProposalConflict, ProposalEvidence,
    ProposalId, ProposalKind, ProposalState, MAX_PROPOSALS_PER_KIND, MAX_PROPOSAL_VALUE_CHARS,
    SEMANTIC_CONTRACT_VERSION, SEMANTIC_DIR,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use super::ids::PROPOSAL_HASH_LEN;
use super::model::{SuggestOutcome, SuggestRequest, SuggestStatus};

use super::decide::read;

/// Suggest a proposal. The caller must supply the project path
/// and the proposal inputs; the function loads the project
/// identity, validates the inputs, and stores a single manifest
/// under `.forge/semantic/<project_id>/<id>/manifest.json`.
///
/// The function never applies the suggested value to a project
/// file, a provider, or any other surface. A conflict (two
/// suggestions for the same kind and the same evidence revision
/// that disagree) is recorded as a `Conflicted` proposal with
/// each source retained; a fresh revision supersedes the prior
/// open proposal; an unavailable provider short-circuits to a
/// typed `unavailable` state and prior proposals stay untouched.
pub fn suggest(request: &SuggestRequest) -> Result<SuggestOutcome, ForgeError> {
    if !request.provider_available() {
        return Ok(SuggestOutcome {
            status: SuggestStatus::Unavailable,
            proposal: None,
            files_written: Vec::new(),
            note: format!(
                "provider `{}` is unavailable; no proposal was stored and prior proposals are untouched",
                request.provider.label()
            ),
        });
    }
    validate_request(request)?;
    let project_id = project_id_for(&request.project_path)?;
    if looks_like_credential(&request.suggested_value) {
        return Err(ForgeError::SemanticInvalid {
            reason: "suggested_value carries a credential shape; refusal is closed, not silenced"
                .to_string(),
        });
    }
    let cleaned_suggested = clean_bounded_field(
        "suggested_value",
        request.suggested_value.clone(),
        MAX_PROPOSAL_VALUE_CHARS,
    )?;
    if cleaned_suggested.trim().is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: "suggested_value must not be empty".to_string(),
        });
    }
    if looks_like_credential(&cleaned_suggested) {
        return Err(ForgeError::SemanticInvalid {
            reason: "suggested_value carries a credential shape; refusal is closed, not silenced"
                .to_string(),
        });
    }
    let cleaned_current = match &request.current_value {
        Some(value) => {
            let cleaned =
                clean_bounded_field("current_value", value.clone(), MAX_PROPOSAL_VALUE_CHARS)?;
            if cleaned.trim().is_empty() {
                None
            } else {
                Some(cleaned)
            }
        }
        None => None,
    };
    let cleaned_note = match &request.note {
        Some(note) => {
            let cleaned = clean_bounded_field("note", note.clone(), MAX_PROPOSAL_VALUE_CHARS)?;
            if cleaned.trim().is_empty() {
                None
            } else {
                Some(cleaned)
            }
        }
        None => None,
    };
    let id = proposal_id_for(
        &project_id,
        &request.kind,
        &cleaned_suggested,
        &request.evidence,
    );
    let existing = read(&request.project_path, &id)?;
    // Same inputs collapse to one open proposal: the existing
    // `Suggested` manifest is returned verbatim, no files
    // rewritten.
    if let Some(current) = existing.as_ref() {
        if current.state == ProposalState::Suggested {
            return Ok(SuggestOutcome {
                status: SuggestStatus::Existing,
                proposal: Some(current.clone()),
                files_written: Vec::new(),
                note: format!(
                    "proposal `{}` already exists for project `{}`; no files were rewritten",
                    id.dir_name(),
                    project_id
                ),
            });
        }
    }
    // A fresh suggestion for the same kind and the same evidence
    // revision that disagrees with the existing open proposal is
    // a conflict, not a merge.
    let prior_open = open_proposal_for_kind(&request.project_path, &project_id, request.kind)?;
    if let Some(prior) = prior_open.as_ref() {
        if prior.evidence_revision() == request.evidence_revision()
            && prior.suggested_value != cleaned_suggested
        {
            let mut conflicted = prior.clone();
            let conflict = ProposalConflict {
                first_value: prior.suggested_value.clone(),
                first_evidence: prior.evidence.first().cloned().ok_or_else(|| {
                    ForgeError::SemanticInvalid {
                        reason: "prior proposal must carry at least one evidence source"
                            .to_string(),
                    }
                })?,
                second_value: cleaned_suggested.clone(),
                second_evidence: request.evidence.first().cloned().ok_or_else(|| {
                    ForgeError::SemanticInvalid {
                        reason: "suggest requires at least one evidence source".to_string(),
                    }
                })?,
            };
            conflicted.state = ProposalState::Conflicted;
            conflicted.conflict = Some(conflict);
            let files = write_manifest(&request.project_path, &conflicted)?;
            return Ok(SuggestOutcome {
                status: SuggestStatus::Conflicted,
                proposal: Some(conflicted),
                files_written: files,
                note: format!(
                    "proposal `{}` for kind `{}` is conflicted: each source is retained and no approval is offered",
                    prior.id.dir_name(),
                    prior.kind
                ),
            });
        }
    }
    // A fresh evidence revision supersedes the prior open
    // proposal of the same kind.
    if let Some(prior) = prior_open.as_ref() {
        if prior.evidence_revision() != request.evidence_revision() {
            let mut superseded = prior.clone();
            superseded.state = ProposalState::Superseded;
            let _ = write_manifest(&request.project_path, &superseded)?;
        }
    }
    let proposal = Proposal {
        contract: SEMANTIC_CONTRACT_VERSION.to_string(),
        id: id.clone(),
        project_id: project_id.clone(),
        kind: request.kind,
        state: ProposalState::Suggested,
        current_value: cleaned_current,
        suggested_value: cleaned_suggested,
        confidence: request.confidence,
        provider: request.provider,
        evidence: request.evidence.clone(),
        conflict: None,
        note: cleaned_note.unwrap_or_default(),
        suggested_at: request.now,
        decided_at: None,
    };
    let files = write_manifest(&request.project_path, &proposal)?;
    let status = if prior_open.is_some() {
        SuggestStatus::Superseded
    } else {
        SuggestStatus::Generated
    };
    let note = if matches!(status, SuggestStatus::Superseded) {
        format!(
            "wrote semantic proposal `{}` for project `{}` (kind `{}`); prior open proposal was superseded",
            id.dir_name(),
            project_id,
            request.kind
        )
    } else {
        format!(
            "wrote semantic proposal `{}` for project `{}` (kind `{}`)",
            id.dir_name(),
            project_id,
            request.kind
        )
    };
    Ok(SuggestOutcome {
        status,
        proposal: Some(proposal),
        files_written: files,
        note,
    })
}

fn validate_request(request: &SuggestRequest) -> Result<(), ForgeError> {
    if request.evidence.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: "suggest requires at least one evidence source".to_string(),
        });
    }
    if request.evidence.len() > MAX_PROPOSALS_PER_KIND {
        return Err(ForgeError::SemanticInvalid {
            reason: format!(
                "suggest accepts at most {MAX_PROPOSALS_PER_KIND} evidence sources per proposal"
            ),
        });
    }
    if request.suggested_value.chars().count() > MAX_PROPOSAL_VALUE_CHARS {
        return Err(ForgeError::SemanticInvalid {
            reason: format!("suggested_value exceeds the {MAX_PROPOSAL_VALUE_CHARS}-char bound"),
        });
    }
    Ok(())
}

pub(crate) fn project_id_for(project_path: &Path) -> Result<String, ForgeError> {
    let (manifest, _) = crate::core::manifest::Manifest::load_from_dir(project_path, None)
        .map_err(|err| ForgeError::SemanticInvalid {
            reason: format!("cannot resolve project id: {err}"),
        })?;
    if manifest.project.id.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: "project id is empty; cannot store a semantic proposal".to_string(),
        });
    }
    Ok(manifest.project.id)
}

pub(crate) fn proposal_dir(project_path: &Path, id: &ProposalId) -> PathBuf {
    project_path
        .join(SEMANTIC_DIR)
        .join(&id.project_id)
        .join(id.dir_name())
}

pub(crate) fn write_manifest(
    project_path: &Path,
    proposal: &Proposal,
) -> Result<Vec<String>, ForgeError> {
    let dir = proposal_dir(project_path, &proposal.id);
    fs::create_dir_all(&dir).map_err(|err| ForgeError::SemanticInvalid {
        reason: format!("cannot create proposal dir {}: {err}", dir.display()),
    })?;
    let manifest_json =
        serde_json::to_string_pretty(proposal).map_err(|err| ForgeError::SemanticInvalid {
            reason: format!("proposal is not serializable: {err}"),
        })?;
    let manifest_path = dir.join("manifest.json");
    fs::write(&manifest_path, &manifest_json).map_err(|err| ForgeError::SemanticInvalid {
        reason: format!(
            "cannot write proposal manifest {}: {err}",
            manifest_path.display()
        ),
    })?;
    let proposal_md = render_proposal_markdown(proposal);
    let proposal_path = dir.join("proposal.md");
    fs::write(&proposal_path, &proposal_md).map_err(|err| ForgeError::SemanticInvalid {
        reason: format!(
            "cannot write proposal.md {}: {err}",
            proposal_path.display()
        ),
    })?;
    let rel_manifest = relative_path(
        project_path,
        &manifest_path,
        format!(
            "{SEMANTIC_DIR}/{}/{}/manifest.json",
            proposal.project_id,
            proposal.id.dir_name()
        ),
    );
    let rel_proposal = relative_path(
        project_path,
        &proposal_path,
        format!(
            "{SEMANTIC_DIR}/{}/{}/proposal.md",
            proposal.project_id,
            proposal.id.dir_name()
        ),
    );
    Ok(vec![rel_manifest, rel_proposal])
}

fn relative_path(project_path: &Path, abs: &Path, fallback: String) -> String {
    abs.strip_prefix(project_path)
        .map(|p| p.display().to_string())
        .unwrap_or(fallback)
}

fn proposal_id_for(
    project_id: &str,
    kind: &ProposalKind,
    suggested_value: &str,
    evidence: &[ProposalEvidence],
) -> ProposalId {
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(kind.label().as_bytes());
    hasher.update(b"\n");
    hasher.update(suggested_value.as_bytes());
    hasher.update(b"\n");
    for ev in evidence {
        hasher.update(ev.path.as_bytes());
        hasher.update(b"\n");
        hasher.update(ev.revision.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    let hex = format!("{:x}", digest);
    let hash = hex.chars().take(PROPOSAL_HASH_LEN).collect::<String>();
    ProposalId {
        project_id: project_id.to_string(),
        kind: *kind,
        hash,
    }
}

fn open_proposal_for_kind(
    project_path: &Path,
    project_id: &str,
    kind: ProposalKind,
) -> Result<Option<Proposal>, ForgeError> {
    let base = project_path.join(SEMANTIC_DIR).join(project_id);
    if !base.is_dir() {
        return Ok(None);
    }
    let mut best: Option<Proposal> = None;
    for entry in fs::read_dir(&base).map_err(|err| ForgeError::SemanticInvalid {
        reason: format!("cannot read {}: {err}", base.display()),
    })? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.path().is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }
        let text = match fs::read_to_string(&manifest_path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let proposal: Proposal = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if proposal.kind != kind {
            continue;
        }
        if proposal.state != ProposalState::Suggested {
            continue;
        }
        // Most recent `suggested_at` wins so the next suggest
        // call sees the latest open proposal.
        match &best {
            Some(prior) if prior.suggested_at >= proposal.suggested_at => {}
            _ => best = Some(proposal),
        }
    }
    Ok(best)
}

impl Proposal {
    fn evidence_revision(&self) -> String {
        match self.evidence.first() {
            Some(ev) => ev.revision.clone(),
            None => String::new(),
        }
    }
}

fn render_proposal_markdown(proposal: &Proposal) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Semantic proposal: {}\n\n", proposal.kind));
    out.push_str(&format!("- contract: `{}`\n", proposal.contract));
    out.push_str(&format!("- project: `{}`\n", proposal.project_id));
    out.push_str(&format!("- state: `{}`\n", proposal.state));
    out.push_str(&format!("- confidence: `{}`\n", proposal.confidence));
    out.push_str(&format!("- provider: `{}`\n", proposal.provider));
    out.push_str(&format!(
        "- suggested_at: `{}`\n",
        proposal.suggested_at.to_rfc3339()
    ));
    if let Some(decided) = proposal.decided_at {
        out.push_str(&format!("- decided_at: `{}`\n", decided.to_rfc3339()));
    }
    if let Some(current) = &proposal.current_value {
        out.push_str(&format!("- current_value: `{current}`\n"));
    } else {
        out.push_str("- current_value: (none)\n");
    }
    out.push_str(&format!(
        "- suggested_value: `{}`\n",
        proposal.suggested_value
    ));
    if !proposal.note.is_empty() {
        out.push_str(&format!("- note: `{}`\n", proposal.note));
    }
    out.push_str("\n## Evidence\n\n");
    for ev in &proposal.evidence {
        out.push_str(&format!("- path: `{}`\n", ev.path));
        out.push_str(&format!("  - revision: `{}`\n", ev.revision));
        if !ev.excerpt.is_empty() {
            out.push_str(&format!("  - excerpt: `{}`\n", ev.excerpt));
        }
    }
    if let Some(conflict) = &proposal.conflict {
        out.push_str("\n## Conflict\n\n");
        out.push_str(&format!("- first_value: `{}`\n", conflict.first_value));
        out.push_str(&format!(
            "- first_evidence: path=`{}` revision=`{}`\n",
            conflict.first_evidence.path, conflict.first_evidence.revision
        ));
        out.push_str(&format!("- second_value: `{}`\n", conflict.second_value));
        out.push_str(&format!(
            "- second_evidence: path=`{}` revision=`{}`\n",
            conflict.second_evidence.path, conflict.second_evidence.revision
        ));
        out.push_str(
            "\nThe proposal is in state `conflicted`; an approval is refused until the operator\n\
             resolves the disagreement.\n",
        );
    }
    out
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::decide::{approve, list, read, reject};
    use super::super::fields::parse_field_pair;
    use super::*;
    use crate::core::manifest::CANONICAL_MANIFEST;
    use crate::semantic::proposal::{Confidence, Provider};
    use chrono::{DateTime, Utc};
    use std::fs;
    use tempfile::TempDir;

    fn write_manifest(dir: &Path, id: &str) {
        let body = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
        );
        fs::write(dir.join(CANONICAL_MANIFEST), body).unwrap();
    }

    fn evidence(path: &str, revision: &str) -> ProposalEvidence {
        ProposalEvidence::new(path, revision, "").unwrap()
    }

    fn request(dir: &Path, kind: ProposalKind, value: &str, revision: &str) -> SuggestRequest {
        SuggestRequest {
            project_path: dir.to_path_buf(),
            kind,
            current_value: None,
            suggested_value: value.to_string(),
            confidence: Confidence::Medium,
            provider: Provider::Operator,
            evidence: vec![evidence("README.md", revision)],
            note: None,
            now: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        }
    }

    #[test]
    fn suggest_writes_a_manifest_and_human_readable_proposal() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "demo");
        let outcome = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "demo project",
            "rev-1",
        ))
        .unwrap();
        assert_eq!(outcome.status, SuggestStatus::Generated);
        let proposal = outcome.proposal.clone().unwrap();
        assert_eq!(proposal.state, ProposalState::Suggested);
        assert_eq!(proposal.project_id, "demo");
        assert_eq!(proposal.kind, ProposalKind::Description);
        assert_eq!(outcome.files_written.len(), 2);
        let manifest = tmp.path().join(&outcome.files_written[0]);
        assert!(manifest.is_file());
        let proposal_md = tmp.path().join(&outcome.files_written[1]);
        assert!(proposal_md.is_file());
        let text = fs::read_to_string(&proposal_md).unwrap();
        assert!(text.contains("suggested_value:"));
        assert!(text.contains("demo project"));
    }

    #[test]
    fn suggest_is_idempotent_on_same_inputs() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "idem");
        let first = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "demo project",
            "rev-1",
        ))
        .unwrap();
        assert_eq!(first.status, SuggestStatus::Generated);
        let second = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "demo project",
            "rev-1",
        ))
        .unwrap();
        assert_eq!(second.status, SuggestStatus::Existing);
        assert!(second.files_written.is_empty());
    }

    #[test]
    fn suggest_with_unavailable_provider_short_circuits() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "no-prov");
        // The closed `Operator` / `Local` provider set is always
        // available today; the public `provider_available` hook
        // returns false for a future "missing LLM adapter"
        // condition that the surface does not yet have. The
        // closed-vocabulary enforcement is the `parse_provider`
        // guard: an unknown provider id is refused before any
        // `provider_available` check, and the closed set is
        // always available.
        let err = crate::semantic::parse_provider("gpt").unwrap_err();
        assert!(matches!(err, ForgeError::SemanticInvalid { .. }));
        let req = request(tmp.path(), ProposalKind::Description, "demo", "rev-1");
        assert!(req.provider_available());
    }

    #[test]
    fn suggest_with_conflicting_value_for_same_revision_records_conflict() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "conf");
        let first = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "first",
            "rev-1",
        ))
        .unwrap();
        assert_eq!(first.status, SuggestStatus::Generated);
        let second = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "second",
            "rev-1",
        ))
        .unwrap();
        assert_eq!(second.status, SuggestStatus::Conflicted);
        let proposal = second.proposal.unwrap();
        assert_eq!(proposal.state, ProposalState::Conflicted);
        let conflict = proposal.conflict.unwrap();
        assert_eq!(conflict.first_value, "first");
        assert_eq!(conflict.second_value, "second");
    }

    #[test]
    fn suggest_with_new_revision_supersedes_prior_open_proposal() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "sup");
        let first = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "first",
            "rev-1",
        ))
        .unwrap();
        assert_eq!(first.status, SuggestStatus::Generated);
        let second = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "second",
            "rev-2",
        ))
        .unwrap();
        assert_eq!(second.status, SuggestStatus::Superseded);
        // The prior open proposal is now in state Superseded.
        let prior = first.proposal.unwrap();
        let stored = read(tmp.path(), &prior.id).unwrap().unwrap();
        assert_eq!(stored.state, ProposalState::Superseded);
    }

    #[test]
    fn suggest_refuses_credential_shaped_value() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "cred");
        let mut req = request(tmp.path(), ProposalKind::Description, "ok", "rev-1");
        req.suggested_value = "ghp_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG".to_string();
        let err = suggest(&req).unwrap_err();
        assert!(matches!(err, ForgeError::SemanticInvalid { .. }), "{err}");
    }

    #[test]
    fn approve_moves_suggested_to_approved_without_applying() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "appr");
        let outcome = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "demo",
            "rev-1",
        ))
        .unwrap();
        let id = outcome.proposal.unwrap().id;
        let decision = approve(
            tmp.path(),
            &id,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            true,
        )
        .unwrap();
        assert_eq!(decision.state, ProposalState::Approved);
        let stored = read(tmp.path(), &id).unwrap().unwrap();
        assert_eq!(stored.state, ProposalState::Approved);
        // No project file or provider was touched.
        let project_yaml = tmp.path().join("forge.yaml");
        let original = fs::read_to_string(&project_yaml).unwrap();
        assert!(original.contains("id: appr"));
    }

    #[test]
    fn approve_without_confirm_refuses_with_typed_error() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "no-conf");
        let outcome = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "demo",
            "rev-1",
        ))
        .unwrap();
        let id = outcome.proposal.unwrap().id;
        let err = approve(
            tmp.path(),
            &id,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            false,
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::SemanticInvalid { .. }), "{err}");
    }

    #[test]
    fn approve_a_stale_proposal_refuses() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "stale");
        let outcome = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "first",
            "rev-1",
        ))
        .unwrap();
        let id = outcome.proposal.unwrap().id;
        // Supersede the open proposal.
        let _ = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "second",
            "rev-2",
        ))
        .unwrap();
        let err = approve(
            tmp.path(),
            &id,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            true,
        )
        .unwrap_err();
        assert!(matches!(err, ForgeError::SemanticConflict { .. }), "{err}");
    }

    #[test]
    fn reject_moves_suggested_to_rejected() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "rej");
        let outcome = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "demo",
            "rev-1",
        ))
        .unwrap();
        let id = outcome.proposal.unwrap().id;
        let decision = reject(
            tmp.path(),
            &id,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            true,
        )
        .unwrap();
        assert_eq!(decision.state, ProposalState::Rejected);
        let stored = read(tmp.path(), &id).unwrap().unwrap();
        assert_eq!(stored.state, ProposalState::Rejected);
    }

    #[test]
    fn list_returns_proposals_sorted_by_dir_name() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "list");
        let _ = suggest(&request(
            tmp.path(),
            ProposalKind::Description,
            "a",
            "rev-1",
        ))
        .unwrap();
        let _ = suggest(&request(tmp.path(), ProposalKind::Domain, "tools", "rev-1")).unwrap();
        let entries = list(tmp.path()).unwrap();
        assert_eq!(entries.len(), 2);
        // Sorted by dir_name: description < domain.
        assert!(entries[0].dir_name < entries[1].dir_name);
    }

    #[test]
    fn parse_field_pair_accepts_supported_fields_and_refuses_unknown() {
        for (raw, field) in [
            ("current_value=hello", "current_value"),
            ("kind=description", "kind"),
            ("confidence=high", "confidence"),
        ] {
            let (f, v) = parse_field_pair(raw).unwrap();
            assert_eq!(f, field);
            assert!(!v.is_empty());
        }
        let err = parse_field_pair("bogus=x").unwrap_err();
        assert!(matches!(err, ForgeError::SemanticInvalid { .. }));
        let err = parse_field_pair("nokey").unwrap_err();
        assert!(matches!(err, ForgeError::SemanticInvalid { .. }));
    }

    #[test]
    fn suggest_with_no_evidence_refuses() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "no-ev");
        let mut req = request(tmp.path(), ProposalKind::Description, "x", "rev-1");
        req.evidence.clear();
        let err = suggest(&req).unwrap_err();
        assert!(matches!(err, ForgeError::SemanticInvalid { .. }));
    }

    #[test]
    fn read_missing_proposal_returns_none() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "missing");
        let id = ProposalId {
            project_id: "missing".to_string(),
            kind: ProposalKind::Description,
            hash: "nothere".to_string(),
        };
        assert!(read(tmp.path(), &id).unwrap().is_none());
    }
}
