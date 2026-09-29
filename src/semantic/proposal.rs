//! Traceable semantic proposal record and closed state machine
//! (`project-semantic-description-review`).
//!
//! Core owns the versioned proposal shape, the closed vocabulary
//! (`ProposalKind`, `ProposalState`, `Confidence`, `Provider`) and the
//! invariants a proposal must satisfy before it is stored:
//!
//! - **Closed kind set.** `Description`, `Domain`, `PortfolioTags`,
//!   `Profile`, `Lifecycle`. An unknown kind is refused at
//!   normalization.
//! - **Closed state machine.** `Suggested` is the only state a
//!   freshly-suggested proposal can hold. `Approved` and `Rejected`
//!   are reached only through an explicit operator transition.
//!   `Superseded` is set when a later evidence revision is bound
//!   to the same kind and a fresh proposal is suggested.
//!   `Conflicted` is set when the same evidence revision produces
//!   two incompatible values from the same evidence source.
//! - **Bounded, control-free, credential-scrubbed text.** Every
//!   string field is trimmed, control bytes are dropped, and the
//!   [`crate::policy::redact_credentials`] redactor runs over each
//!   one. The bound is [`MAX_PROPOSAL_VALUE_CHARS`] (2000 chars) so
//!   an unbounded payload cannot hide in a single field.
//! - **Revision-bound provenance.** Every proposal records the
//!   evidence revision it was generated against; an approval against
//!   a stale revision refuses.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Versioned contract data version for the semantic-proposal
/// surface.
pub const SEMANTIC_CONTRACT_VERSION: &str = "forge-semantic-proposal/0.1.0";

/// Directory (relative to the project root) holding semantic
/// proposals. Matches the `.forge/specs/` and `.forge/agents/`
/// layout the spec and agent packages use.
pub const SEMANTIC_DIR: &str = ".forge/semantic";

/// Maximum length of a single bounded string field (current value,
/// suggested value, evidence path, evidence revision, note). The
/// same 200-char bound the catalog uses; semantic metadata is
/// bounded on purpose so a generated blob cannot hide in the
/// proposal.
pub const MAX_PROPOSAL_VALUE_CHARS: usize = 2000;

/// Maximum length of a single evidence path token. Paths can be
/// long, but a 4096-char evidence path is not a metadata path; it
/// is an attempt to land bytes in a single field.
pub const MAX_EVIDENCE_PATH_CHARS: usize = 4096;

/// Maximum number of proposals the same kind may hold open at
/// once. The state machine is per-project-per-kind, so the bound
/// lives here rather than in the transport.
pub const MAX_PROPOSALS_PER_KIND: usize = 16;

/// What a proposal is changing. The set is closed: any other label
/// is refused at normalization so a model cannot smuggle in a
/// new kind as the source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProposalKind {
    /// Project description (a short, human-readable summary).
    Description,
    /// Domain classification (e.g. `tools`, `finance`, `research`).
    Domain,
    /// Portfolio tags the project should carry.
    PortfolioTags,
    /// Profile id the project should adopt.
    Profile,
    /// Lifecycle classification (e.g. `experimental`, `stable`,
    /// `deprecated`).
    Lifecycle,
}

impl ProposalKind {
    pub fn label(&self) -> &'static str {
        match self {
            ProposalKind::Description => "description",
            ProposalKind::Domain => "domain",
            ProposalKind::PortfolioTags => "portfolio-tags",
            ProposalKind::Profile => "profile",
            ProposalKind::Lifecycle => "lifecycle",
        }
    }

    /// Whether two kinds are mutually exclusive for the same
    /// evidence revision. The vocabulary is disjoint so two
    /// proposals of different kinds are never collapsed.
    pub fn same_kind(&self, other: ProposalKind) -> bool {
        self == &other
    }
}

impl fmt::Display for ProposalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Closed confidence vocabulary. Confidence is a label, not truth,
/// and never a basis for granting access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn label(&self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
        }
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Closed provider vocabulary. The Core contract is provider-neutral;
/// `Operator` and `Local` are the only two provider identities
/// Forge recognises today, and any other id is refused at
/// normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// A human authored the suggestion through the CLI; the
    /// proposal's evidence is what the operator supplied.
    Operator,
    /// The local Core contract produced the suggestion. The
    /// provider does not embed a model: it is the closed-vocabulary
    /// default for a forge-owned, deterministic suggestion.
    Local,
}

