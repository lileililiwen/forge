//! What may leave Forge.
//!
//! Every transport hands raw operator input to
//! [`validate_share`] and nothing else validates a share write, so a
//! CLI flag, an HTTP body field and a future browser form are refused
//! identically and leave the same persisted finding.

use std::collections::{BTreeMap, BTreeSet};

use crate::portfolio::share::{
    CREDENTIAL_QUERY_KEYS, EVIDENCE_KEYS, MAX_CATEGORY_CHARS, MAX_EVIDENCE_KEYS,
    MAX_SUMMARY_CHARS, MAX_SURFACES, MAX_TITLE_CHARS, MAX_URL_CHARS, PRIVATE_HOST_SUFFIXES,
    PRIVATE_PATH_SEGMENTS, ShareSurface, ShareWrite,
};

/// Well-known credential prefixes. A value carrying one of these is
/// refused outright: a public manifest never redacts, it rejects.
const SECRET_PREFIXES: [&str; 11] = [
    "AKIA",
    "ASIA",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "eyJ",
];

// --- secret detection ---------------------------------------------------

/// Whether a value carries a credential shape.
///
/// The shared policy owns the `key=value` and well-known token
/// detectors, so this reuses [`redact_credentials`] as its first
/// line rather than re-implementing the marker set. The explicit
/// prefix scan catches the token shapes a summary may carry without a
/// surrounding `=` (a leaked PAT pasted into a sentence, a PEM
/// header, a bare JWT).
pub fn looks_like_secret(value: &str) -> bool {
    if crate::policy::redact_credentials(value) != value {
        return true;
    }
    if value.contains("-----BEGIN") || value.contains("-----begin") {
        return true;
    }
    SECRET_PREFIXES.iter().any(|prefix| value.contains(prefix))
}

/// Describe a rejected value without echoing it. The finding carries
/// the field and the reason; the offending bytes never leave the
/// process, which is what makes a rejection safe to surface in an
/// audit trail.
pub(super) fn secret_finding(field: &str) -> String {
    format!(
        "{field} was refused because it carries a credential-shaped value; \
         the value itself was not recorded"
    )
}

// --- text validation ----------------------------------------------------

/// Validate a required public text field: bounded, single-line and
/// control-free. Secret-shaped content is refused, not redacted.
pub(super) fn validate_public_text(
    field: &str,
    raw: &str,
    max_chars: usize,
    allow_empty: bool,
) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        if allow_empty {
            return Ok(String::new());
        }
        return Err(format!("{field} is required"));
    }
    if looks_like_secret(trimmed) {
        return Err(secret_finding(field));
    }
    if trimmed.chars().count() > max_chars {
        return Err(format!("{field} is longer than {max_chars} characters"));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(format!("{field} must not contain control characters"));
    }
    Ok(trimmed.to_string())
}

// --- URL validation -----------------------------------------------------

/// Validate one public HTTPS URL.
///
/// The checks are ordered cheapest-first and every refusal names the
/// field, because this string ends up in an admin-facing finding. A
/// public showcase URL must be: `https://`, free of embedded
/// credentials, free of a non-default port, on a fully qualified
/// public host, free of dot-segments or percent-encoding, and free of
/// credential-bearing query keys.
pub fn validate_public_url(field: &str, raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} is required"));
    }
    if trimmed.chars().count() > MAX_URL_CHARS {
        return Err(format!("{field} is longer than {MAX_URL_CHARS} characters"));
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(format!(
            "{field} must not contain whitespace or control characters"
        ));
    }
    let rest = match trimmed.strip_prefix("https://") {
        Some(rest) => rest,
        None => {
            return Err(format!(
                "{field} must use https://; a plain-http or scheme-less public URL is refused"
            ))
        }
    };
    let (authority, remainder) = match rest.find(['/', '?', '#']) {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err(format!("{field} has no host"));
    }
    if authority.contains('@') {
        return Err(format!(
            "{field} must not embed credentials in its authority; \
             a public URL cannot carry a username or password"
        ));
    }
    let (host, port) = split_authority(authority, field)?;
    if let Some(port) = port {
        if port != "443" {
            return Err(format!(
                "{field} must not declare port {port}; only the default https port is public"
            ));
        }
    }
    validate_public_host(field, host)?;
    validate_public_path(field, remainder)?;
    Ok(trimmed.to_string())
}

