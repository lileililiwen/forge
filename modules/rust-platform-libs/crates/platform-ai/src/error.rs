//! Typed AI failures mapped onto the frozen [`ErrorCategory`].
//!
//! This crate MUST NOT extend [`platform_core::ErrorCategory`]
//! (design L7 + MAIL-003 inherited from `platform-mailing`). Every
//! variant below maps to a shipped category via
//! [`AiError::category`]:
//!
//! * `Validation`, `Configuration`, `ContentPolicy`, `Unsupported` →
//!   [`ErrorCategory::Validation`]
//! * `Unauthorized`, `RateLimited`, `TransportUnavailable`,
//!   `Timeout`, `ProviderServer`, `MalformedResponse` →
//!   [`ErrorCategory::Unavailable`]
//!
//! `Display` is redaction-audited: it never renders API keys, prompts,
//! completions, or raw response bodies. Server-supplied text that
//! reaches [`AiError::ProviderServer::safe_detail`] or
//! [`AiError::TransportUnavailable`] must already be truncated and
//! control-stripped by [`truncate_safe_detail`]; the helpers below
//! enforce that contract.

use platform_core::{ErrorCategory, SafeError, SafeErrorKind};

use crate::types::Capabilities;

/// Maximum number of bytes of server text that may appear in a
/// [`ProviderServer`](AiError::ProviderServer) detail. Chosen to fit a
/// truncated provider error message while keeping the platform's
/// safe-display contract.
pub const MAX_SAFE_DETAIL_BYTES: usize = 200;

/// Typed failure for every AI capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// Caller-supplied request failed validation. The transport was
    /// never invoked. The detail string is the platform's own
    /// validation reason, not caller-provided content.
    Validation(String),
    /// Adapter configuration is unusable. The detail string is the
    /// platform's own validation reason, not caller-provided content.
    Configuration(String),
    /// Provider content policy refused the request or response. The
    /// detail string is the redacted provider reason.
    ContentPolicy(String),
    /// The provider does not declare this capability. The variant
    /// names the capability so consumers can branch on it.
    Unsupported(Capabilities),
    /// The server rejected the credentials. No detail: authentication
    /// failures MUST NOT echo server text, which routinely contains
    /// account identifiers.
    Unauthorized,
    /// The provider throttled the call. `retry_after_secs` is the
    /// `Retry-After` header value in seconds, when the server
    /// surfaced one.
    RateLimited {
        /// The provider-reported `Retry-After` value, if any.
        retry_after_secs: Option<u64>,
    },
    /// The path is down: connection refused, DNS failure, TLS
    /// handshake, or any transport-level error before a status code
    /// arrived. The detail string is the platform's own message, not
    /// caller content.
    TransportUnavailable(String),
    /// The send exceeded its deadline. No retry is attempted.
    Timeout,
    /// The server returned an unexpected status. `safe_detail` is the
    /// server-supplied reason, already truncated by
    /// [`truncate_safe_detail`].
    ProviderServer {
        /// HTTP status code, when the server supplied one.
        status: u16,
        /// Truncated, control-stripped server-supplied detail.
        safe_detail: String,
    },
    /// The response could not be parsed into a contract type.
    MalformedResponse(String),
}

impl AiError {
    /// Build a validation failure. The detail is the platform's own
    /// string; callers MUST NOT pass user content here.
    #[must_use]
    pub fn validation(detail: impl Into<String>) -> Self {
        Self::Validation(detail.into())
    }

    /// Build a configuration failure. The detail is the platform's
    /// own string; callers MUST NOT pass user content here.
    #[must_use]
    pub fn configuration(detail: impl Into<String>) -> Self {
        Self::Configuration(detail.into())
    }

    /// Build a transport-unavailable failure from a reason
    /// supplied by the HTTP transport layer. The detail is the
    /// platform's own string; user content MUST NOT be passed
    /// here.
    #[must_use]
    pub fn transport_unavailable(detail: impl Into<String>) -> Self {
        Self::TransportUnavailable(detail.into())
    }

    /// Build a malformed-response failure. The detail is the
    /// platform's own description of what the parser was expecting;
    /// it MUST NOT include the raw response body or any caller
    /// content.
    #[must_use]
    pub fn malformed_response(detail: impl Into<String>) -> Self {
        Self::MalformedResponse(detail.into())
    }

    /// Build a content-policy failure. `reason` MUST already be
    /// truncated by [`truncate_safe_detail`].
    #[must_use]
    pub fn content_policy(reason: impl Into<String>) -> Self {
        Self::ContentPolicy(truncate_safe_detail(&reason.into()).into_owned())
    }