impl Provider {
    pub fn label(&self) -> &'static str {
        match self {
            Provider::Operator => "operator",
            Provider::Local => "local",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Closed state machine. Only an explicit operator transition may
/// move `Suggested` to `Approved` or `Rejected`. `Superseded` is
/// set by a later suggestion targeting the same kind and a
/// different evidence revision. `Conflicted` is set when two
/// suggestions for the same kind and the same evidence revision
/// disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalState {
    /// A new proposal awaiting an operator decision. No write has
    /// been attempted.
    Suggested,
    /// An operator approved the proposal. The actual change is
    /// still owned by the remediation and adapter packages; this
    /// package only records the operator decision.
    Approved,
    /// An operator rejected the proposal. The current value stays
    /// in force.
    Rejected,
    /// A later evidence revision superseded the proposal. An
    /// approval attempt against a superseded proposal is refused.
    Superseded,
    /// Two suggestions for the same kind and the same evidence
    /// revision disagreed. The proposal is not approved and the
    /// evidence sources are retained.
    Conflicted,
}

impl ProposalState {
    pub fn label(&self) -> &'static str {
        match self {
            ProposalState::Suggested => "suggested",
            ProposalState::Approved => "approved",
            ProposalState::Rejected => "rejected",
            ProposalState::Superseded => "superseded",
            ProposalState::Conflicted => "conflicted",
        }
    }

    /// Whether an operator may transition out of this state.
    pub fn operator_transitionable(&self) -> bool {
        matches!(self, ProposalState::Suggested)
    }
}

impl fmt::Display for ProposalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// One piece of evidence a proposal cites. Paths and revisions are
/// bounded, control-free and credential-scrubbed; an excerpt is
/// optional and may be empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalEvidence {
    /// Path to the source the evidence was drawn from (e.g.
    /// `README.md`, `forge.yaml`).
    pub path: String,
    /// Revision the path was observed at. Free-form but bounded;
    /// the package never parses this as a version.
    pub revision: String,
    /// Optional bounded excerpt. Empty when the proposal cites
    /// only the path.
    #[serde(default)]
    pub excerpt: String,
}

impl ProposalEvidence {
    /// Build a sanitized, bounded, credential-scrubbed evidence
    /// record. Returns `SemanticInvalid` when a field is empty or
    /// out of bounds.
    pub fn new(
        path: impl Into<String>,
        revision: impl Into<String>,
        excerpt: impl Into<String>,
    ) -> Result<Self, ForgeError> {
        let path = clean_bounded_field("evidence.path", path.into(), MAX_EVIDENCE_PATH_CHARS)?;
        let revision = clean_bounded_field(
            "evidence.revision",
            revision.into(),
            MAX_PROPOSAL_VALUE_CHARS,
        )?;
        let excerpt =
            clean_bounded_field("evidence.excerpt", excerpt.into(), MAX_PROPOSAL_VALUE_CHARS)?;
        if path.is_empty() {
            return Err(ForgeError::SemanticInvalid {
                reason: "evidence.path must not be empty".to_string(),
            });
        }
        if revision.is_empty() {
            return Err(ForgeError::SemanticInvalid {
                reason: "evidence.revision must not be empty".to_string(),
            });
        }
        Ok(ProposalEvidence {
            path,
            revision,
            excerpt,
        })
    }
}

/// A proposal that records two conflicting evidence sources for
/// the same kind and the same evidence revision. Each source is
/// retained; no approval is offered on the conflict. The
/// proposal is written as a single manifest so an audit can
/// reconstruct the disagreement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalConflict {
    /// Value the first source implied.
    pub first_value: String,
    /// Evidence the first source drew from.
    pub first_evidence: ProposalEvidence,
    /// Value the second source implied.
    pub second_value: String,
    /// Evidence the second source drew from.
    pub second_evidence: ProposalEvidence,
}

/// Stable, project-scoped proposal id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProposalId {
    /// Project id the proposal is anchored to.
    pub project_id: String,
    /// Kind the proposal targets.
    pub kind: ProposalKind,
    /// Stable short hash derived from the kind, the evidence
    /// revision, the evidence source identity and the suggested
    /// value. The hash is what makes a proposal deterministic
    /// across re-suggestions of the same inputs.
    pub hash: String,
}