/// Split `host[:port]`, tolerating a bracketed IPv6 literal.
fn split_authority<'a>(
    authority: &'a str,
    field: &str,
) -> Result<(&'a str, Option<&'a str>), String> {
    if let Some(rest) = authority.strip_prefix('[') {
        let Some(close) = rest.find(']') else {
            return Err(format!("{field} has an unterminated IPv6 host"));
        };
        let host = &authority[..close + 2];
        let tail = &authority[close + 2..];
        return match tail.strip_prefix(':') {
            Some(port) if !port.is_empty() => Ok((host, Some(port))),
            Some(_) => Err(format!("{field} has an empty port")),
            None if tail.is_empty() => Ok((host, None)),
            None => Err(format!("{field} has a malformed authority")),
        };
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => {
            if port.is_empty() {
                return Err(format!("{field} has an empty port"));
            }
            if !port.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!("{field} has a non-numeric port"));
            }
            Ok((host, Some(port)))
        }
        None => Ok((authority, None)),
    }
}

/// Refuse a host that is not a public, fully qualified name.
fn validate_public_host(field: &str, host: &str) -> Result<(), String> {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if bare.is_empty() {
        return Err(format!("{field} has no host"));
    }
    if bare.contains(':') {
        // IPv6 literal: only a routable, non-loopback, non-link-local
        // address is acceptable, and it must carry the brackets.
        if host != format!("[{bare}]") {
            return Err(format!("{field} has a malformed IPv6 host"));
        }
        return validate_public_ipv6(field, bare);
    }
    let lower = bare.to_ascii_lowercase();
    if PRIVATE_HOST_SUFFIXES
        .iter()
        .any(|suffix| lower == *suffix || lower.ends_with(suffix))
    {
        return Err(format!(
            "{field} points at a private or local host; a public showcase URL must resolve publicly"
        ));
    }
    if !lower.contains('.') {
        return Err(format!(
            "{field} host `{lower}` is not fully qualified; a bare name resolves inside the operator's network"
        ));
    }
    if let Some(octets) = parse_ipv4(&lower) {
        return validate_public_ipv4(field, &octets);
    }
    if !lower
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(format!(
            "{field} host contains characters that are not valid in a host name"
        ));
    }
    if lower.starts_with('.') || lower.contains("..") || lower.ends_with('.') {
        return Err(format!("{field} host `{lower}` is not a valid host name"));
    }
    for label in lower.split('.') {
        if label.is_empty() || label.starts_with('-') || label.ends_with('-') {
            return Err(format!("{field} host `{lower}` is not a valid host name"));
        }
    }
    Ok(())
}

fn parse_ipv4(host: &str) -> Option<Vec<u8>> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut out = Vec::with_capacity(4);
    for part in parts {
        if part.is_empty() || part.len() > 3 || !part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        out.push(part.parse::<u8>().ok()?);
    }
    Some(out)
}

fn validate_public_ipv4(field: &str, octets: &[u8]) -> Result<(), String> {
    let [a, b, ..] = octets else {
        return Err(format!("{field} host is not a valid IPv4 address"));
    };
    let private = matches!(
        (a, b),
        (0, _) | (10, _) | (127, _) | (169, 254) | (172, 16..=31) | (192, 168) | (100, 64..=127)
    );
    if private {
        return Err(format!(
            "{field} points at a private or loopback address; a public showcase URL must resolve publicly"
        ));
    }
    Ok(())
}

