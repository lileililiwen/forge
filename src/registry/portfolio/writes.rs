//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::portfolio::{Confidence, EvidenceStatus, Lifecycle};

/// A validated, ready-to-write user-owned portfolio record.
#[derive(Debug, Clone, Default)]
pub struct PortfolioWrite {
    pub lifecycle: Option<Lifecycle>,
    pub confidence: Option<Confidence>,
    pub next_action: Option<String>,
    pub blocker: Option<String>,
}
/// A validated, ready-to-write evidence snapshot. Every field is
/// required except the freshness bound: a snapshot without an
/// honest source, revision or observation time would let a
/// local observation pose as an external one.
#[derive(Debug, Clone)]
pub struct SnapshotWrite {
    pub source_system: String,
    pub source_revision: String,
    pub observed_at: String,
    pub status: EvidenceStatus,
    pub stale_after: Option<String>,
    pub evidence_json: String,
}