impl ProposalId {
    /// Directory name (relative to `<SEMANTIC_DIR>/<project_id>/`).
    pub fn dir_name(&self) -> String {
        format!("{}-{}", self.kind.label(), self.hash)
    }
}

/// One full proposal record. The manifest on disk is the
/// authoritative state; the in-memory copy is a strict mirror so a
/// re-load is byte-equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// Contract data version. Bumped only on a breaking change to
    /// the proposal shape.
    pub contract: String,
    /// Proposal id (== directory name).
    pub id: ProposalId,
    /// Project id the proposal is anchored to.
    pub project_id: String,
    /// Kind the proposal targets.
    pub kind: ProposalKind,
    /// State of the proposal at the time of the manifest.
    pub state: ProposalState,
    /// Currently-observed or currently-approved value, when one
    /// exists; `None` when the project has no recorded value for
    /// this kind yet.
    #[serde(default)]
    pub current_value: Option<String>,
    /// Bounded, control-free, credential-scrubbed suggested value.
    pub suggested_value: String,
    /// Confidence label.
    pub confidence: Confidence,
    /// Provider identity (`operator` or `local`).
    pub provider: Provider,
    /// Evidence sources the proposal cites.
    pub evidence: Vec<ProposalEvidence>,
    /// Conflict payload, when `state == Conflicted`. Each retained
    /// source stays available so an operator can review the
    /// disagreement.
    #[serde(default)]
    pub conflict: Option<ProposalConflict>,
    /// Optional human note from the operator. Never auto-populated
    /// by the package; never a basis for an automatic transition.
    #[serde(default)]
    pub note: String,
    /// When the proposal was created (UTC, second precision).
    pub suggested_at: DateTime<Utc>,
    /// When the proposal was last operator-decided. Empty when no
    /// decision has been recorded yet.
    #[serde(default)]
    pub decided_at: Option<DateTime<Utc>>,
}

/// Parse a [`ProposalKind`] from its kebab-case label. Returns
/// `SemanticInvalid` for an unknown label.
pub fn parse_kind(raw: &str) -> Result<ProposalKind, ForgeError> {
    match raw.trim() {
        "description" => Ok(ProposalKind::Description),
        "domain" => Ok(ProposalKind::Domain),
        "portfolio-tags" | "portfolio_tags" => Ok(ProposalKind::PortfolioTags),
        "profile" => Ok(ProposalKind::Profile),
        "lifecycle" => Ok(ProposalKind::Lifecycle),
        other => Err(ForgeError::SemanticInvalid {
            reason: format!(
                "unknown proposal kind `{other}`; expected one of description, domain, portfolio-tags, profile, lifecycle"
            ),
        }),
    }
}

/// Parse a [`Confidence`] label. Returns `SemanticInvalid` for an
/// unknown label.
pub fn parse_confidence(raw: &str) -> Result<Confidence, ForgeError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(Confidence::Low),
        "medium" => Ok(Confidence::Medium),
        "high" => Ok(Confidence::High),
        other => Err(ForgeError::SemanticInvalid {
            reason: format!("unknown confidence `{other}`; expected low, medium, or high"),
        }),
    }
}

/// Parse a [`Provider`] label. Returns `SemanticInvalid` for an
/// unknown label. The closed set keeps a model from being cited
/// as the source of truth implicitly.
pub fn parse_provider(raw: &str) -> Result<Provider, ForgeError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "operator" => Ok(Provider::Operator),
        "local" => Ok(Provider::Local),
        other => Err(ForgeError::SemanticInvalid {
            reason: format!("unknown semantic provider `{other}`; expected `operator` or `local`"),
        }),
    }
}

/// Trim, control-strip, credential-scrub and char-bound a string
/// field. Empty inputs are allowed only for optional fields; the
/// caller enforces the empty rule.
pub fn clean_bounded_field(field: &str, raw: String, bound: usize) -> Result<String, ForgeError> {
    if raw.chars().count() > bound {
        return Err(ForgeError::SemanticInvalid {
            reason: format!("{field} exceeds the {bound}-char bound"),
        });
    }
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c.is_control() {
            // Drop control bytes but keep printable whitespace.
            if c == '\n' || c == '\r' || c == '\t' {
                out.push(c);
            }
            continue;
        }
        out.push(c);
    }
    let scrubbed = redact_credentials(&out);
    Ok(scrubbed)
}

