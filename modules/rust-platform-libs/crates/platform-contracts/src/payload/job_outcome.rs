//! `platform.job-outcome` payload.

use serde::{Deserialize, Serialize};

/// Job outcome document.
///
/// The wire shape mirrors `schemas/job-outcome.schema.json` in the
/// `platform-contracts` repository. The Rust type is a permissive
/// builder; the JSON Schema is the source of truth for what is
/// accepted on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobOutcome {
    /// Stable job identifier.
    pub job_id: String,
    /// Job type, e.g. `data_import`.
    pub job_type: String,
    /// RFC3339 timestamp the job started.
    pub started_at: String,
    /// RFC3339 timestamp the job finished, if it has terminated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Terminal job result.
    pub result: JobResult,
    /// Number of items processed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items_processed: Option<u64>,
    /// Number of items that failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items_failed: Option<u64>,
    /// Optional tenant identifier for multi-tenant runners.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Terminal result of a background job. Mirrors the `result` enum in
/// the platform schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobResult {
    /// The job succeeded.
    Succeeded,
    /// The job failed.
    Failed,
    /// The job partially succeeded.
    Partial,
    /// The job was cancelled.
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_value_round_trips() {
        let value = JobOutcome {
            job_id: "job_basic".to_string(),
            job_type: "data_import".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            finished_at: None,
            result: JobResult::Succeeded,
            items_processed: None,
            items_failed: None,
            tenant_id: None,
        };
        let json = serde_json::to_value(&value).expect("serializes");
        for key in ["job_id", "job_type", "started_at", "result"] {
            assert!(json.get(key).is_some(), "missing required key {key}");
        }
        let back: JobOutcome = serde_json::from_value(json).expect("deserializes");
        assert_eq!(back, value);
    }
}
