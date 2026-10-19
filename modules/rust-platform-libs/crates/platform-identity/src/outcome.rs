//! Outcome wrapper used by identity-layer entry points.
//!
//! [`IdentityOutcome<T>`] is a thin wrapper that distinguishes
//! "no subject" (anonymous) from "subject resolved" in a way that is
//! distinct from the [`Result`] type so that a missing subject is
//! not accidentally conflated with an authentication failure.

use crate::error::IdentityError;

/// Outcome of an identity lookup.
///
/// * `Authenticated(value)` — a subject was resolved.
/// * `Unauthenticated(reason)` — no subject could be resolved, but
///   the lookup did not fail (e.g. anonymous request).
/// * `Failed(error)` — the lookup failed for a non-auth reason
///   (provider unavailable, configuration error, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityOutcome<T> {
    /// Subject resolved.
    Authenticated(T),
    /// No subject present; the request continues as anonymous.
    Unauthenticated(UnauthenticatedReason),
    /// Lookup failed.
    Failed(IdentityError),
}

/// Reason why no subject is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnauthenticatedReason {
    /// No credentials were supplied.
    NoCredentials,
    /// The credentials were presented but did not authenticate.
    InvalidCredentials,
    /// The session or token has expired.
    Expired,
}

impl<T> IdentityOutcome<T> {
    /// Returns true iff a subject was resolved.
    #[must_use]
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Self::Authenticated(_))
    }

    /// Convert into a [`Result`] by treating `Unauthenticated` as
    /// an `InvalidCredentials` error.
    ///
    /// Use this when the caller MUST have a subject (e.g. an
    /// authenticated endpoint).
    pub fn require_authenticated(self) -> Result<T, IdentityError> {
        match self {
            Self::Authenticated(value) => Ok(value),
            Self::Unauthenticated(reason) => {
                Err(IdentityError::new(reason.into(), "authentication required"))
            }
            Self::Failed(err) => Err(err),
        }
    }
}

impl From<UnauthenticatedReason> for crate::error::IdentityFailure {
    fn from(reason: UnauthenticatedReason) -> Self {
        match reason {
            UnauthenticatedReason::NoCredentials | UnauthenticatedReason::InvalidCredentials => {
                Self::InvalidCredentials
            }
            UnauthenticatedReason::Expired => Self::Expired,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_authenticated_promotes_unauthenticated() {
        let outcome: IdentityOutcome<()> =
            IdentityOutcome::Unauthenticated(UnauthenticatedReason::NoCredentials);
        let err = outcome.require_authenticated().unwrap_err();
        assert_eq!(
            err.failure(),
            crate::error::IdentityFailure::InvalidCredentials
        );
    }

    #[test]
    fn require_authenticated_passes_through_authenticated() {
        let outcome: IdentityOutcome<u32> = IdentityOutcome::Authenticated(42);
        assert_eq!(outcome.require_authenticated().unwrap(), 42);
    }
}
