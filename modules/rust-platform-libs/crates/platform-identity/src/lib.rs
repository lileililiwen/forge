//! Provider-neutral identity, session, permission, tenant, and
//! lifecycle contracts.
//!
//! This crate is the **trait surface** for identity in the platform
//! foundation. It does not own:
//!
//! * user records, password hashes, or any credential material,
//! * session storage, token transport, or cookie handling,
//! * OIDC, OAuth 2.0, SAML, or any provider-specific integration,
//! * HTTP, Axum, SQLx, or any database driver,
//! * migration tooling.
//!
//! Applications own those concerns and implement the traits in this
//! crate against their own store, hashing policy, and provider. The
//! platform guarantees the trait shape and the safety rules — in
//! particular, that an [`IdentityError`] never carries a password,
//! token, hash, or provider response detail.
//!
//! All traits are synchronous. Asynchronous adapters can wrap the
//! call in a runtime of their choice; the foundation does not depend
//! on a specific executor.

#![forbid(unsafe_code)]

pub mod error;
pub mod evaluator;
pub mod lifecycle;
pub mod outcome;
pub mod permission;
pub mod session;
pub mod subject;
pub mod tenant;
pub mod tenant_resolver;
pub mod traits;

pub use error::{IdentityError, IdentityFailure};
pub use evaluator::{
    AllowListEvaluator, DenyByDefaultEvaluator, PermissionError, PermissionEvaluation,
    PermissionEvaluator,
};
pub use lifecycle::Lifecycle;
pub use outcome::IdentityOutcome;
pub use permission::{Permission, PermissionDecision, PermissionReason};
pub use session::Session;
pub use subject::{
    Authentication, AuthenticationMethod, Subject, SubjectId, SubjectSnapshot, SubjectType,
};
pub use tenant::{Environment, IsolationLevel, Tenant, TenantContext, TenantId};
pub use tenant_resolver::{NoTenantResolver, StaticTenantResolver, TenantResolver};
pub use traits::{CurrentSubject, IdentityProvider, IdentityStore};
