//! `platform.tenant-context` payload.

use serde::{Deserialize, Serialize};

/// Tenant context document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantContext {
    /// Tenant identifier.
    pub tenant_id: String,
    /// Optional human-readable tenant name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_name: Option<String>,
    /// Environment.
    pub environment: Environment,
    /// Optional region label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Isolation level.
    pub isolation_level: IsolationLevel,
    /// RFC3339 timestamp the context was active at.
    pub active_at: String,
}

/// Runtime environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    /// Production.
    Production,
    /// Staging.
    Staging,
    /// Development.
    Development,
    /// Test.
    Test,
}

/// Tenant isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    /// Logical isolation (shared infrastructure).
    Logical,
    /// Dedicated infrastructure.
    Dedicated,
}
