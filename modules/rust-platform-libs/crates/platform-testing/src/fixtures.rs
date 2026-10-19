//! Test-only re-exports of canonical fixture documents.
//!
//! The fixtures are loaded from the **live** `platform-contracts`
//! repository at runtime (see `platform-contracts::tests` for the
//! canonical walker). This module simply re-exports the most common
//! types so a test can `use platform_testing::fixtures::*;` and get
//! everything it needs to build a value, serialize it, and assert it
//! round-trips.

pub use platform_audit::sink::InMemoryAuditSink;
pub use platform_contracts::payload::identity_subject::IdentitySubject as IdentitySubjectFixture;
pub use platform_contracts::payload::permission_decision::PermissionDecision as PermissionDecisionFixture;
pub use platform_contracts::payload::tenant_context::TenantContext as TenantContextFixture;
pub use platform_contracts::Envelope;