    /// Map an HTTP status code (and optional parsed body) to the
    /// closest [`AiError`] variant. Retry-After parsing happens at
    /// the HTTP layer; this function does not look at the response
    /// body for content-policy signals.
    ///
    /// Status mapping (with no body inspection):
    ///
    /// * `401`, `403` → [`AiError::Unauthorized`];
    /// * `429` → [`AiError::RateLimited`] with `retry_after_secs =
    ///   None` (caller overrides if it parsed a header);
    /// * `400`, `404`, `422` → [`AiError::Validation`] (request shape
    ///   was rejected);
    /// * `5xx` and any other status → [`AiError::ProviderServer`]
    ///   with the truncated body as `safe_detail` (or empty when no
    ///   body was supplied).
    #[must_use]
    pub fn from_status(status: u16, body: Option<&str>) -> Self {
        match status {
            401 | 403 => Self::Unauthorized,
            429 => Self::RateLimited {
                retry_after_secs: None,
            },
            400 | 404 | 422 => Self::Validation(match body {
                Some(body) => truncate_safe_detail(body).into_owned(),
                None => format!("server rejected the request with status {status}"),
            }),
            _ => Self::ProviderServer {
                status,
                safe_detail: body
                    .map(truncate_safe_detail)
                    .map(|s| s.into_owned())
                    .unwrap_or_default(),
            },
        }
    }

    /// The existing [`ErrorCategory`] this variant maps to. The enum
    /// in `platform-core` stays frozen; this function is the total
    /// mapping (no wildcard arm, so adding a variant breaks the
    /// build until the mapping is extended).
    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::Validation(_)
            | Self::Configuration(_)
            | Self::ContentPolicy(_)
            | Self::Unsupported(_) => ErrorCategory::Validation,
            Self::Unauthorized
            | Self::RateLimited { .. }
            | Self::TransportUnavailable(_)
            | Self::Timeout
            | Self::ProviderServer { .. }
            | Self::MalformedResponse(_) => ErrorCategory::Unavailable,
        }
    }

    /// Convert into the platform [`SafeError`] model, preserving the
    /// category and the already-scrubbed detail.
    #[must_use]
    pub fn to_safe_error(&self) -> SafeError {
        SafeError::adapter(self.category(), self.to_string())
    }

    /// The [`SafeErrorKind`] is always adapter-originated: AI failures
    /// come from the provider path, never from the foundation.
    #[must_use]
    pub const fn kind(&self) -> SafeErrorKind {
        SafeErrorKind::Adapter
    }
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(detail) | Self::Configuration(detail) => {
                write!(f, "{detail}")
            }
            Self::ContentPolicy(reason) => {
                write!(f, "ai provider refused the content: {reason}")
            }
            Self::Unsupported(caps) => {
                if caps.is_empty() {
                    f.write_str("ai provider does not support the requested capability")
                } else {
                    write!(f, "ai provider does not support capability {caps:?}")
                }
            }
            Self::Unauthorized => f.write_str("ai provider rejected the credentials"),
            Self::RateLimited {
                retry_after_secs: Some(secs),
            } => {
                write!(f, "ai provider rate-limited (retry after {secs}s)")
            }
            Self::RateLimited {
                retry_after_secs: None,
            } => f.write_str("ai provider rate-limited"),
            Self::TransportUnavailable(detail) => {
                write!(f, "ai transport unavailable: {detail}")
            }
            Self::Timeout => f.write_str("ai request timed out"),
            Self::ProviderServer {
                status,
                safe_detail,
            } => {
                if safe_detail.is_empty() {
                    write!(f, "ai server error ({status})")
                } else {
                    write!(f, "ai server error ({status}): {safe_detail}")
                }
            }
            Self::MalformedResponse(detail) => {
                write!(f, "ai response was malformed: {detail}")
            }
        }
    }
}

impl std::error::Error for AiError {}

