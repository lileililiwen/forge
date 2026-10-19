//! Typed mailing failures mapped onto the frozen `ErrorCategory`.
//!
//! This change MUST NOT extend [`platform_core::ErrorCategory`]
//! (task ledger L3). Every variant below maps to a shipped category
//! via [`MailingError::category`]:
//!
//! * `Validation`, `Configuration` → [`ErrorCategory::Validation`]
//! * `Authentication` → [`ErrorCategory::Authentication`]
//! * `TransportUnavailable`, `Timeout`, `Tls`, `RejectedByServer` →
//!   [`ErrorCategory::Unavailable`]
//!
//! `Display` is redaction-audited: it never renders credentials,
//! message bodies, or recipient local-parts. Callers MUST scrub
//! server-supplied strings with [`scrub_addresses`] before storing
//! them in an error.

use std::fmt;
use std::time::Duration;

use platform_core::{ErrorCategory, SafeError, SafeErrorKind};

/// Typed failure for every mail send path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailingError {
    /// Message or address failed validation; the transport was never
    /// invoked. Detail MUST already be scrubbed at construction.
    Validation(String),
    /// Adapter configuration is unusable (bad host, insecure mode
    /// off-loopback, broken client state). Detail MUST be scrubbed.
    Configuration(String),
    /// The server or network is unreachable (DNS, refused, reset,
    /// 4xx transient). Detail MUST be scrubbed.
    TransportUnavailable {
        /// Scrubbed human-readable cause.
        reason: String,
    },
    /// The server rejected the credentials. Carries no detail at
    /// all: authentication failures MUST NOT echo server text, which
    /// routinely contains usernames.
    Authentication,
    /// The send exceeded its deadline. No retry is attempted.
    Timeout {
        /// The deadline that was exceeded.
        after: Duration,
    },
    /// TLS negotiation failed. Carries no rustls detail: TLS alerts
    /// can contain hostnames and handshake internals.
    Tls,
    /// The server permanently rejected the message (5xx). The code
    /// and the scrubbed server text are preserved for operators.
    RejectedByServer {
        /// SMTP reply code, when the server supplied one.
        code: Option<u16>,
        /// Scrubbed server-supplied reason.
        reason: String,
    },
}

impl MailingError {
    /// Build a validation failure. The detail is stored verbatim, so
    /// callers MUST NOT pass addresses, bodies, or secrets here.
    #[must_use]
    pub fn validation(detail: impl Into<String>) -> Self {
        Self::Validation(detail.into())
    }

    /// Build a configuration failure. Same scrubbing duty as
    /// [`validation`](Self::validation).
    #[must_use]
    pub fn configuration(detail: impl Into<String>) -> Self {
        Self::Configuration(detail.into())
    }

    /// Build a transport-unavailable failure from server text,
    /// scrubbing any `local@domain` material to `...@domain`.
    #[must_use]
    pub fn transport_unavailable(reason: impl Into<String>) -> Self {
        Self::TransportUnavailable {
            reason: scrub_addresses(&reason.into()),
        }
    }

    /// Build a rejection failure from server text, scrubbing any
    /// `local@domain` material.
    #[must_use]
    pub fn rejected_by_server(code: Option<u16>, reason: impl Into<String>) -> Self {
        Self::RejectedByServer {
            code,
            reason: scrub_addresses(&reason.into()),
        }
    }

    /// Build a timeout failure for the given deadline.
    #[must_use]
    pub fn timeout(after: Duration) -> Self {
        Self::Timeout { after }
    }

    /// The existing [`ErrorCategory`] this variant maps to. The enum
    /// in `platform-core` stays frozen; this function is the total
    /// mapping (no wildcard arm, so adding a variant breaks the
    /// build until the mapping is extended).
    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::Validation(_) | Self::Configuration(_) => ErrorCategory::Validation,
            Self::Authentication => ErrorCategory::Authentication,
            Self::TransportUnavailable { .. }
            | Self::Timeout { .. }
            | Self::Tls
            | Self::RejectedByServer { .. } => ErrorCategory::Unavailable,
        }
    }

    /// Convert into the platform [`SafeError`] model, preserving the
    /// category and the already-scrubbed detail.
    #[must_use]
    pub fn to_safe_error(&self) -> SafeError {
        SafeError::adapter(self.category(), self.to_string())
    }

    /// The [`SafeErrorKind`] is always adapter-originated: mailing
    /// failures come from the send path, never from the foundation.
    #[must_use]
    pub const fn kind(&self) -> SafeErrorKind {
        SafeErrorKind::Adapter
    }
}

