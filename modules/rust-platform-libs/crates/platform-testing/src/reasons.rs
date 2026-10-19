//! Assertion helpers.

/// Default list of secret substrings. The list mirrors the
/// `platform-contracts` `schemas/registry.json` `secret_field_substrings`
/// baseline.
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

/// Assert that `value` does not contain any of the registered secret
/// substrings. Panics with a clear message on the first match.
pub fn assert_redacted(value: &str) {
    let lowered = value.to_lowercase();
    for needle in DEFAULT_SECRET_SUBSTRINGS {
        if lowered.contains(needle) {
            panic!(
                "assert_redacted: value contains secret substring {needle:?}; \
                 remove or rename before persisting or forwarding"
            );
        }
    }
}

#[macro_export]
/// Assert that a serialized form of a value does not leak secret
/// substrings. The value is rendered with `serde_json::to_string`,
/// so the input must implement `Serialize`.
macro_rules! assert_redacted_json {
    ($value:expr) => {{
        let rendered = match serde_json::to_string(&$value) {
            Ok(text) => text,
            Err(err) => panic!("assert_redacted_json: cannot serialize value: {err}"),
        };
        $crate::assert_redacted(&rendered);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_safe_value() {
        assert_redacted("hello world");
    }

    #[test]
    #[should_panic(expected = "contains secret substring")]
    fn rejects_password() {
        assert_redacted("the password is hunter2");
    }
}
