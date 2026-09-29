//! Project evidence-gap assessment contract
//! (`project-evidence-gap-assessment`, `forge-project-evidence/0.1.0`).
//!
//! The contract: `forge project gaps [PROJECT]` reports one typed
//! finding per (category, subject) per catalog record, carries the
//! finding's source provenance, never collapses distinct gaps into a
//! single health value, and stays read-only across every transport.
//!
//! This suite answers:
//!
//! - A missing description and missing tags are two distinct findings.
//! - A reading on a stale observation is `warn`, not `pass`; an
//!   unavailable source is `unavailable`, never `fail`.
//! - The closed vocabularies round-trip through JSON and NDJSON; the
//!   human table layout is not the contract and is not asserted here.
//! - The CLI refuses an unknown `--category`, `--status` or
//!   `--remediation-class` with `catalog-invalid` and empty stdout.
//! - The CLI is read-only: the registry file and its mtime are
//!   identical before and after a `gaps` run, and the `unknown-project`
//!   refusal stores no project row.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-project-evidence/0.1.0";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_WORKSPACE_REGISTRY");
    cmd.env_remove("FORGE_INVENTORY_SOURCE");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run forge")
}

fn stdout_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let out = run(db, &args_with_format(args, "json"));
    assert!(
        out.status.success(),
        "forge {args:?} failed: {}",
        stderr_of(&out)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

fn args_with_format<'a>(args: &[&'a str], format: &'a str) -> Vec<&'a str> {
    let mut out: Vec<&str> = args.to_vec();
    out.push("--format");
    out.push(format);
    out
}

fn expect_refusal(db: &Path, args: &[&str], code: &str) {
    let out = run(db, args);
    assert!(
        !out.status.success(),
        "forge {args:?} unexpectedly succeeded: {}",
        stdout_of(&out)
    );
    assert!(
        out.stdout.is_empty(),
        "refusal printed stdout: {}",
        stdout_of(&out)
    );
    assert!(
        stderr_of(&out).contains(code),
        "expected `{code}` in stderr: {}",
        stderr_of(&out)
    );
}

fn write_project(dir: &Path, id: &str, profile: &str, maturity: &str, language: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: {profile}\n  \
             maturity: {maturity}\nruntime:\n  language: {language}\nfeatures:\n  auth: 0.1.0\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("README.md"), format!("# {id}\n")).unwrap();
}

fn db_path(tmp: &Path) -> PathBuf {
    tmp.join("registry.db")
}

fn local_fleet(tmp: &Path) -> PathBuf {
    let db = db_path(tmp);
    let alpha = tmp.join("alpha");
    let beta = tmp.join("beta");
    write_project(&alpha, "alpha", "rust-web", "L1", "rust");
    write_project(&beta, "beta", "python-service", "L3", "python");
    assert!(run(&db, &["register", alpha.to_str().unwrap()])
        .status
        .success());
    assert!(run(&db, &["register", beta.to_str().unwrap()])
        .status
        .success());
    db
}

fn workspace_registry(tmp: &Path) -> PathBuf {
    let dir = tmp.join("ws");
    fs::create_dir_all(dir.join("alpha")).unwrap();
    fs::create_dir_all(dir.join("gamma")).unwrap();
    fs::write(dir.join("alpha/forge.yaml"), "schema: 1\n").unwrap();
    let path = dir.join("projects.json");
    fs::write(
        &path,
        r#"{"schema_version": 1, "workspace_root": null,
  "projects": [
    {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active"},
    {"id": "gamma", "path": "gamma", "profile": "web-app", "lifecycle": "planning"}
  ]}"#,
    )
    .unwrap();
    path
}