fn validate_public_ipv6(field: &str, host: &str) -> Result<(), String> {
    let lower = host.to_ascii_lowercase();
    let compressed = if let Some(tail) = lower.strip_prefix("::") {
        tail.to_string()
    } else if lower.starts_with("::") {
        String::new()
    } else {
        lower.clone()
    };
    let leading_unspecified = lower == "::" || lower == "::0" || lower == "0:0:0:0:0:0:0:0";
    if leading_unspecified
        || compressed == "1"
        || compressed.starts_with("fe80:")
        || compressed.starts_with("fc")
        || compressed.starts_with("fd")
    {
        return Err(format!(
            "{field} points at a loopback, link-local or unique-local address; \
             a public showcase URL must resolve publicly"
        ));
    }
    Ok(())
}

/// Validate the path, query and fragment of a public URL.
fn validate_public_path(field: &str, remainder: &str) -> Result<(), String> {
    if remainder.is_empty() {
        return Ok(());
    }
    let (path, tail) = match remainder.find(['?', '#']) {
        Some(index) => (&remainder[..index], &remainder[index..]),
        None => (remainder, ""),
    };
    for raw_segment in path.split('/') {
        let segment = raw_segment.to_ascii_lowercase();
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." {
            return Err(format!(
                "{field} must not contain `.` or `..` path segments"
            ));
        }
        if segment.contains('%') {
            return Err(format!(
                "{field} must not percent-encode a path segment; an encoded private path could not be reviewed"
            ));
        }
        if PRIVATE_PATH_SEGMENTS.contains(&segment.as_str()) {
            return Err(format!(
                "{field} path segment `{segment}` is a private or administrative surface and cannot be published"
            ));
        }
    }
    for pair in tail
        .split(['?', '#'])
        .filter(|part| !part.is_empty())
        .flat_map(|part| part.split('&'))
    {
        let Some((key, _)) = pair.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if CREDENTIAL_QUERY_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "{field} carries a credential-shaped query key `{key}`; \
                 a public URL must not carry a credential"
            ));
        }
    }
    Ok(())
}

// --- status evidence ----------------------------------------------------

