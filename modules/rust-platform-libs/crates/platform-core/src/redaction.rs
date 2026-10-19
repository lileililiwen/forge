//! Redaction primitives.
//!
//! [`Secret<T>`] wraps a value of any type and renders `<redacted>`
//! through `Debug` and `Display` so secret material cannot leak via
//! `format!`, `unwrap` panic messages, or `dbg!` macros. The wrapped
//! value is accessible via [`Secret::expose`] only inside an explicit
//! `expose()` call, which is easy to grep for in a security review.

use std::fmt;

/// Wrapper that redacts the inner value in formatters.
///
/// `Secret<T>` implements `Debug` and `Display` as `<redacted>`. The
/// inner value is reachable only through [`Secret::expose`], which
/// is named verbosely so reviewers can find every use with `grep`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret<T> {
    inner: T,
}

impl<T> Secret<T> {
    /// Wrap a value, taking ownership.
    #[must_use]
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Explicitly expose the inner value. The call site is the audit
    /// trail: every secret read goes through `expose()`.
    #[must_use]
    pub fn expose(&self) -> &T {
        &self.inner
    }

    /// Consume the wrapper and return the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: fmt::Debug> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T: serde::Serialize> serde::Serialize for Secret<T> {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        // Serializing a Secret as JSON would put the secret on the wire.
        // Reject with a serialization error so the developer must make
        // an explicit decision (e.g. implement Serialize by hand or
        // call `expose()` first).
        use serde::ser::Error as _;
        Err(S::Error::custom(
            "Secret<T> cannot be serialized; call expose() and serialize the inner value deliberately",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_inner() {
        let secret = Secret::new("hunter2");
        assert_eq!(format!("{secret:?}"), "<redacted>");
        assert!(!format!("{secret:?}").contains("hunter2"));
    }

    #[test]
    fn display_redacts_inner() {
        let secret = Secret::new("hunter2");
        assert_eq!(format!("{secret}"), "<redacted>");
    }

    #[test]
    fn expose_returns_inner() {
        let secret = Secret::new(String::from("hunter2"));
        assert_eq!(secret.expose(), "hunter2");
    }

    #[test]
    fn serialize_is_rejected() {
        let secret = Secret::new("hunter2");
        let result = serde_json::to_string(&secret);
        assert!(result.is_err());
    }
}
