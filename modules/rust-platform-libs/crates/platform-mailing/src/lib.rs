//! Provider-neutral mail send contract.
//!
//! `platform-mailing` is a satellite crate: it depends on
//! `platform-core` (error categories, clocks, correlation IDs,
//! redaction) and exposes one typed send contract plus two
//! implementations:
//!
//! * [`MailTransport`] — the async send trait every consumer injects.
//! * `mock::MockTransport` (feature `mock`) — scripted, network-free
//!   outcomes for consumer test suites. It lives in this crate so
//!   production crates never touch `platform-testing`.
//! * `smtp::LettreSmtpTransport` (feature `smtp-lettre`) — the opt-in
//!   lettre-based SMTP adapter. Default features are empty, so the
//!   crate compiles lettre-free unless the consumer opts in.
//!
//! Validation always runs before any transport call, and every
//! [`MailingError`] maps onto an existing
//! [`platform_core::ErrorCategory`] without leaking credentials,
//! message bodies, or recipient local-parts.

#![forbid(unsafe_code)]

pub mod error;
pub mod transport;
pub mod types;

#[cfg(feature = "mock")]
pub mod mock;

#[cfg(feature = "smtp-lettre")]
pub mod smtp;

pub use error::MailingError;
pub use transport::{DeliveryReceipt, MailTransport, SendContext};
pub use types::{AttachmentRef, Body, EmailAddress, EmailMessage};