/// Validate the optional `status_evidence` block. It is a closed,
/// bounded, string-valued object: no nested documents, no arrays, no
/// arbitrary metadata map, and every value redaction-checked.
pub fn validate_status_evidence(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.len() > crate::portfolio::MAX_EVIDENCE_BYTES {
        return Err(format!(
            "status evidence is larger than {} bytes",
            crate::portfolio::MAX_EVIDENCE_BYTES
        ));
    }
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("status evidence is not valid JSON: {err}"))?;
    let serde_json::Value::Object(entries) = value else {
        return Err("status evidence must be a JSON object".to_string());
    };
    if entries.is_empty() {
        return Ok(None);
    }
    if entries.len() > MAX_EVIDENCE_KEYS {
        return Err(format!(
            "status evidence carries more than {MAX_EVIDENCE_KEYS} keys"
        ));
    }
    let mut normalized: BTreeMap<String, String> = BTreeMap::new();
    for (key, value) in entries {
        if !EVIDENCE_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "status evidence key `{key}` is not allowed; expected one of {}",
                EVIDENCE_KEYS.join(", ")
            ));
        }
        let serde_json::Value::String(text) = value else {
            return Err(format!("status evidence `{key}` must be a string"));
        };
        let text = validate_public_text(
            &format!("status evidence `{key}`"),
            &text,
            crate::portfolio::MAX_NOTE_CHARS,
            false,
        )?;
        normalized.insert(key, text);
    }
    serde_json::to_string(&normalized)
        .map(Some)
        .map_err(|err| format!("status evidence could not be serialized: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::share::{ShowcaseStatus, Visibility};

    fn write() -> ShareWrite {
        ShareWrite {
            title: "Alethefy".to_string(),
            summary: "Deterministic project evidence.".to_string(),
            category: "platform".to_string(),
            source_url: "https://github.com/lileililiwen/alethefy".to_string(),
            demo_url: Some("https://alethefy.example.com/".to_string()),
            visibility: Visibility::Public,
            featured: true,
            showcase_status: ShowcaseStatus::Beta,
            status_evidence: None,
            surfaces: vec![ShareSurface {
                label: "Docs".to_string(),
                url: "https://alethefy.example.com/docs".to_string(),
            }],
        }
    }

    #[test]
    fn public_urls_must_be_https() {
        assert!(validate_public_url("source_url", "https://example.com/x").is_ok());
        for bad in [
            "http://example.com",
            "example.com",
            "HTTPS://example.com",
            "https://",
            "ftp://example.com",
        ] {
            assert!(
                validate_public_url("source_url", bad).is_err(),
                "must refuse {bad}"
            );
        }
    }

    #[test]
    fn public_urls_refuse_credentials_ports_and_internal_hosts() {
        for bad in [
            "https://user:pass@example.com/",
            "https://example.com:8443/",
            "https://localhost/",
            "https://forge.internal/",
            "https://box.local/",
            "https://builder",
            "https://10.1.2.3/",
            "https://192.168.0.9/",
            "https://127.0.0.1/",
            "https://169.254.10.1/",
            "https://[::1]/",
            "https://[fe80::1]/",
        ] {
            assert!(
                validate_public_url("source_url", bad).is_err(),
                "must refuse {bad}"
            );
        }
        assert!(validate_public_url("source_url", "https://example.com:443/").is_ok());
        assert!(validate_public_url("source_url", "https://203.0.113.9/").is_ok());
    }

    #[test]
    fn admin_and_account_surfaces_are_refused() {
        for bad in [
            "https://example.com/admin/settings",
            "https://example.com/admin",
            "https://example.com/account/billing",
            "https://example.com/ACCOUNT",
            "https://example.com/app/private/data",
            "https://example.com/a/../admin",
            "https://example.com/%61dmin",
            "https://example.com/x?token=abc",
            "https://example.com/x?api_key=abc&ok=1",
        ] {
            assert!(
                validate_public_url("surface url", bad).is_err(),
                "must refuse {bad}"
            );
        }
        assert!(validate_public_url("surface url", "https://example.com/app/admins").is_ok());
        assert!(validate_public_url("surface url", "https://example.com/app?view=demo").is_ok());
    }

    #[test]
    fn secret_shaped_values_are_refused_and_never_echoed() {
        assert!(looks_like_secret("password=hunter2"));
        assert!(looks_like_secret("AKIAIOSFODNN7EXAMPLE"));
        assert!(looks_like_secret("ghp_abcdefghijklmnopqrstuvwxyz0123"));
        assert!(looks_like_secret("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(!looks_like_secret("deterministic project evidence"));
        let err = validate_public_text("summary", "token=abcdef", 100, false)
            .expect_err("a credential must be refused");
        assert!(err.contains("summary"), "{err}");
        assert!(
            !err.contains("abcdef"),
            "the value must not be echoed: {err}"
        );
    }

    #[test]
    fn public_text_is_required_bounded_and_control_free() {
        assert!(validate_public_text("title", "  Alethefy  ", 120, false).is_ok());
        assert!(validate_public_text("title", "   ", 120, false).is_err());
        assert!(validate_public_text("title", &"x".repeat(121), 120, false).is_err());
        assert!(validate_public_text("title", "a\nb", 120, false).is_err());
        assert!(validate_public_text("category", "", 64, true).is_ok());
    }

    #[test]
    fn surfaces_parse_and_reject_duplicates() {
        let surface = ShareSurface::split("Docs=https://example.com/docs").expect("surface");
        assert_eq!(surface.label, "Docs");
        assert!(ShareSurface::split("Docs").is_err());
        assert!(surface.validate().is_ok());
        assert!(ShareSurface::new("Admin", "https://example.com/admin")
            .validate()
            .is_err());
        let mut candidate = write();
        candidate.surfaces = vec![
            ShareSurface {
                label: "Docs".to_string(),
                url: "https://example.com/docs".to_string(),
            },
            ShareSurface {
                label: "Docs".to_string(),
                url: "https://example.com/docs".to_string(),
            },
        ];
        assert!(validate_share(&candidate).is_err());
    }

    #[test]
    fn a_share_write_is_normalized_and_sorted() {
        let mut candidate = write();
        candidate.surfaces = vec![
            ShareSurface {
                label: "Zeta".to_string(),
                url: "https://example.com/z".to_string(),
            },
            ShareSurface {
                label: "Alpha".to_string(),
                url: "https://example.com/a".to_string(),
            },
        ];
        let validated = validate_share(&candidate).expect("valid share");
        assert_eq!(validated.title, "Alethefy");
        assert_eq!(validated.surfaces[0].label, "Alpha");
        assert_eq!(validated.surfaces[1].label, "Zeta");
    }

    #[test]
    fn status_evidence_is_a_closed_bounded_object() {
        assert_eq!(
            validate_status_evidence(Some(r#"{"source":"fleet","state":"unknown"}"#))
                .expect("evidence")
                .as_deref(),
            Some(r#"{"source":"fleet","state":"unknown"}"#)
        );
        assert!(validate_status_evidence(Some("{}"))
            .expect("empty")
            .is_none());
        assert!(validate_status_evidence(Some(r#"{"token":"abc"}"#)).is_err());
        assert!(validate_status_evidence(Some(r#"{"state":3}"#)).is_err());
        assert!(validate_status_evidence(Some(r#"["state"]"#)).is_err());
        assert!(validate_status_evidence(Some(r#"{"state":"token=abcdef123456"}"#)).is_err());
        assert!(validate_status_evidence(None).expect("absent").is_none());
    }
}

// Validate one share write end to end and return it normalized.
/// Every field arrives unvalidated: the transports hand raw operator
/// input straight to this gate.
///
/// The order matters: the URL and private-surface rules run before
/// the text bounds, because a rejected private path is the finding an
/// admin most needs to see, and the secret-shape check runs before
/// anything is normalized so a credential never reaches a string the
/// caller could log.
pub fn validate_share(write: &ShareWrite) -> Result<ShareWrite, String> {
    let source_url = validate_public_url("source_url", &write.source_url)?;
    let demo_url = match &write.demo_url {
        Some(value) => Some(validate_public_url("demo_url", value)?),
        None => None,
    };
    if write.surfaces.len() > MAX_SURFACES {
        return Err(format!(
            "a share record may declare at most {MAX_SURFACES} public surfaces"
        ));
    }
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    let mut surfaces = Vec::with_capacity(write.surfaces.len());
    for surface in &write.surfaces {
        let validated = surface.validate()?;
        let key = (validated.label.clone(), validated.url.clone());
        if !seen.insert(key) {
            return Err(format!(
                "public surface `{}` is declared more than once",
                validated.label
            ));
        }
        surfaces.push(validated);
    }
    // Deterministic ordering keeps the canonical manifest stable
    // regardless of the order the surfaces were supplied in.
    surfaces.sort_by(|a, b| (&a.label, &a.url).cmp(&(&b.label, &b.url)));
    Ok(ShareWrite {
        title: validate_public_text("title", &write.title, MAX_TITLE_CHARS, false)?,
        summary: validate_public_text("summary", &write.summary, MAX_SUMMARY_CHARS, false)?,
        category: validate_public_text("category", &write.category, MAX_CATEGORY_CHARS, false)?,
        source_url,
        demo_url,
        visibility: write.visibility,
        featured: write.featured,
        showcase_status: write.showcase_status,
        status_evidence: validate_status_evidence(write.status_evidence.as_deref())?,
        surfaces,
    })
}
