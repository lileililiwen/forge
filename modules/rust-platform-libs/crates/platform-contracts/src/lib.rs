//! Language-neutral platform contract envelopes, payload types, and
//! the compatibility validator.
//!
//! This crate is the Rust side of the `platform-contracts` repository.
//! It does not own the schemas (the language-neutral repository is the
//! single source of truth) and it does not own any persistence, HTTP,
//! or provider integration. The application is responsible for keeping
//! the canonical Forge `contracts/` directory reachable; standalone
//! consumers can set `PLATFORM_CONTRACTS_DIR`. See
//! [`registry::ContractRegistry::load`] and the
//! [`validate`] module for the runtime contract.
//!
//! The foundation rule "match language-neutral contracts" is enforced
//! by reading the JSON Schemas published by the `platform-contracts`
//! directory and validating every payload against them. Tests under
//! `tests/` use the exact `PLATFORM_CONTRACTS_DIR` override when set,
//! otherwise they resolve Forge's canonical `contracts/` directory from
//! this nested workspace. If the selected root is not reachable the test
//! fails with a clear next-action message; missing evidence never passes.

#![forbid(unsafe_code)]

pub mod envelope;
pub mod payload;
pub mod registry;
pub mod validate;

pub use envelope::{parse_contract_identifier, ContractIdError, ContractVersion, Envelope};
pub use payload::{
    AuditActor, AuditCategory, AuditEvent, AuditOutcome, Authentication, AuthenticationMethod,
    Capability, Environment, EvidenceKind, GateCheck, GateResult, GateResultValue, IdentitySubject,
    IsolationLevel, JobOutcome, JobResult, PermissionDecision, PermissionReason, Readiness,
    ReadinessCheck, ReadinessStatus, ReleaseEvidence, Stability, SubjectType, TenantContext,
};
pub use registry::{ContractEntry, ContractRegistry, RegistryError};
pub use validate::{
    validate_document, validate_payload, ValidationError, ValidationResult, ValidationWarning,
};
