//! Traceable semantic project metadata proposals
//! (`project-semantic-description-review`).
//!
//! Core owns the versioned proposal record, the closed state machine
//! and the suggest/approve/reject/supersede operations; transports
//! (CLI now) render Core outcomes without reinterpreting them.
//!
//! A proposal is **always traceable**: the [`proposal::Proposal`]
//! records the project identity, the evidence revision it was
//! generated against, the bounded, control-free suggested text, the
//! confidence label and the provider identity. Two suggestions
//! targeting the same project and the same kind collapse to one
//! open proposal when they share an evidence revision; a later
//! evidence revision supersedes the prior proposal rather than
//! merging into it. Conflicting evidence (two sources implying
//! different values) is recorded as a [`proposal::ProposalState::Conflicted`]
//! proposal with each source retained; no approval is offered on a
//! conflict.
//!
//! Storage layout (under the project root):
//!
//! ```text
//! .forge/semantic/
//!   <project_id>/
//!     <proposal-id>/
//!       manifest.json   machine-readable Proposal + provenance
//!       proposal.md     bounded, human-readable description
//! ```
//!
//! Proposal ids are stable across invocations:
//! `<kind>-<short-hash>`, where the hash is a content-derived
//! identifier over the kind, the evidence revision, the evidence
//! source identity and the suggested value. Two calls that carry
//! the same inputs collapse to one proposal; two calls that differ
//! in any of those fields refuse to merge so the operator can
//! review each suggestion explicitly.
//!
//! The package makes no network call. The `Local` provider writes
//! the proposal from the caller-supplied evidence and the
//! `Operator` provider labels a manually-authored suggestion. Any
//! other provider id is refused at normalization so a model cannot
//! be implicitly cited as the source of truth.

pub mod apply;
pub mod derive;
pub mod proposal;
pub mod review;

pub use apply::{apply, ApplyOutcome};
pub use derive::{derive, MAX_DERIVE_TAGS, MAX_DOMAIN_CHARS};
pub use proposal::{
    parse_confidence, parse_kind, parse_provider, Confidence, Proposal, ProposalConflict,
    ProposalEvidence, ProposalId, ProposalKind, ProposalState, Provider, MAX_EVIDENCE_PATH_CHARS,
    MAX_PROPOSALS_PER_KIND, MAX_PROPOSAL_VALUE_CHARS, SEMANTIC_CONTRACT_VERSION, SEMANTIC_DIR,
};
pub use review::{
    approve, list, parse_field_pair, read, reject, suggest, supersede, DecideOutcome,
    ProposalListEntry, SuggestOutcome, SuggestRequest, SuggestStatus,
};
