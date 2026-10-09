//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::PolicyReport;

/// Redact credential-like values from a single evidence string. The
/// rule set is intentionally conservative: only well-known token shapes
/// (AWS, GitHub, GitLab, Slack, JWT, private keys) and obvious
/// `key=value` secrets are replaced. Generic high-entropy values are
/// left alone so legitimate code is not over-scrubbed.
pub fn redact_credentials(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        let (next_start, next_end) = match match_secret(rest) {
            Some(span) => span,
            None => {
                out.push_str(rest);
                return out;
            }
        };
        out.push_str(&rest[..next_start]);
        out.push_str("[REDACTED]");
        rest = &rest[next_end..];
    }
}

fn match_secret(input: &str) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for (start, end) in [
        span_aws_access_key(input),
        span_github_pat(input),
        span_github_fine_pat(input),
        span_gitlab_pat(input),
        span_slack_token(input),
        span_jwt(input),
        span_private_key(input),
        span_kv_secret(input),
    ]
    .into_iter()
    .flatten()
    {
        if best.is_none_or(|(s, _)| start < s) {
            best = Some((start, end));
        }
    }
    best
}

fn span_aws_access_key(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"AKIA";
    let start = find_subslice(bytes, needle)?;
    let mut end = start + needle.len();
    for _ in 0..16 {
        if end >= bytes.len() || !bytes[end].is_ascii_alphanumeric() {
            return None;
        }
        end += 1;
    }
    Some((start, end))
}

fn span_github_pat(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"ghp_";
    let start = find_subslice(bytes, needle)?;
    let mut end = start + needle.len();
    let mut matched = 0;
    while end < bytes.len() && bytes[end].is_ascii_alphanumeric() && matched < 64 {
        end += 1;
        matched += 1;
    }
    if matched < 20 {
        return None;
    }
    Some((start, end))
}

fn span_github_fine_pat(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"github_pat_";
    let start = find_subslice(bytes, needle)?;
    let mut end = start + needle.len();
    let mut matched = 0;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
        && matched < 128
    {
        end += 1;
        matched += 1;
    }
    if matched < 20 {
        return None;
    }
    Some((start, end))
}

fn span_gitlab_pat(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"glpat-";
    let start = find_subslice(bytes, needle)?;
    let mut end = start + needle.len();
    let mut matched = 0;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
        && matched < 128
    {
        end += 1;
        matched += 1;
    }
    if matched < 18 {
        return None;
    }
    Some((start, end))
}

fn span_slack_token(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"xox";
    let start = find_subslice(bytes, needle)?;
    if start + 4 >= bytes.len() || !matches!(bytes[start + 3], b'b' | b'a' | b'p' | b'r' | b's') {
        return None;
    }
    if bytes[start + 4] != b'-' {
        return None;
    }
    let mut end = start + 5;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-') {
        end += 1;
    }
    Some((start, end))
}

fn span_jwt(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"eyJ";
    let start = find_subslice(bytes, needle)?;
    let parts = [start, 0, 0, 0];
    let _ = parts;
    // Token shape: header.payload.signature, each segment URL-safe
    // base64-ish. Reject if any segment contains whitespace or '='.
    let mut cursor = start;
    for seg in 0..3 {
        if seg > 0 {
            if cursor >= bytes.len() || bytes[cursor] != b'.' {
                return None;
            }
            cursor += 1;
        }
        let seg_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'.' && !bytes[cursor].is_ascii_whitespace()
        {
            cursor += 1;
        }
        if cursor == seg_start {
            return None;
        }
        // Reject segments that are too short to be a JWT segment.
        if cursor - seg_start < 4 {
            return None;
        }
    }
    Some((start, cursor))
}

fn span_private_key(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let begin = find_subslice(bytes, b"-----BEGIN ")?;
    let after = begin + b"-----BEGIN ".len();
    let line_end = bytes[after..]
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len() - after);
    let header = &bytes[after..after + line_end];
    if !header
        .iter()
        .all(|b| b.is_ascii_uppercase() || *b == b' ' || *b == b'-')
    {
        return None;
    }
    if !contains_ci(header, b"PRIVATE KEY") {
        return None;
    }
    let end_needle = b"-----END ";
    let rest = &bytes[after..];
    let end_off = find_subslice(rest, end_needle)?;
    let after_end = after + end_off + end_needle.len();
    let end_line = bytes[after_end..]
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len() - after_end);
    if !contains_ci(&bytes[after_end..after_end + end_line], b"PRIVATE KEY") {
        return None;
    }
    Some((begin, after_end + end_line))
}

fn contains_ci(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| {
        w.iter()
            .zip(needle.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
}

pub(super) fn span_kv_secret(input: &str) -> Option<(usize, usize)> {
    let lower = input.to_ascii_lowercase();
    let mut keys: Vec<String> = vec![
        "password".to_string(),
        "passwd".to_string(),
        "pwd".to_string(),
        "secret".to_string(),
        "token".to_string(),
        "api_key".to_string(),
        "apikey".to_string(),
        "api-key".to_string(),
        "access_key".to_string(),
        "access-key".to_string(),
    ];
    for extra in crate::contract::secret_field_substrings() {
        let e = extra.to_ascii_lowercase();
        if !keys.iter().any(|k| k == &e) {
            keys.push(e);
        }
    }
    let keys = keys;
    let mut best: Option<(usize, usize)> = None;
    for key in &keys {
        let mut search_from = 0;
        while let Some(rel) = lower[search_from..].find(key.as_str()) {
            let key_start = search_from + rel;
            let key_end = key_start + key.len();
            // Require a non-letter boundary so `tokenized` does not match.
            let prev_ok = key_start == 0 || !input.as_bytes()[key_start - 1].is_ascii_alphabetic();
            let after = &input[key_end..];
            let after_trim = after.trim_start();
            let trim_len = after.len() - after_trim.len();
            if !prev_ok || after_trim.is_empty() || !matches!(after_trim.as_bytes()[0], b':' | b'=')
            {
                search_from = key_end;
                continue;
            }
            // Skip the separator.
            let sep_off = trim_len + 1;
            let bytes = input.as_bytes();
            let mut value_start = key_end + sep_off;
            // Skip one optional space after the separator.
            if value_start < bytes.len() && bytes[value_start] == b' ' {
                value_start += 1;
            }
            let mut end = value_start;
            while end < bytes.len() {
                let b = bytes[end];
                if b.is_ascii_whitespace() || b == b',' || b == b';' || b == b'}' || b == b']' {
                    break;
                }
                end += 1;
            }
            let value_len = end - value_start;
            if value_len >= 6 {
                let span = (key_start, end);
                if best.is_none_or(|(s, _)| key_start < s) {
                    best = Some(span);
                }
                // Continue scanning for the same key in case a later
                // occurrence of the same key produces an earlier span
                // in a different key iteration (the outer `for` will
                // still pick the earliest across all keys).
            }
            search_from = key_end;
        }
    }
    best
}

pub(super) fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Redact credential-like values from a [`PolicyReport`] in place.
/// Public so callers (doctor, registry, future transports) can apply
/// the same pipeline when they consume a report from a source that did
/// not go through [`run_driftwatch`].
pub fn redact_report_in_place(report: &mut PolicyReport) {
    for finding in &mut report.findings {
        finding.message = redact_credentials(&finding.message);
        finding.evidence = std::mem::take(&mut finding.evidence)
            .into_iter()
            .map(|line| redact_credentials(&line))
            .collect();
    }
}
