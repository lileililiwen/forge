//! The deploy status projection of a journal row's phase evidence.
//!
//! One rule, one place: a value the journal row carries is that value,
//! and a value the row does not carry is `unknown` — never an omitted
//! field, so a deploy status entry never reads as a verified success by
//! silence.
//!
//! The projection is a pure function over one [`OperationEntry`]: it
//! contacts no provider, reads no directory and writes nothing. The
//! registry owns the journal row and the meaning of its columns, so the
//! rule lives here, beside the row, and the CLI's human renderer, the
//! CLI's JSON document and the API's JSON document all read it.

use serde::Serialize;

use super::model::OperationEntry;

/// The value a reader sees for a phase the journal row does not carry.
pub const UNKNOWN_PHASE_VALUE: &str = "unknown";

/// One journal row's phase evidence, projected for readers.
///
/// A value the row carries is that value; a value it does not carry is
/// `unknown`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhaseEvidence {
    /// The committed revision the row carried, or `unknown`.
    pub revision: String,
    /// The terminal build status the row carried, or `unknown`.
    pub build_status: String,
    /// The terminal run status the row carried, or `unknown`.
    pub run_status: String,
    /// The container identity the row carried, or `unknown`.
    pub container_identity: String,
}

impl PhaseEvidence {
    /// Project one journal row's phase evidence.
    pub fn project(entry: &OperationEntry) -> Self {
        Self {
            revision: value(entry.revision.as_deref()),
            build_status: value(entry.build_status.as_deref()),
            run_status: value(entry.run_status.as_deref()),
            container_identity: value(entry.container_identity.as_deref()),
        }
    }
}

/// The value a reader sees: the row's own value, or `unknown` when the
/// row carries none.
fn value(carried: Option<&str>) -> String {
    carried
        .map(str::to_string)
        .unwrap_or_else(|| UNKNOWN_PHASE_VALUE.to_string())
}

/// One journal row projected for the deploy status surfaces.
///
/// The row keeps every field it has today; only the four phase
/// evidence fields change from *absent* to *always present*, carrying
/// `unknown` when the row carries none.
#[derive(Debug, Clone, Serialize)]
pub struct DeployStatusEntry {
    /// The journal row's own operation id.
    pub op_id: i64,
    /// The journal row's own kind.
    pub kind: String,
    /// The journal row's own project id.
    pub project_id: String,
    /// The journal row's own state.
    pub state: String,
    /// The journal row's own start timestamp.
    pub started_at: String,
    /// The journal row's own finish timestamp.
    pub finished_at: Option<String>,
    /// The journal row's own detail line.
    pub detail: Option<String>,
    /// The journal row's own idempotency key, when it carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// The journal row's own request hash, when it carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_hash: Option<String>,
    /// The journal row's own fleet queue id, when it carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<String>,
    /// The revision the row carried, or `unknown`.
    pub revision: String,
    /// The build status the row carried, or `unknown`.
    pub build_status: String,
    /// The run status the row carried, or `unknown`.
    pub run_status: String,
    /// The container identity the row carried, or `unknown`.
    pub container_identity: String,
}

impl From<&OperationEntry> for DeployStatusEntry {
    fn from(entry: &OperationEntry) -> Self {
        let evidence = PhaseEvidence::project(entry);
        Self {
            op_id: entry.op_id,
            kind: entry.kind.clone(),
            project_id: entry.project_id.clone(),
            state: entry.state.clone(),
            started_at: entry.started_at.clone(),
            finished_at: entry.finished_at.clone(),
            detail: entry.detail.clone(),
            idempotency_key: entry.idempotency_key.clone(),
            request_hash: entry.request_hash.clone(),
            queue_id: entry.queue_id.clone(),
            revision: evidence.revision,
            build_status: evidence.build_status,
            run_status: evidence.run_status,
            container_identity: evidence.container_identity,
        }
    }
}
