//! Redacted context: the only kind of structured detail that the
//! audit crate accepts in a payload.
//!
//! [`RedactedContext`] is a `BTreeMap<String, String>` (or value) that
//! is rejected at construction time if any key matches a registered
//! secret substring. Producers can therefore build a context freely
//! without worrying about accidentally leaking a credential name
//! into a downstream log.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Default list of secret field-name substrings.
///
/// This list is a defensive baseline; the canonical list lives in the
/// `platform-contracts` repository's `schemas/registry.json`. The
/// audit crate re-checks against the live registry at validation
/// time. The local list lets the type reject obvious cases at
/// construction time, before the event ever reaches a sink.
pub const DEFAULT_SECRET_SUBSTRINGS: &[&str] = &[
    "password",
    "passwd",
    "token",
    "secret",
    "apikey",
    "api_key",
    "credential",
    "private_key",
    "privatekey",
    "bearer",
    "session_ke",
];

/// Error returned when a context key matches a secret substring.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("context key {key:?} matches secret substring {needle:?}; remove or rename the field")]
pub struct RedactedContextError {
    /// Key that triggered the rejection.
    pub key: String,
    /// Substring that matched.
    pub needle: String,
}

/// A context payload that has been checked against the redaction
/// policy.
///
/// The context is a flat string-to-string map. Values are coerced to
/// strings at construction; structured payloads belong on the
/// `AuditEvent::details` field, which is checked against the live
/// registry's secret substrings by the `platform-contracts`
/// validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RedactedContext(BTreeMap<String, String>);

impl RedactedContext {
    /// Construct an empty context.
    #[must_use]
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Construct a context from a key-value iterator, rejecting any
    /// key that matches a secret substring.
    ///
    /// # Errors
    ///
    /// Returns [`RedactedContextError`] on the first offending key.
    pub fn from_iter<I>(iter: I) -> Result<Self, RedactedContextError>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut map = BTreeMap::new();
        for (key, value) in iter {
            Self::check_key(&key)?;
            map.insert(key, value);
        }
        Ok(Self(map))
    }

    /// Insert a key-value pair, rejecting secret-named keys.
    ///
    /// # Errors
    ///
    /// Returns [`RedactedContextError`] when the key matches a
    /// secret substring.
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), RedactedContextError> {
        let key = key.into();
        Self::check_key(&key)?;
        self.0.insert(key, value.into());
        Ok(())
    }

    /// Borrow the inner map.
    #[must_use]
    pub fn as_map(&self) -> &BTreeMap<String, String> {
        &self.0
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True if the context has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn check_key(key: &str) -> Result<(), RedactedContextError> {
        let lowered = key.to_lowercase();
        for needle in DEFAULT_SECRET_SUBSTRINGS {
            if lowered.contains(needle) {
                return Err(RedactedContextError {
                    key: key.to_string(),
                    needle: (*needle).to_string(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_non_secret_keys() {
        let ctx = RedactedContext::from_iter([
            ("request_id".to_string(), "req-1".to_string()),
            ("client_hint".to_string(), "desktop-app".to_string()),
        ])
        .unwrap();
        assert_eq!(ctx.len(), 2);
    }

    #[test]
    fn rejects_password_key() {
        let err = RedactedContext::from_iter([("password".to_string(), "hunter2".to_string())])
            .unwrap_err();
        assert_eq!(err.key, "password");
        assert_eq!(err.needle, "password");
    }

    #[test]
    fn rejects_api_key_key() {
        let err =
            RedactedContext::from_iter([("api_key".to_string(), "abc".to_string())]).unwrap_err();
        assert_eq!(err.key, "api_key");
        assert_eq!(err.needle, "api_key");
    }

    #[test]
    fn insert_rejects_secret_key() {
        let mut ctx = RedactedContext::new();
        assert!(ctx.insert("ok", "value").is_ok());
        assert!(ctx.insert("bearer_token", "abc").is_err());
    }
}
