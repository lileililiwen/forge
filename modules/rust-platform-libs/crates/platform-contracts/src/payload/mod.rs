//! Per-family payload types.
//!
//! These types mirror the JSON Schemas published by the
//! `platform-contracts` repository. They are not a substitute for
//! schema validation; the [`crate::validate`] module is the source of
//! truth for wire-format compatibility. The Rust types exist to:
//!
//! * give producers a typed builder and a typed serializer,
//! * make the Rust API explicit about which fields are required,
//! * make consumer code easier to read than `serde_json::Value`.
//!
//! Adding a field to a payload is a minor bump on the corresponding
//! contract family; changing the type or removing a field is a
//! breaking change.

pub mod audit_event;
pub mod capability;
pub mod gate_result;
pub mod identity_subject;
pub mod job_outcome;
pub mod permission_decision;
pub mod readiness;
pub mod release_evidence;
pub mod tenant_context;

pub use audit_event::{AuditActor, AuditCategory, AuditEvent, AuditOutcome};
pub use capability::{Capability, Stability};
pub use gate_result::{GateCheck, GateResult, GateResultValue};
pub use identity_subject::{Authentication, AuthenticationMethod, IdentitySubject, SubjectType};
pub use job_outcome::{JobOutcome, JobResult};
pub use permission_decision::{PermissionDecision, PermissionReason};
pub use readiness::{Readiness, ReadinessCheck, ReadinessStatus};
pub use release_evidence::{EvidenceKind, ReleaseEvidence};
pub use tenant_context::{Environment, IsolationLevel, TenantContext};

/// Optional RFC3339 timestamp string carried in payloads.
///
/// The contract schemas use the `date-time` JSON Schema format. We
/// store the raw string so producers can pick the exact serialization
/// the runtime wants (`time::OffsetDateTime`'s RFC3339 form, etc.).
pub type Timestamp = String;

/// Optional bounded subject identifier (max 128 chars per the
/// language-neutral schema for `subject_id`, `tenant_id`, etc.).
pub type Identifier128 = String;

/// Optional bounded tenant identifier (max 64 chars).
pub type Identifier64 = String;

/// Marker trait for payload types that can be wrapped in an envelope.
pub trait ContractPayload {
    /// Family identifier, e.g. `platform.identity-subject`.
    fn family() -> &'static str;
}

impl ContractPayload for IdentitySubject {
    fn family() -> &'static str {
        "platform.identity-subject"
    }
}

impl ContractPayload for PermissionDecision {
    fn family() -> &'static str {
        "platform.permission-decision"
    }
}

impl ContractPayload for TenantContext {
    fn family() -> &'static str {
        "platform.tenant-context"
    }
}

impl ContractPayload for AuditEvent {
    fn family() -> &'static str {
        "platform.audit-event"
    }
}

impl ContractPayload for Readiness {
    fn family() -> &'static str {
        "platform.readiness"
    }
}

impl ContractPayload for GateResult {
    fn family() -> &'static str {
        "platform.gate-result"
    }
}

impl ContractPayload for JobOutcome {
    fn family() -> &'static str {
        "platform.job-outcome"
    }
}

impl ContractPayload for ReleaseEvidence {
    fn family() -> &'static str {
        "platform.release-evidence"
    }
}

impl ContractPayload for Capability {
    fn family() -> &'static str {
        "platform.capability"
    }
}
