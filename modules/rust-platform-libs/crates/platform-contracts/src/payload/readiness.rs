//! `platform.readiness` payload.

use serde::{Deserialize, Serialize};

/// Readiness document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Readiness {
    /// Service identifier.
    pub service: String,
    /// Overall status.
    pub status: ReadinessStatus,
    /// RFC3339 timestamp the readiness was checked.
    pub checked_at: String,
    /// Per-dependency checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<ReadinessCheck>>,
    /// Optional tenant identifier for multi-tenant services.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// A single readiness check entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessCheck {
    /// Check name.
    pub name: String,
    /// Check status.
    pub status: ReadinessStatus,
    /// Optional RFC3339 observation timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    /// Optional human-readable detail. Must not exceed 512 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Readiness status enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStatus {
    /// Ready to serve traffic.
    Ready,
    /// Serving traffic but with degraded dependencies.
    Degraded,
    /// Not ready to serve traffic.
    NotReady,
}
