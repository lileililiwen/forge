//! CLI contract for `component-qualify-evidence-truth`.
//!
//! A component promotion records only the evidence the operator
//! supplied. A promotion to `certified` without a test coverage, a
//! last-verified timestamp and a security review is refused: the
//! refusal names the missing evidence, the prior quality level stays
//! and no receipt is written. A receipt never carries a value nobody
//! produced.
//!
//! - a certified promotion without evidence is refused and writes no
//!   receipt;
//! - a certified promotion with complete evidence is accepted and the
//!   receipt records exactly those values;
//! - a promotion below `certified` writes a receipt that carries only
//!   the evidence the operator supplied.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    assert_eq!(
        out.status.code(),
        Some(0),
        "the qualify command exits 0; stderr={}",
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("the qualify command is a JSON document")
}

/// A verification timestamp the freshness gate accepts.
fn fresh_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[test]
fn a_certified_promotion_without_evidence_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    fs::create_dir_all(&work).unwrap();

    let value = run_json(
        &db,
        &[
            "component",
            "qualify",
            "toast",
            "--to",
            "certified",
            "--reason",
            "looks fine",
            "--path",
            work.to_str().unwrap(),
        ],
    );

    assert_eq!(value["promoted"], false);
    assert_eq!(value["prior_quality"], "verified");
    assert_eq!(value["target_quality"], "certified");
    let missing = value["missing_evidence"]
        .as_array()
        .expect("the refusal names the evidence");
    let missing: Vec<&str> = missing.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(
        missing,
        vec!["test_coverage", "last_verified", "security_review"],
        "{missing:?}"
    );
    assert!(value["files_written"].as_array().unwrap().is_empty());
    assert!(value["note"]
        .as_str()
        .unwrap()
        .contains("missing required evidence"));
    // Nothing was measured, reviewed or verified, so nothing is
    // written: no receipt, and no project directory at all.
    assert!(!work.join(".forge/components/toast/qualify.json").exists());
    assert!(!work.join(".forge").exists());
}

#[test]
fn a_certified_promotion_with_complete_evidence_is_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    fs::create_dir_all(&work).unwrap();
    let stamp = fresh_timestamp();

    let value = run_json(
        &db,
        &[
            "component",
            "qualify",
            "toast",
            "--to",
            "certified",
            "--reason",
            "production ready",
            "--coverage",
            "0.93",
            "--last-verified",
            &stamp,
            "--security-review",
            "true",
            "--path",
            work.to_str().unwrap(),
        ],
    );

    assert_eq!(value["promoted"], true);
    assert_eq!(value["prior_quality"], "verified");
    assert_eq!(value["target_quality"], "certified");
    assert!(value["missing_evidence"].as_array().unwrap().is_empty());
    let files = value["files_written"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], ".forge/components/toast/qualify.json");

    let body = fs::read_to_string(work.join(".forge/components/toast/qualify.json")).unwrap();
    assert!(body.contains("\"target_quality\": \"certified\""));
    assert!(body.contains("\"test_coverage\": 0.93"), "{body}");
    assert!(body.contains("\"security_review\": true"), "{body}");
    assert!(body.contains(stamp.trim_end_matches('Z')), "{body}");
}

#[test]
fn a_receipt_carries_only_the_supplied_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    fs::create_dir_all(&work).unwrap();

    // `file-picker` is an experimental component; a promotion to
    // `verified` needs no evidence gate, so the receipt must carry
    // the coverage the operator supplied and nothing else.
    let value = run_json(
        &db,
        &[
            "component",
            "qualify",
            "file-picker",
            "--to",
            "verified",
            "--reason",
            "exercised on one profile",
            "--coverage",
            "0.5",
            "--path",
            work.to_str().unwrap(),
        ],
    );

    assert_eq!(value["promoted"], true);
    assert_eq!(value["prior_quality"], "experimental");
    let body = fs::read_to_string(work.join(".forge/components/file-picker/qualify.json")).unwrap();
    assert!(body.contains("\"test_coverage\": 0.5"), "{body}");
    assert!(body.contains("\"known_issues\": []"), "{body}");
    assert!(!body.contains("last_verified"), "{body}");
    assert!(!body.contains("security_review"), "{body}");
}
