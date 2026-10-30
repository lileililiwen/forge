//! The two dependency-policy files and the lockfile must agree about the
//! advisories Forge accepts.
//!
//! `cargo deny check` reads `deny.toml`; `cargo audit` reads
//! `.cargo/audit.toml`. They are hand-maintained and used to drift: an
//! exception added to one left the other job red. These checks do not run the
//! scanners (they fetch an advisory database); they keep the policy honest.

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

/// Every `RUSTSEC-YYYY-NNNN` token in the text, deduplicated in file order.
fn advisory_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("RUSTSEC-") {
        let tail = &rest[start..];
        let id: String = tail
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let length = id.len();
        if length >= "RUSTSEC-0000-0000".len() && !ids.contains(&id) {
            ids.push(id);
        }
        rest = &tail[length..];
    }
    ids
}

/// The exception lines in `deny.toml`'s `[advisories] ignore` list.
fn deny_ignore_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.trim_start().starts_with("{ id = \"RUSTSEC-"))
        .collect()
}

#[test]
fn both_policy_files_name_every_vulnerability_exception() {
    let deny = read("deny.toml");
    let audit = read(".cargo/audit.toml");
    let deny_ids = advisory_ids(&deny);
    let audit_ids = advisory_ids(&audit);
    for id in ["RUSTSEC-2023-0071", "RUSTSEC-2026-0387"] {
        assert!(
            deny_ids.iter().any(|candidate| candidate == id),
            "deny.toml is missing {id}"
        );
        assert!(
            audit_ids.iter().any(|candidate| candidate == id),
            ".cargo/audit.toml is missing {id}; the two policy files must agree"
        );
    }
}

#[test]
fn every_deny_ignore_carries_a_non_empty_reason() {
    let deny = read("deny.toml");
    let lines = deny_ignore_lines(&deny);
    assert!(!lines.is_empty(), "deny.toml has no advisory exceptions");
    for line in lines {
        assert!(
            line.contains("reason = \""),
            "exception has no reason: {line}"
        );
        let reason = line
            .split("reason = \"")
            .nth(1)
            .unwrap_or_default()
            .split('"')
            .next()
            .unwrap_or_default();
        assert!(reason.len() > 20, "exception reason is too thin: {line}");
    }
}

#[test]
fn the_crypto_bigint_exception_names_the_reachable_boundary() {
    let deny = read("deny.toml");
    let line = deny_ignore_lines(&deny)
        .into_iter()
        .find(|line| line.contains("RUSTSEC-2026-0387"))
        .expect("RUSTSEC-2026-0387 exception is missing from deny.toml");
    for needle in [
        "openidconnect",
        "public-key",
        "verification",
        "no private-key",
    ] {
        assert!(
            line.contains(needle),
            "exception does not name `{needle}`: {line}"
        );
    }
    assert!(
        line.contains("0.7.6") || line.contains("0.14"),
        "exception does not name the upstream trigger that ends it: {line}"
    );
}

#[test]
fn the_audit_exception_names_the_same_path() {
    let audit = read(".cargo/audit.toml");
    assert!(audit.contains("RUSTSEC-2026-0387"));
    assert!(audit.contains("openidconnect"));
    assert!(audit.contains("public keys") || audit.contains("public-key"));
}

#[test]
fn the_lockfile_does_not_reinstate_the_removed_advisories() {
    let lock = read("Cargo.lock");
    assert!(
        !lock.contains("name = \"paste\"\nversion = \"1.0.15\""),
        "paste 1.0.15 (unmaintained) is back in Cargo.lock"
    );
    assert!(
        !lock.contains("name = \"lru\"\nversion = \"0.12.5\""),
        "lru 0.12.5 (unsound) is back in Cargo.lock"
    );
    assert!(
        !lock.contains("name = \"tokio\"\nversion = \"1.53.2\""),
        "tokio 1.53.2 (unsound) is back in Cargo.lock"
    );
}