/// Whether `value` carries an obvious secret shape that the
/// caller should not have supplied at all. The redactor still
/// scrubs on the way out, but a clear pre-check refuses the
/// obvious case with a typed error.
pub fn looks_like_credential(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.contains("ghp_")
        || trimmed.contains("gho_")
        || trimmed.contains("AKIA")
        || trimmed.contains("xoxb-")
        || trimmed.contains("xoxp-")
        || trimmed.contains("-----BEGIN ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kind_accepts_the_closed_set() {
        for (raw, expected) in [
            ("description", ProposalKind::Description),
            ("domain", ProposalKind::Domain),
            ("portfolio-tags", ProposalKind::PortfolioTags),
            ("profile", ProposalKind::Profile),
            ("lifecycle", ProposalKind::Lifecycle),
        ] {
            assert_eq!(parse_kind(raw).unwrap(), expected, "{raw}");
        }
    }

    #[test]
    fn parse_kind_rejects_unknown_labels() {
        for raw in ["", "tag", "Portfolio", "team"] {
            let err = parse_kind(raw).unwrap_err();
            assert!(matches!(err, ForgeError::SemanticInvalid { .. }), "{raw}");
        }
    }

    #[test]
    fn parse_confidence_accepts_the_closed_set() {
        assert_eq!(parse_confidence("low").unwrap(), Confidence::Low);
        assert_eq!(parse_confidence("Medium").unwrap(), Confidence::Medium);
        assert_eq!(parse_confidence("HIGH").unwrap(), Confidence::High);
    }

    #[test]
    fn parse_confidence_rejects_unknown_labels() {
        let err = parse_confidence("critical").unwrap_err();
        assert!(matches!(err, ForgeError::SemanticInvalid { .. }));
    }

    #[test]
    fn parse_provider_accepts_the_closed_set() {
        assert_eq!(parse_provider("operator").unwrap(), Provider::Operator);
        assert_eq!(parse_provider("Local").unwrap(), Provider::Local);
    }

    #[test]
    fn parse_provider_rejects_model_providers() {
        for raw in ["gpt", "claude", "openai", "anthropic"] {
            let err = parse_provider(raw).unwrap_err();
            assert!(matches!(err, ForgeError::SemanticInvalid { .. }), "{raw}");
        }
    }

    #[test]
    fn clean_bounded_field_drops_control_bytes_and_scrubs_credentials() {
        let cleaned = clean_bounded_field(
            "current_value",
            "hello\u{0001} token=ghp_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG end".to_string(),
            MAX_PROPOSAL_VALUE_CHARS,
        )
        .unwrap();
        assert!(cleaned.contains("[REDACTED]"), "{cleaned}");
        assert!(!cleaned.contains("ghp_"), "{cleaned}");
        assert!(!cleaned.contains('\u{0001}'), "{cleaned}");
    }

    #[test]
    fn clean_bounded_field_refuses_oversized_input() {
        let big = "x".repeat(MAX_PROPOSAL_VALUE_CHARS + 1);
        let err =
            clean_bounded_field("suggested_value", big, MAX_PROPOSAL_VALUE_CHARS).unwrap_err();
        assert!(matches!(err, ForgeError::SemanticInvalid { .. }));
    }

    #[test]
    fn proposal_id_dir_name_is_kind_then_hash() {
        let id = ProposalId {
            project_id: "demo".to_string(),
            kind: ProposalKind::Description,
            hash: "deadbeef".to_string(),
        };
        assert_eq!(id.dir_name(), "description-deadbeef");
    }

    #[test]
    fn state_machine_rejects_operator_transition_outside_suggested() {
        for state in [
            ProposalState::Approved,
            ProposalState::Rejected,
            ProposalState::Superseded,
            ProposalState::Conflicted,
        ] {
            assert!(!state.operator_transitionable(), "{state:?}");
        }
        assert!(ProposalState::Suggested.operator_transitionable());
    }
}