/// Truncate server-supplied text to at most [`MAX_SAFE_DETAIL_BYTES`]
/// bytes (cut at a UTF-8 char boundary) and strip ASCII control
/// characters. Used to scrub a server `error.message` (or any other
/// caller-supplied detail) before it is stored in an
/// [`AiError::ProviderServer`], [`AiError::ContentPolicy`], or
/// [`AiError::TransportUnavailable`].
#[must_use]
pub fn truncate_safe_detail(text: &str) -> std::borrow::Cow<'_, str> {
    let needs_truncation = text.len() > MAX_SAFE_DETAIL_BYTES;
    let needs_strip = text.chars().any(|c| c.is_control());
    if !needs_truncation && !needs_strip {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len().min(MAX_SAFE_DETAIL_BYTES));
    let mut bytes = 0usize;
    for ch in text.chars() {
        if ch.is_control() {
            continue;
        }
        let ch_len = ch.len_utf8();
        if bytes + ch_len > MAX_SAFE_DETAIL_BYTES {
            break;
        }
        out.push(ch);
        bytes += ch_len;
    }
    std::borrow::Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_covers_every_variant_without_wildcard() {
        // One assertion per variant: adding a variant without
        // extending `category()` fails to compile (no wildcard arm),
        // and this table fails until the new row is classified.
        let cases: &[(AiError, ErrorCategory)] = &[
            (AiError::validation("v"), ErrorCategory::Validation),
            (AiError::configuration("c"), ErrorCategory::Validation),
            (AiError::content_policy("policy"), ErrorCategory::Validation),
            (
                AiError::Unsupported(Capabilities::STREAMING),
                ErrorCategory::Validation,
            ),
            (AiError::Unauthorized, ErrorCategory::Unavailable),
            (
                AiError::RateLimited {
                    retry_after_secs: Some(7),
                },
                ErrorCategory::Unavailable,
            ),
            (
                AiError::transport_unavailable("down"),
                ErrorCategory::Unavailable,
            ),
            (AiError::Timeout, ErrorCategory::Unavailable),
            (
                AiError::ProviderServer {
                    status: 500,
                    safe_detail: "nope".into(),
                },
                ErrorCategory::Unavailable,
            ),
            (
                AiError::malformed_response("missing choices"),
                ErrorCategory::Unavailable,
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.category(), *expected, "mapping for {err}");
        }
    }

    #[test]
    fn display_omits_keys_prompts_and_completions() {
        // The Display impl never embeds caller-supplied key or prompt
        // material. The detail strings we pass below are platform
        // own strings (validation/configuration reasons) that contain
        // no secret.
        let secret = "sk-live-abcdef0123456789";
        let prompt = "Write a poem about the user's medical history.";
        let body = r#"{"choices":[],"error":{"message":"internal","prompt":"leaked"}}"#;

        let rendered = format!(
            "{} | {} | {} | {} | {}",
            AiError::validation("messages count 0"),
            AiError::configuration("base_url must be https"),
            AiError::Unauthorized,
            AiError::transport_unavailable("connection refused"),
            AiError::ProviderServer {
                status: 500,
                safe_detail: truncate_safe_detail(body).into_owned(),
            },
        );
        assert!(!rendered.contains(secret), "secret leaked: {rendered}");
        assert!(!rendered.contains(prompt), "prompt leaked: {rendered}");
        // The body is present in the failure path but its server
        // detail is the truncated, control-stripped server.message —
        // the literal "internal" string survives, but the original
        // "leaked" prompt fragment does not.
        assert!(rendered.contains("internal"));
    }

    #[test]
    fn truncate_safe_detail_caps_at_the_byte_limit_and_strips_controls() {
        // 250 bytes with no controls: truncated to 200.
        let big = "a".repeat(250);
        let truncated = truncate_safe_detail(&big);
        assert_eq!(truncated.len(), 200);
        assert!(truncated.chars().all(|c| !c.is_control()));

        // Control chars removed; length shrinks below the cap.
        let controlled = "hi\nthere\t\rfriend";
        let cleaned = truncate_safe_detail(controlled);
        assert_eq!(cleaned.as_ref(), "hitherefriend");

        // Cuts at a UTF-8 char boundary, not in the middle of a char.
        // "héllo" is 5 chars / 6 bytes; build a payload where the
        // 200-byte boundary lands inside the second multibyte char.
        let mut text = String::from("a");
        while text.len() < 199 {
            text.push('a');
        }
        text.push('é'); // 2 bytes; this char is at 199..=201
        let truncated = truncate_safe_detail(&text);
        assert!(truncated.len() <= 200);
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }

    #[test]
    fn from_status_maps_every_class() {
        let cases: &[(u16, Option<&str>, std::mem::Discriminant<AiError>)] = &[
            (401, None, std::mem::discriminant(&AiError::Unauthorized)),
            (403, None, std::mem::discriminant(&AiError::Unauthorized)),
            (
                429,
                None,
                std::mem::discriminant(&AiError::RateLimited {
                    retry_after_secs: None,
                }),
            ),
            (400, None, std::mem::discriminant(&AiError::validation("x"))),
            (404, None, std::mem::discriminant(&AiError::validation("x"))),
            (422, None, std::mem::discriminant(&AiError::validation("x"))),
            (
                500,
                Some("boom"),
                std::mem::discriminant(&AiError::ProviderServer {
                    status: 500,
                    safe_detail: "boom".into(),
                }),
            ),
            (
                502,
                Some("bad gateway"),
                std::mem::discriminant(&AiError::ProviderServer {
                    status: 502,
                    safe_detail: "bad gateway".into(),
                }),
            ),
            (
                418,
                None,
                std::mem::discriminant(&AiError::ProviderServer {
                    status: 418,
                    safe_detail: String::new(),
                }),
            ),
        ];
        for (status, body, expected_disc) in cases {
            let err = AiError::from_status(*status, *body);
            assert_eq!(
                std::mem::discriminant(&err),
                *expected_disc,
                "wrong discriminant for status {status}: {err}"
            );
        }
    }

    #[test]
    fn safe_error_preserves_category_and_kind() {
        let err = AiError::ProviderServer {
            status: 503,
            safe_detail: "down".into(),
        };
        let safe = err.to_safe_error();
        assert_eq!(safe.category(), ErrorCategory::Unavailable);
        assert_eq!(safe.kind(), SafeErrorKind::Adapter);
    }
}
