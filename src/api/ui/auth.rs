//! Authorization for the in-process portal UI.
//!
//! The UI routes reuse the existing bearer-token
//! authorization that the JSON API already enforces. A POST
//! to `/ui/projects/{id}/publish` additionally re-checks the
//! token against the form's hidden `token` field and refuses
//! cross-origin POSTs whose `Origin` header (when present)
//! does not match the loopback bind address.
//!
//! The bind address is recorded by the API service at start
//! time and exposed through [`crate::api::ApiConfig`]; the UI
//! layer reads it back so a custom `FORGE_API_BIND` /
//! `FORGE_API_PORT` is honoured without a parallel env lookup.

use std::net::IpAddr;

use crate::api::ApiConfig;

/// One authorization result. Either the request is allowed
/// (and carries the resolved token), or it is refused with a
/// typed code the renderer surfaces.
#[derive(Debug, Clone)]
pub enum AuthDecision {
    Allow {
        token: String,
    },
    Refuse {
        code: &'static str,
        message: &'static str,
    },
}

impl AuthDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthDecision::Allow { .. })
    }
}

/// Resolve the bearer token from the request. Prefers the
/// `Authorization: Bearer <id>` header (matches the JSON API
/// surface). Falls back to a `?token=<id>` query parameter so
/// a browser link with the token in the URL still works for
/// read paths. The hidden form field on POSTs is checked
/// separately by [`recheck_post_token`].
pub fn resolve_bearer_token(
    authorization: Option<&str>,
    query_token: Option<&str>,
) -> AuthDecision {
    if let Some(value) = authorization {
        if let Some(rest) = value.strip_prefix("Bearer ") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() && is_hex(trimmed) {
                return AuthDecision::Allow {
                    token: trimmed.to_string(),
                };
            }
            return AuthDecision::Refuse {
                code: "api-unauthorized",
                message: "bearer token must be a non-empty hex session id",
            };
        }
        return AuthDecision::Refuse {
            code: "api-unauthorized",
            message: "Authorization header must use the Bearer scheme",
        };
    }
    if let Some(value) = query_token {
        let trimmed = value.trim();
        if !trimmed.is_empty() && is_hex(trimmed) {
            return AuthDecision::Allow {
                token: trimmed.to_string(),
            };
        }
        return AuthDecision::Refuse {
            code: "api-unauthorized",
            message: "query token must be a non-empty hex session id",
        };
    }
    AuthDecision::Refuse {
        code: "api-unauthorized",
        message: "missing Authorization: Bearer <session-id> header or ?token=<id>",
    }
}

/// Re-check the bearer token on a POST against the hidden
/// form field. The form field is the only thing the browser
/// sends back, so any attacker who tricks the browser into
/// submitting the form without the matching token is refused
/// before the journal row is written.
pub fn recheck_post_token(form_token: Option<&str>, bearer_token: &str) -> AuthDecision {
    match form_token {
        Some(value) if constant_time_eq(value.as_bytes(), bearer_token.as_bytes()) => {
            AuthDecision::Allow {
                token: bearer_token.to_string(),
            }
        }
        Some(_) => AuthDecision::Refuse {
            code: "api-token-mismatch",
            message: "form token does not match the bearer token; refusing the publish",
        },
        None => AuthDecision::Refuse {
            code: "api-token-required",
            message: "publish requires the matching bearer token in the hidden form field",
        },
    }
}

/// Refuse a POST whose `Origin` header (when present) does not
/// match the loopback bind address. A request with no `Origin`
/// header (some browsers omit it on same-origin POSTs; some
/// proxies strip it) is allowed — the bearer-token re-check is
/// the primary defence.
pub fn check_origin(origin: Option<&str>, config: &ApiConfig) -> AuthDecision {
    let Some(origin) = origin else {
        return AuthDecision::Allow {
            token: String::new(),
        };
    };
    let expected = expected_origin(config);
    if origin == expected {
        AuthDecision::Allow {
            token: String::new(),
        }
    } else {
        AuthDecision::Refuse {
            code: "ui-origin-mismatch",
            message: "Origin header does not match the loopback bind address; refusing the publish",
        }
    }
}

/// The expected Origin value for the API bind address.
/// Loopback v4 addresses carry `http://`; loopback v6 carry
/// `http://[::1]`. Non-loopback binds are not allowed by the
/// API at startup, so we never have to handle `https`.
pub fn expected_origin(config: &ApiConfig) -> String {
    let scheme = "http";
    let host = match config.bind {
        IpAddr::V4(v4) if v4.is_loopback() => "127.0.0.1".to_string(),
        IpAddr::V6(v6) if v6.is_loopback() => "[::1]".to_string(),
        other => other.to_string(),
    };
    format!("{scheme}://{host}:{}", config.port)
}

fn is_hex(raw: &str) -> bool {
    !raw.is_empty() && raw.chars().all(|c| c.is_ascii_hexdigit())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(bind: &str, port: u16) -> ApiConfig {
        let mut c = ApiConfig::default();
        c.bind = bind.parse().unwrap();
        c.port = port;
        c
    }

    #[test]
    fn resolve_bearer_accepts_honor_header() {
        let d = resolve_bearer_token(Some("Bearer deadbeef"), None);
        assert!(matches!(d, AuthDecision::Allow { ref token } if token == "deadbeef"));
    }

    #[test]
    fn resolve_bearer_accepts_honor_query() {
        let d = resolve_bearer_token(None, Some("abcdef"));
        assert!(matches!(d, AuthDecision::Allow { ref token } if token == "abcdef"));
    }

    #[test]
    fn resolve_bearer_rejects_non_hex() {
        let d = resolve_bearer_token(Some("Bearer not-hex!"), None);
        assert!(matches!(d, AuthDecision::Refuse { .. }));
    }

    #[test]
    fn resolve_bearer_rejects_wrong_scheme() {
        let d = resolve_bearer_token(Some("Basic deadbeef"), None);
        assert!(matches!(d, AuthDecision::Refuse { .. }));
    }

    #[test]
    fn recheck_post_token_matches() {
        let d = recheck_post_token(Some("deadbeef"), "deadbeef");
        assert!(d.is_allowed());
    }

    #[test]
    fn recheck_post_token_mismatch() {
        let d = recheck_post_token(Some("nope"), "deadbeef");
        assert!(!d.is_allowed());
    }

    #[test]
    fn recheck_post_token_missing() {
        let d = recheck_post_token(None, "deadbeef");
        assert!(!d.is_allowed());
    }

    #[test]
    fn origin_check_passes_matching_loopback() {
        let c = cfg("127.0.0.1", 8765);
        let d = check_origin(Some("http://127.0.0.1:8765"), &c);
        assert!(d.is_allowed());
    }

    #[test]
    fn origin_check_rejects_external() {
        let c = cfg("127.0.0.1", 8765);
        let d = check_origin(Some("http://evil.example.com"), &c);
        assert!(!d.is_allowed());
    }

    #[test]
    fn origin_check_passes_when_missing() {
        let c = cfg("127.0.0.1", 8765);
        let d = check_origin(None, &c);
        assert!(d.is_allowed());
    }

    #[test]
    fn expected_origin_renders_loopback_v4() {
        let c = cfg("127.0.0.1", 18765);
        assert_eq!(expected_origin(&c), "http://127.0.0.1:18765");
    }

    #[test]
    fn expected_origin_renders_loopback_v6() {
        let c = cfg("::1", 18765);
        assert_eq!(expected_origin(&c), "http://[::1]:18765");
    }
}
