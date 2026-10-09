//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::semantic::proposal::{
    Confidence, Proposal, ProposalEvidence, ProposalId, ProposalKind, ProposalState, Provider,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;

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
        match self.evidence.first() {
            Some(ev) => ev.revision.clone(),
            None => String::new(),
        }
    }
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
