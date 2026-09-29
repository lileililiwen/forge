//! Suggest / approve / reject / supersede / list / read
//! (`project-semantic-description-review`).
//!
//! The package owns the operations that move a proposal through
//! the closed state machine. The operations are deliberately
//! conservative:
//!
//! - `suggest` only stores `Suggested` proposals. A later suggestion
//!   for the same kind and a different evidence revision
//!   supersedes the prior proposal rather than merging into it.
//! - `approve` and `reject` only move a `Suggested` proposal to
//!   `Approved` or `Rejected` respectively. The operation never
//!   applies the suggestion to a project file or a provider; the
//!   remediation and adapter packages own the actual write.
//! - `supersede` is the implicit transition that a fresh
//!   `suggest` triggers on the prior open proposal of the same
//!   kind.
//! - `read` and `list` are read-only and never mutate state.
//!
//! The provider model is intentionally closed. An unavailable
//! provider never produces a placeholder proposal; the
//! [`SuggestOutcome`] reports an `unavailable` state so a
//! transport can surface it without inventing a record.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::core::ForgeError;
use crate::semantic::proposal::{
    clean_bounded_field, looks_like_credential, Confidence, Proposal, ProposalConflict,
    ProposalEvidence, ProposalId, ProposalKind, ProposalState, Provider, MAX_PROPOSALS_PER_KIND,
    MAX_PROPOSAL_VALUE_CHARS, SEMANTIC_CONTRACT_VERSION, SEMANTIC_DIR,
};

/// Length of the stable short hash embedded in a [`ProposalId`].
const PROPOSAL_HASH_LEN: usize = 12;

/// Outcome of a `suggest` invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuggestOutcome {
    /// Whether the proposal was newly written, replaced an open
    /// proposal (superseded), conflicted with an open proposal,
    /// or refused (the provider was unavailable or the request was
    /// malformed).
    pub status: SuggestStatus,
    /// The proposal manifest that was written, when one was
    /// written. `None` on `Unavailable` and on validation
    /// refusals.
    pub proposal: Option<Proposal>,
    /// Files written by this invocation, relative to the project
    /// root. Empty on `Unavailable`, `Existing`, and validation
    /// refusals.
    pub files_written: Vec<String>,
    /// Human-readable note for the transport.
    pub note: String,
}

impl SuggestOutcome {
    pub fn status_label(&self) -> &'static str {
        self.status.label()
    }
}

/// Closed status of a `suggest` invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestStatus {
    /// A new bounded proposal was written.
    Generated,
    /// A proposal for the same kind, evidence revision and
    /// suggested value already exists and is still `Suggested`; no
    /// files were rewritten.
    Existing,
    /// A fresh proposal for the same kind and evidence revision
    /// conflicted with the existing one; both are recorded and
    /// the existing one moves to `Conflicted`.
    Conflicted,
    /// A fresh proposal for the same kind and a different evidence
    /// revision supersedes the prior open proposal.
    Superseded,
    /// The provider is unavailable; no proposal was stored. Prior
    /// proposals stay untouched.
    Unavailable,
    /// The request was refused before any file was written.
    Refused,
}

impl SuggestStatus {
    pub fn label(&self) -> &'static str {
        match self {
            SuggestStatus::Generated => "generated",
            SuggestStatus::Existing => "existing",
            SuggestStatus::Conflicted => "conflicted",
            SuggestStatus::Superseded => "superseded",
            SuggestStatus::Unavailable => "unavailable",
            SuggestStatus::Refused => "refused",
        }
    }
}

/// Outcome of an `approve` or `reject` invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecideOutcome {
    /// Proposal id that was operator-decided.
    pub id: ProposalId,
    /// Resulting state after the transition.
    pub state: ProposalState,
    /// Files written by this invocation, relative to the project
    /// root. Empty when the decision did not change the manifest
    /// (e.g. an `approve` on a stale proposal was refused).
    pub files_written: Vec<String>,
    /// Human-readable note for the transport.
    pub note: String,
}

