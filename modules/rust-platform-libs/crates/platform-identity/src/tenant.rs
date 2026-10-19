//! Tenant types.

use platform_core::bounded::BoundedString;
use platform_core::id::Id;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Tenant identifier, bounded to 64 characters to mirror the
/// `platform-contracts` schema's `maxLength: 64` on `tenant_id`.
pub type TenantId = Id<TenantTag>;

/// Phantom tag for [`TenantId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TenantTag {}

/// Runtime environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    /// Logical isolation (shared infrastructure).
    Logical,
    /// Dedicated infrastructure.
    Dedicated,
}

/// Resolved tenant. The application owns the lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tenant {
    id: TenantId,
    name: Option<BoundedString<256>>,
    environment: Environment,
    isolation_level: IsolationLevel,
}

impl Tenant {
    /// Construct a tenant.
    #[must_use]
    pub fn new(id: TenantId, environment: Environment, isolation_level: IsolationLevel) -> Self {
        Self {
            id,
            name: None,
            environment,
            isolation_level,
        }
    }

    /// Attach a display name.
    #[must_use]
    pub fn with_name(mut self, name: BoundedString<256>) -> Self {
        self.name = Some(name);
        self
    }

    /// Tenant identifier accessor.
    #[must_use]
    pub fn id(&self) -> &TenantId {
        &self.id
    }

    /// Display name accessor.
    #[must_use]
    pub fn name(&self) -> Option<&BoundedString<256>> {
        self.name.as_ref()
    }

    /// Environment accessor.
    #[must_use]
    pub fn environment(&self) -> Environment {
        self.environment
    }

    /// Isolation level accessor.
    #[must_use]
    pub fn isolation_level(&self) -> IsolationLevel {
        self.isolation_level
    }
}

/// Tenant context under which a request is executed.
///
/// Mirrors the `platform.tenant-context` contract payload. The
/// application builds this once at the edge of a request and threads
/// it through downstream calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantContext {
    tenant_id: TenantId,
    tenant_name: Option<BoundedString<256>>,
    environment: Environment,
    region: Option<BoundedString<64>>,
    isolation_level: IsolationLevel,
    #[serde(with = "time::serde::rfc3339")]
    active_at: OffsetDateTime,
}

impl TenantContext {
    /// Construct a tenant context.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        environment: Environment,
        isolation_level: IsolationLevel,
        active_at: OffsetDateTime,
    ) -> Self {
        Self {
            tenant_id,
            tenant_name: None,
            environment,
            region: None,
            isolation_level,
            active_at,
        }
    }

    /// Attach a tenant name.
    #[must_use]
    pub fn with_name(mut self, name: BoundedString<256>) -> Self {
        self.tenant_name = Some(name);
        self
    }

    /// Attach a region.
    #[must_use]
    pub fn with_region(mut self, region: BoundedString<64>) -> Self {
        self.region = Some(region);
        self
    }

    /// Tenant identifier accessor.
    #[must_use]
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    /// Environment accessor.
    #[must_use]
    pub fn environment(&self) -> Environment {
        self.environment
    }

    /// Isolation level accessor.
    #[must_use]
    pub fn isolation_level(&self) -> IsolationLevel {
        self.isolation_level
    }

    /// Active-at accessor.
    #[must_use]
    pub fn active_at(&self) -> OffsetDateTime {
        self.active_at
    }
}