impl fmt::Display for MailingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(detail) | Self::Configuration(detail) => {
                write!(f, "{detail}")
            }
            Self::TransportUnavailable { reason } => {
                write!(f, "mail transport unavailable: {reason}")
            }
            Self::Authentication => f.write_str("mail server rejected the credentials"),
            Self::Timeout { after } => {
                write!(f, "mail send timed out after {}s", after.as_secs())
            }
            Self::Tls => f.write_str("mail transport TLS negotiation failed"),
            Self::RejectedByServer { code, reason } => match code {
                Some(code) => {
                    write!(f, "mail server rejected the message ({code}): {reason}")
                }
                None => write!(f, "mail server rejected the message: {reason}"),
            },
        }
    }
}

impl std::error::Error for MailingError {}

/// Scrub `local@domain` occurrences to `...@domain` so server text
/// and caller strings can be stored in errors without leaking
/// recipient local-parts. Runs of `A–Z a–z 0–9 . _ % + -` before an
/// `@` are treated as the local part.
#[must_use]
pub fn scrub_addresses(text: &str) -> String {
    const LOCAL: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._%+-";
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            // Walk back over a locally-remembered run only if the
            // output currently ends with local-part characters.
            let mut back = out.len();
            while back > 0 && LOCAL.contains(&out.as_bytes()[back - 1]) {
                back -= 1;
            }
            // Only scrub when a non-empty local part directly precedes
            // the `@` and the character before it is a boundary (start
            // of string or a non-local character, e.g. space, `<`,
            // `:`). This avoids mangling lone `@` signs.
            if back < out.len() {
                out.truncate(back);
                out.push_str("...@");
                i += 1;
                continue;
            }
            out.push('@');
            i += 1;
            continue;
        }
        // Push the next UTF-8 scalar value verbatim.
        let ch = text[i..].chars().next().expect("char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_covers_every_variant_without_wildcard() {
        // One assertion per variant: adding a variant without
        // extending `category()` fails to compile (no wildcard arm),
        // and this table fails until the new row is classified.
        let cases: &[(MailingError, ErrorCategory)] = &[
            (MailingError::validation("bad"), ErrorCategory::Validation),
            (
                MailingError::configuration("bad"),
                ErrorCategory::Validation,
            ),
            (
                MailingError::transport_unavailable("down"),
                ErrorCategory::Unavailable,
            ),
            (MailingError::Authentication, ErrorCategory::Authentication),
            (
                MailingError::timeout(Duration::from_secs(3)),
                ErrorCategory::Unavailable,
            ),
            (MailingError::Tls, ErrorCategory::Unavailable),
            (
                MailingError::rejected_by_server(Some(550), "nope"),
                ErrorCategory::Unavailable,
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.category(), *expected, "mapping for {err}");
        }
    }

    #[test]
    fn display_omits_local_parts_and_caller_text_stays_static() {
        // Server-supplied text is scrubbed of local-parts at
        // construction; free-text variants (Validation/Configuration)
        // store caller detail verbatim, so callers MUST pass only
        // static text there — enforced by review, not by runtime.
        let rendered = format!(
            "{} | {} | {} | {} | {} | {}",
            MailingError::validation("static detail"),
            MailingError::transport_unavailable(
                "dial failed for relay of alice@example.com".to_string()
            ),
            MailingError::Authentication,
            MailingError::timeout(Duration::from_secs(30)),
            MailingError::Tls,
            MailingError::rejected_by_server(
                Some(550),
                "mailbox unavailable: bob@example.org".to_string()
            ),
        );
        assert!(
            !rendered.contains("alice@") && !rendered.contains("bob@"),
            "local-part leaked: {rendered}"
        );
        assert!(
            rendered.contains("...@example.com") && rendered.contains("...@example.org"),
            "scrubbed domain missing: {rendered}"
        );
        // The auth and TLS variants carry no server text at all.
        assert_eq!(
            MailingError::Authentication.to_string(),
            "mail server rejected the credentials"
        );
        assert_eq!(
            MailingError::Tls.to_string(),
            "mail transport TLS negotiation failed"
        );
    }

    #[test]
    fn scrub_leaves_plain_text_alone() {
        assert_eq!(scrub_addresses("connection refused"), "connection refused");
        assert_eq!(scrub_addresses("5.7.1 relay denied"), "5.7.1 relay denied");
        assert_eq!(
            scrub_addresses("<alice@example.com>, <bob@example.org>"),
            "<...@example.com>, <...@example.org>"
        );
    }

    #[test]
    fn safe_error_preserves_category() {
        let err = MailingError::rejected_by_server(Some(550), "denied");
        let safe = err.to_safe_error();
        assert_eq!(safe.category(), ErrorCategory::Unavailable);
        assert_eq!(safe.kind(), SafeErrorKind::Adapter);
    }
}