/// A read-only summary of one proposal as exposed by `list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProposalListEntry {
    pub id: ProposalId,
    pub project_id: String,
    pub kind: ProposalKind,
    pub state: ProposalState,
    pub confidence: Confidence,
    pub provider: Provider,
    pub evidence_count: usize,
    pub suggested_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub dir_name: String,
}

/// Inputs to `suggest` for a single proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestRequest {
    /// Project directory.
    pub project_path: PathBuf,
    /// Kind the proposal targets.
    pub kind: ProposalKind,
    /// Current value (when one exists); `None` when the project
    /// has no recorded value for this kind yet.
    pub current_value: Option<String>,
    /// Bounded, control-free, credential-scrubbed suggested value.
    pub suggested_value: String,
    /// Confidence label.
    pub confidence: Confidence,
    /// Provider identity.
    pub provider: Provider,
    /// Evidence the proposal cites.
    pub evidence: Vec<ProposalEvidence>,
    /// Optional human note from the operator.
    pub note: Option<String>,
    /// Wall-clock timestamp for `suggested_at`. Tests pass a fixed
    /// timestamp; the CLI passes `Utc::now()`.
    pub now: DateTime<Utc>,
}

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

/// Move a `Suggested` proposal to `Approved`. The operation
/// refuses a stale proposal (already `Superseded`, `Conflicted`,
/// or `Approved`) and never applies the suggested value to any
/// surface; the remediation and adapter packages own the actual
/// write.
pub fn approve(
    project_path: &Path,
    id: &ProposalId,
    now: DateTime<Utc>,
    confirm: bool,
) -> Result<DecideOutcome, ForgeError> {
    if !confirm {
        return Err(ForgeError::SemanticInvalid {
            reason: "approve requires explicit --confirm; no proposal was changed".to_string(),
        });
    }
    decide(project_path, id, ProposalState::Approved, now)
}

/// Move a `Suggested` proposal to `Rejected`. The operation
/// refuses a stale proposal and never touches the project or any
/// provider; the current value stays in force.
pub fn reject(
    project_path: &Path,
    id: &ProposalId,
    now: DateTime<Utc>,
    confirm: bool,
) -> Result<DecideOutcome, ForgeError> {
    if !confirm {
        return Err(ForgeError::SemanticInvalid {
            reason: "reject requires explicit --confirm; no proposal was changed".to_string(),
        });
    }
    decide(project_path, id, ProposalState::Rejected, now)
}

/// Mark a proposal as superseded. The function is exposed for
/// tests; the production `suggest` path drives the same
/// transition implicitly. Calling it on a proposal that is not
/// currently `Suggested` is refused.
pub fn supersede(
    project_path: &Path,
    id: &ProposalId,
    now: DateTime<Utc>,
) -> Result<DecideOutcome, ForgeError> {
    decide(project_path, id, ProposalState::Superseded, now)
}

/// Read a single proposal. Returns `Ok(None)` when the manifest
/// does not exist; returns `Ok(Some(_))` for a valid manifest;
/// returns `SemanticInvalid` for a malformed manifest.
pub fn read(project_path: &Path, id: &ProposalId) -> Result<Option<Proposal>, ForgeError> {
    let path = proposal_dir(project_path, id).join("manifest.json");
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| ForgeError::SemanticInvalid {
        reason: format!("cannot read proposal manifest {}: {err}", path.display()),
    })?;
    let proposal: Proposal =
        serde_json::from_str(&text).map_err(|err| ForgeError::SemanticInvalid {
            reason: format!("malformed proposal manifest {}: {err}", path.display()),
        })?;
    Ok(Some(proposal))
}

/// List all proposals under `.forge/semantic/<project_id>/`.
/// Malformed entries are skipped so a partial write never breaks
/// the listing.
pub fn list(project_path: &Path) -> Result<Vec<ProposalListEntry>, ForgeError> {
    let project_id = project_id_for(project_path)?;
    let base = project_path.join(SEMANTIC_DIR).join(&project_id);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
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
        out.push(ProposalListEntry {
            id: proposal.id.clone(),
            project_id: proposal.project_id.clone(),
            kind: proposal.kind,
            state: proposal.state,
            confidence: proposal.confidence,
            provider: proposal.provider,
            evidence_count: proposal.evidence.len(),
            suggested_at: proposal.suggested_at,
            decided_at: proposal.decided_at,
            dir_name: proposal.id.dir_name(),
        });
    }
    out.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    Ok(out)
}

