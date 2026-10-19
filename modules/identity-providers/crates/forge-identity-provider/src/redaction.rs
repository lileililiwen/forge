//! Secret-free redaction helpers.
//!
//! A provider response is shown to an owner and written into a project manifest. It
//! must never carry a secret value, so every string that could carry one is checked
/// against the marker list before it is used.
///
/// The substrings that mark a value as a secret.
pub const SECRET_MARKERS: &[&str] = &[
    "secret",
    "password",
    "passwd",
    "api_key",
    "apikey",
    "private_key",
    "client_secret",
    "access_token",
    "refresh_token",
    "bearer ",
    "authorization",
];

/// Returns true when the text carries a secret marker.
pub fn looks_secretful(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    SECRET_MARKERS.iter().any(|marker| lowered.contains(marker))
}

/// Redacts a secret value out of free text, keeping the surrounding shape.
pub fn redact_text(text: &str) -> String {
    if !looks_secretful(text) {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    for token in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        if looks_secretful(token) {
            out.push_str("[redacted]");
        } else {
            out.push_str(token);
        }
    }
    out
}

/// Returns the URL without its query and fragment, which can carry a code.
pub fn redact_url(url: &str) -> String {
    match url.split_once('?') {
        Some((base, _)) => base.to_string(),
        None => url.to_string(),
    }
}
