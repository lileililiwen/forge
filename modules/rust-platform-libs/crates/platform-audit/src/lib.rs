//! Normalized audit event types and sink trait.
//!
//! The audit crate is intentionally small: it defines the event
//! shape that consumers can serialize, the redaction policy that
//! every detail field MUST respect, and the sink trait that the
//! application implements to forward events to its storage of
//! choice.
//!
//! The platform does not own:
//!
//! * audit storage (a database table, a log file, a SIEM
//!   integration, ...),
//! * audit transport (HTTP, gRPC, message queue, ...),
//! * retention policy or redaction-at-rest rules.
//!
//! The application supplies a sink implementation and decides where
//! the events go. The platform guarantees only the event shape and
//! the redaction policy.

#![forbid(unsafe_code)]

pub mod context;
pub mod event;
pub mod producers;
pub mod sink;

pub use context::{RedactedContext, RedactedContextError};
pub use event::{
    AuditActor, AuditCategory, AuditEvent, AuditEventBuilder, AuditOutcome, AuditSubjectType,
};
pub use producers::{AuthenticationAuditor, PermissionAuditor, SessionAuditor};
pub use sink::{AuditSink, NoopAuditSink, SinkError};