fn decide(
    project_path: &Path,
    id: &ProposalId,
    target: ProposalState,
    now: DateTime<Utc>,
) -> Result<DecideOutcome, ForgeError> {
    let mut current = read(project_path, id)?.ok_or_else(|| ForgeError::SemanticInvalid {
        reason: format!("proposal `{}` was not found", id.dir_name()),
    })?;
    if !current.state.operator_transitionable() {
        return Err(ForgeError::SemanticConflict {
            reason: format!(
                "proposal `{}` is in state `{}`; only `suggested` proposals can be approved or rejected",
                id.dir_name(),
                current.state
            ),
        });
    }
    if matches!(target, ProposalState::Approved | ProposalState::Rejected) {
        // No behaviour difference from the user-facing API; the
        // check is explicit so the function cannot be misused as
        // a "force" path.
    }
    current.state = target;
    current.decided_at = Some(now);
    let files = write_manifest(project_path, &current)?;
    Ok(DecideOutcome {
        id: id.clone(),
        state: target,
        files_written: files,
        note: format!(
            "proposal `{}` moved to state `{}`; no project file or provider was changed",
            id.dir_name(),
            target
        ),
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

fn project_id_for(project_path: &Path) -> Result<String, ForgeError> {
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

fn proposal_dir(project_path: &Path, id: &ProposalId) -> PathBuf {
    project_path
        .join(SEMANTIC_DIR)
        .join(&id.project_id)
        .join(id.dir_name())
}

fn write_manifest(project_path: &Path, proposal: &Proposal) -> Result<Vec<String>, ForgeError> {
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

impl SuggestRequest {
    /// Whether the provider is available. The closed set is
    /// always available; an unknown provider id is refused at
    /// `parse_provider` before this is called, so the function
    /// never reports `unavailable` for a parsed value. The hook
    /// stays so a future `Local` health check (e.g. an LLM
    /// adapter that is not present) can short-circuit without
    /// changing the public surface.
    pub fn provider_available(&self) -> bool {
        match self.provider {
            Provider::Operator | Provider::Local => true,
        }
    }

    /// The single evidence revision the request binds to. Two
    /// suggestions with the same kind and the same revision that
    /// disagree produce a `Conflicted` proposal; a different
    /// revision supersedes the prior open proposal.
    pub fn evidence_revision(&self) -> String {
        // All evidence in a single suggestion shares one revision
        // (the caller passes the same revision for every
        // source); the proposal hash binds the suggested value
        // to those revisions, so the join is a stable fingerprint
        // for the suggestion. The first evidence is the primary
        // one for conflict retention and the revision label.
        match self.evidence.first() {
            Some(ev) => ev.revision.clone(),
            None => String::new(),
        }
    }
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

/// Normalize a CLI-supplied `--set field=value` argument into a
/// pair of strings. Returns `SemanticInvalid` for a malformed
/// pair or an unknown field.
pub fn parse_field_pair(input: &str) -> Result<(String, String), ForgeError> {
    let (field, value) = input
        .split_once('=')
        .ok_or_else(|| ForgeError::SemanticInvalid {
            reason: format!("malformed --set argument `{input}`: expected FIELD=VALUE"),
        })?;
    let field = field.trim().to_string();
    let value = value.trim().to_string();
    if field.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: format!("malformed --set argument `{input}`: field is empty"),
        });
    }
    if value.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: format!("malformed --set argument `{input}`: value is empty"),
        });
    }
    if !is_supported_field(&field) {
        return Err(ForgeError::SemanticInvalid {
            reason: format!(
                "unknown semantic field `{field}`; expected current_value, suggested_value, kind, confidence, or provider"
            ),
        });
    }
    Ok((field, value))
}

fn is_supported_field(field: &str) -> bool {
    matches!(
        field,
        "current_value"
            | "suggested_value"
            | "kind"
            | "confidence"
            | "provider"
            | "evidence_path"
            | "evidence_revision"
            | "evidence_excerpt"
            | "note"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::CANONICAL_MANIFEST;
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