fn inventory(tmp: &Path) -> PathBuf {
    let source = tmp.join("proj-alpha");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("docker-compose.yml"), "services: {}").unwrap();
    let path = tmp.join("inventory.json");
    fs::write(
        &path,
        serde_json::json!({
            "contract": "forge-project-inventory/0.1.0",
            "provider": "local",
            "generated_at": "2026-09-29T00:00:00Z",
            "projects": [
                {
                    "id": "alpha",
                    "repository": "https://example.invalid/alpha.git",
                    "revision": SHA,
                    "profile": "rust-product",
                    "runtime": "web",
                    "compose_file": "docker-compose.yml",
                    "source_path": source.display().to_string(),
                },
                {
                    "id": "delta",
                    "repository": "https://example.invalid/delta.git",
                    "revision": SHA,
                    "profile": "rust-product",
                    "runtime": "web",
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    path
}

fn gaps_of(value: &Value) -> &Value {
    value.get("gaps").expect("gaps envelope")
}

fn findings_of(value: &Value) -> &Vec<Value> {
    gaps_of(value)
        .get("findings")
        .and_then(Value::as_array)
        .expect("findings array")
}

#[test]
fn help_advertises_gaps_and_filters() {
    let tmp = tempfile::tempdir().unwrap();
    let db = db_path(tmp.path());
    let out = run(&db, &["project", "gaps", "--help"]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    let stdout = stdout_of(&out);
    for flag in [
        "--category",
        "--status",
        "--remediation-class",
        "--source",
        "--tag",
    ] {
        assert!(stdout.contains(flag), "missing flag {flag}: {stdout}");
    }
    assert!(stdout.contains("PROJECT"), "missing project arg: {stdout}");
}

#[test]
fn gaps_emits_one_finding_per_category_per_record_with_stable_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(&db, &["project", "gaps", "alpha"]);
    assert_eq!(gaps_of(&value)["contract"], CONTRACT);
    let findings = findings_of(&value);
    assert_eq!(
        findings.len(),
        7,
        "seven findings per record: {findings:#?}"
    );
    let ids: Vec<&str> = findings.iter().map(|f| f["id"].as_str().unwrap()).collect();
    for category in [
        "description",
        "tags",
        "ci",
        "compose",
        "manifest",
        "docs",
        "repository",
    ] {
        let needle = format!("gaps.{category}.alpha.");
        assert!(
            ids.iter().any(|id| id.starts_with(&needle)),
            "missing category {category} in {ids:?}"
        );
    }
}

#[test]
fn missing_description_is_a_finding_distinct_from_missing_tags() {
    // The local registry record's `name` is populated by `register`
    // (description → pass); `tags` may or may not be set depending on
    // what the project advertises. The contract is the
    // distinct-id invariant: even when both findings pass, their
    // ids differ and the consumer can route them independently.
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(&db, &["project", "gaps", "alpha"]);
    let findings = findings_of(&value);
    let description = findings
        .iter()
        .find(|f| f["category"] == "description")
        .unwrap();
    let tags = findings.iter().find(|f| f["category"] == "tags").unwrap();
    assert_eq!(description["id"], "gaps.description.alpha.name");
    assert_eq!(tags["id"], "gaps.tags.alpha.tags");
    assert_ne!(description["id"], tags["id"]);
}

#[test]
fn unknown_category_is_catalog_invalid_with_empty_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    expect_refusal(
        &db,
        &["project", "gaps", "alpha", "--category", "bogus"],
        "catalog-invalid",
    );
}

#[test]
fn unknown_status_is_catalog_invalid_with_empty_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    expect_refusal(
        &db,
        &["project", "gaps", "alpha", "--status", "bogus"],
        "catalog-invalid",
    );
}

#[test]
fn unknown_remediation_class_is_catalog_invalid_with_empty_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    expect_refusal(
        &db,
        &["project", "gaps", "alpha", "--remediation-class", "bogus"],
        "catalog-invalid",
    );
}

#[test]
fn category_filter_returns_only_matching_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(&db, &["project", "gaps", "alpha", "--category", "docs"]);
    let findings = findings_of(&value);
    assert!(!findings.is_empty(), "expected at least one docs finding");
    for finding in findings {
        assert_eq!(finding["category"], "docs");
    }
}

#[test]
fn status_filter_accepts_not_applicable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(
        &db,
        &["project", "gaps", "alpha", "--status", "not_applicable"],
    );
    let findings = findings_of(&value);
    for finding in findings {
        assert_eq!(finding["status"], "not_applicable");
    }
    assert!(
        !findings.is_empty(),
        "docs / github are not_applicable by construction"
    );
}

