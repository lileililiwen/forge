//! Identity provider-neutral traits.
//!
//! The application implements these traits against its own store,
//! hashing policy, and provider integration. The platform guarantees
//! only the trait shape and the safety rules documented in
//! `docs/ownership.md` of the `platform-contracts` repository.

use platform_core::correlation::CorrelationId;

use crate::outcome::IdentityOutcome;
use crate::session::Session;
use crate::subject::Subject;
use crate::tenant::TenantContext;

/// Application-implemented subject lookup.
///
/// The platform does not own a subject store; the application
/// supplies one. The platform guarantees that no
/// `IdentityError` returned by this trait contains a password,
/// token, hash, or provider response detail.
pub trait IdentityStore: Send + Sync {
    /// Resolve a subject from a session reference (e.g. cookie,
    /// bearer token, header value). The session reference is opaque
    /// to the platform; the application is responsible for parsing
    /// the transport-specific representation.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] on any failure. The error MUST NOT
    /// carry credential material.
    fn load_subject(
        &self,
        session_ref: &str,
        correlation_id: CorrelationId,
    ) -> IdentityOutcome<Subject>;

    /// Resolve a session and its subject from a session reference.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] on any failure. The error MUST NOT
    /// carry credential material.
    fn load_session(
        &self,
        session_ref: &str,
        correlation_id: CorrelationId,
    ) -> IdentityOutcome<Session>;
}

/// Application-implemented identity provider integration.
///
/// The platform does not own a provider integration; the application
/// supplies one. The trait is intentionally minimal — it only
/// authenticates a credential bundle and returns a subject. The
/// "credential bundle" type is a string slice the application parses
/// (e.g. `Basic <base64>`, a bearer token, a SAML assertion). The
/// platform does not dictate the transport.
pub trait IdentityProvider: Send + Sync {
    /// Authenticate a credential bundle. On success, returns the
    /// resolved subject.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] on any failure. The error MUST NOT
    /// carry credential material.
    fn authenticate(
        &self,
        credential: &str,
        correlation_id: CorrelationId,
    ) -> IdentityOutcome<Subject>;
}

/// Application-implemented accessor for the current request's
/// subject.
///
/// The platform does not know how a web framework exposes the
/// authenticated subject; the application supplies an adapter that
/// reads it from the request context. Adapter implementations
/// typically delegate to an [`IdentityStore`].
pub trait CurrentSubject: Send + Sync {
    /// Resolve the current request's subject.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] on any failure. The error MUST NOT
    /// carry credential material.
    fn current_subject(&self, correlation_id: CorrelationId) -> IdentityOutcome<Subject>;

    /// Resolve the current request's tenant context.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] on any failure.
    fn current_tenant(&self, correlation_id: CorrelationId) -> IdentityOutcome<TenantContext>;
}