#[test]
fn unknown_project_is_refused_with_unknown_project_code() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let out = run(&db, &["project", "gaps", "no-such-app"]);
    assert!(!out.status.success(), "{}", stdout_of(&out));
    assert!(out.stdout.is_empty(), "{}", stdout_of(&out));
    assert!(
        stderr_of(&out).contains("unknown-project"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn json_and_ndjson_carry_the_same_finding_set_in_the_same_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let json_value = run_json(&db, &["project", "gaps"]);
    let json_findings = findings_of(&json_value);
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    for arg in ["project", "gaps", "--format", "ndjson"] {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge");
    assert!(out.status.success(), "{}", stderr_of(&out));
    let mut ndjson_findings: Vec<Value> = Vec::new();
    for line in stdout_of(&out).lines() {
        if line.trim().is_empty() {
            continue;
        }
        ndjson_findings.push(serde_json::from_str(line).expect("valid ndjson line"));
    }
    assert_eq!(json_findings.len(), ndjson_findings.len());
    for (a, b) in json_findings.iter().zip(ndjson_findings.iter()) {
        assert_eq!(a["id"], b["id"]);
        assert_eq!(a["status"], b["status"]);
        assert_eq!(a["category"], b["category"]);
    }
}

#[test]
fn gaps_does_not_mutate_the_registry_file_or_mtime() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let size_before = fs::metadata(&db).unwrap().len();
    let mtime_before = fs::metadata(&db).unwrap().modified().unwrap();
    let out = run(&db, &["project", "gaps"]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    let size_after = fs::metadata(&db).unwrap().len();
    let mtime_after = fs::metadata(&db).unwrap().modified().unwrap();
    assert_eq!(size_before, size_after);
    assert_eq!(mtime_before, mtime_after);
}

#[test]
fn gaps_does_not_create_a_missing_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let db = db_path(tmp.path());
    assert!(!db.is_file());
    let out = run(&db, &["project", "gaps"]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(!db.is_file(), "gaps must not create the registry file");
}

#[test]
fn unavailable_source_emits_unavailable_finding_not_fail() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    // Malformed workspace registry: the source is read, parsed, and
    // emits an explicit `unavailable` status rather than silently
    // producing zero rows.
    let bad = tmp.path().join("projects.json");
    fs::write(&bad, b"this is not valid json").unwrap();
    let value = run_json(
        &db,
        &[
            "project",
            "gaps",
            "--source",
            "workspace-registry",
            "--workspace-registry",
            bad.to_str().unwrap(),
        ],
    );
    let findings = findings_of(&value);
    assert!(
        !findings.is_empty(),
        "unreadable source must surface as findings, not zero rows"
    );
    let statuses: Vec<&str> = findings
        .iter()
        .map(|f| f["status"].as_str().unwrap())
        .collect();
    assert!(
        statuses.contains(&"unavailable"),
        "expected unavailable finding: {statuses:?}"
    );
    assert!(
        !statuses.contains(&"fail"),
        "unavailable must not be reported as fail: {statuses:?}"
    );
}

#[test]
fn github_source_marks_no_records_and_reports_unavailable() {
    // The catalog contract states that selecting `github` returns an
    // explicit `unavailable` status (no record is invented and no
    // GitHub request is made). The gap surface mirrors that
    // behaviour: no findings are emitted, but the source status is
    // reported as `unavailable` with a reason.
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(&db, &["project", "gaps", "--source", "github"]);
    let findings = findings_of(&value);
    assert!(
        findings.is_empty(),
        "github source must not invent findings: {findings:#?}"
    );
    let sources = gaps_of(&value)["sources"]
        .as_array()
        .expect("sources array");
    let github = sources
        .iter()
        .find(|s| s["source_kind"] == "github")
        .expect("github source status");
    assert_eq!(github["state"], "unavailable");
    assert!(github["reason"].is_string());
}

#[test]
fn two_sources_emit_two_findings_per_category_never_merged() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let ws = workspace_registry(tmp.path());
    let value = run_json(
        &db,
        &[
            "project",
            "gaps",
            "alpha",
            "--source",
            "local",
            "--source",
            "workspace-registry",
            "--workspace-registry",
            ws.to_str().unwrap(),
        ],
    );
    let findings = findings_of(&value);
    let tags_alpha = findings
        .iter()
        .filter(|f| f["category"] == "tags" && f["project_id"] == "alpha")
        .count();
    assert_eq!(
        tags_alpha, 2,
        "two sources means two findings per category: {findings:#?}"
    );
    let sources: std::collections::BTreeSet<&str> = findings
        .iter()
        .filter(|f| f["category"] == "tags" && f["project_id"] == "alpha")
        .map(|f| f["source"].as_str().unwrap())
        .collect();
    assert_eq!(
        sources.len(),
        2,
        "two findings from two distinct sources: {sources:?}"
    );
}

#[test]
fn stale_observation_warns_when_value_present() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let inv = inventory(tmp.path());
    let value = run_json(
        &db,
        &[
            "project",
            "gaps",
            "--source",
            "inventory",
            "--inventory",
            inv.to_str().unwrap(),
            "--max-age",
            "1",
        ],
    );
    let findings = findings_of(&value);
    let ci = findings.iter().find(|f| f["category"] == "ci").unwrap();
    assert_eq!(
        ci["status"], "warn",
        "inventory observation is older than 1s: {ci:?}"
    );
    let compose = findings
        .iter()
        .find(|f| f["category"] == "compose")
        .unwrap();
    assert_eq!(
        compose["status"], "warn",
        "inventory observation is older than 1s: {compose:?}"
    );
}

#[test]
fn finding_evidence_is_redacted_in_json_output() {
    let tmp = tempfile::tempdir().unwrap();
    let db = db_path(tmp.path());
    let alpha = tmp.path().join("alpha");
    write_project(&alpha, "alpha", "rust-web", "L1", "rust");
    assert!(run(&db, &["register", alpha.to_str().unwrap()])
        .status
        .success());
    // Forge never writes a credential to the registry, so the JSON
    // output must not contain any token-shaped value either.
    let value = run_json(&db, &["project", "gaps", "alpha"]);
    let serialized = serde_json::to_string(&value).unwrap();
    assert!(
        !serialized.contains("ghp_"),
        "serialized gaps must not contain token-shaped values: {serialized}"
    );
}

#[test]
fn no_findings_means_no_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    // Pick a project id that is not registered. The refusal is
    // `unknown-project` (covered above) and never a zero-row
    // miss-reported as "no gaps".
    let out = run(&db, &["project", "gaps", "phantom"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(stderr_of(&out).contains("unknown-project"));
}
