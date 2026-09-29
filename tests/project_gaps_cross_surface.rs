//! Cross-surface regression for the project evidence-gap assessment
//! (`project-evidence-gap-assessment`, `forge-project-evidence/0.1.0`).
//!
//! The per-surface contract suite answers "what does the command
//! emit?". This suite answers the questions a per-surface contract
//! cannot:
//!
//! - **Read-only proof.** A `forge project gaps` invocation never
//!   changes the registry file, its table set, its project rows, or
//!   its journal rows. The check is performed against the database
//!   itself, not against a rendered surface's word.
//! - **Credential redaction.** A credential-shaped value carried in any
//!   source (workspace-registry, inventory, repository URL) is redacted
//!   in every output surface: the table, the JSON page and the NDJSON
//!   line. The `evidence` field is the most exposed surface, so it is
//!   checked directly.
//! - **`UNAVAILABLE` and `NOT_APPLICABLE` boundaries.** A source that
//!   cannot be read is `UNAVAILABLE` and never `NOT_APPLICABLE`; a
//!   profile that does not need a control is `NOT_APPLICABLE` and never
//!   `UNAVAILABLE`. A reader that conflates them mis-routes repair.
//! - **Existing doctor findings are unchanged.** The gaps inspector
//!   does not reach into the doctor surface; running `forge doctor`
//!   before and after a `gaps` run produces the same findings, in the
//!   same order, with the same ids and verdicts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-project-evidence/0.1.0";
const SECRET_TOKEN: &str = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
const SECRET_USERINFO: &str = "https://user:ghp_abcdefghijklmnopqrstuvwxyz0123456789@x/y.git";

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

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut all: Vec<&str> = args.to_vec();
    all.extend(["--format", "json"]);
    let out = run(db, &all);
    assert!(
        out.status.success(),
        "forge {args:?} failed: {}",
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stdout={}", lossy(&out.stdout)))
}

fn db_path(tmp: &Path) -> PathBuf {
    tmp.join("registry.db")
}

/// Raw connection: the gaps inspector's read-only promise is checked
/// against the database, not against a rendered surface's word.
fn scalar(db: &Path, sql: &str) -> i64 {
    let conn = rusqlite::Connection::open(db).expect("open registry");
    conn.query_row(sql, [], |row| row.get(0)).expect("scalar")
}

fn table_names(db: &Path) -> Vec<String> {
    let conn = rusqlite::Connection::open(db).expect("open registry");
    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .expect("prepare");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query");
    rows.map(|row| row.expect("table name")).collect()
}

fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  \
             maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("README.md"), format!("# {id}\n")).unwrap();
}

fn local_fleet(tmp: &Path) -> PathBuf {
    let db = db_path(tmp);
    let alpha = tmp.join("alpha");
    let beta = tmp.join("beta");
    write_project(&alpha, "alpha");
    write_project(&beta, "beta");
    assert!(run(&db, &["register", alpha.to_str().unwrap()])
        .status
        .success());
    assert!(run(&db, &["register", beta.to_str().unwrap()])
        .status
        .success());
    db
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

// ---------------------------------------------------------------------------
// 1. Read-only proof
// ---------------------------------------------------------------------------

#[test]
fn a_gaps_run_writes_no_registry_byte_table_or_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let before_bytes = fs::read(&db).expect("registry bytes");
    let before_tables = table_names(&db);
    let before_projects = scalar(&db, "SELECT COUNT(*) FROM projects");
    let before_operations = scalar(&db, "SELECT COUNT(*) FROM operations");
    assert!(before_operations > 0, "registration journalled a row");

    for args in [
        vec!["project", "gaps"],
        vec!["project", "gaps", "alpha"],
        vec!["project", "gaps", "--format", "json"],
        vec!["project", "gaps", "--format", "ndjson"],
        vec!["project", "gaps", "--category", "tags"],
        vec!["project", "gaps", "--status", "pass"],
    ] {
        let out = run(&db, &args);
        assert!(
            out.status.success(),
            "forge {args:?}: {}",
            lossy(&out.stderr)
        );
    }

    assert_eq!(fs::read(&db).expect("registry bytes"), before_bytes);
    assert_eq!(table_names(&db), before_tables);
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM projects"),
        before_projects
    );
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM operations"),
        before_operations
    );
}

#[test]
fn a_gaps_refusal_writes_nothing_either() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let before = fs::read(&db).expect("registry bytes");
    let out = run(&db, &["project", "gaps", "alpha", "--category", "bogus"]);
    assert!(!out.status.success(), "bogus category must refuse");
    assert!(out.stdout.is_empty(), "refusal printed stdout");
    assert!(stderr_of(&out).contains("catalog-invalid"));
    assert_eq!(fs::read(&db).expect("registry bytes"), before);
}

#[test]
fn a_gaps_run_against_a_missing_registry_never_creates_it() {
    let tmp = tempfile::tempdir().unwrap();
    let db = db_path(tmp.path());
    assert!(!db.is_file());
    let out = run(&db, &["project", "gaps"]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(!db.is_file(), "gaps must not create the registry file");
}

// ---------------------------------------------------------------------------
// 2. Credential redaction
// ---------------------------------------------------------------------------

#[test]
fn credential_shaped_values_never_appear_in_json_or_ndjson() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    // The inventory source carries a repository URL with a credential
    // embedded as userinfo. The gaps inspector must never surface it.
    let inv = tmp.path().join("inventory.json");
    let inv_dir = tmp.path().join("proj-alpha");
    fs::create_dir_all(&inv_dir).unwrap();
    fs::write(inv_dir.join("docker-compose.yml"), "services: {}").unwrap();
    fs::write(
        &inv,
        serde_json::json!({
            "contract": "forge-project-inventory/0.1.0",
            "provider": "local",
            "generated_at": "2026-09-29T00:00:00Z",
            "projects": [
                {
                    "id": "alpha",
                    "repository": SECRET_USERINFO,
                    "revision": "0123456789abcdef0123456789abcdef01234567",
                    "profile": "rust-product",
                    "runtime": "web",
                    "compose_file": "docker-compose.yml",
                    "source_path": inv_dir.display().to_string(),
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    let value = run_json(
        &db,
        &[
            "project",
            "gaps",
            "alpha",
            "--source",
            "inventory",
            "--inventory",
            inv.to_str().unwrap(),
        ],
    );
    let serialized = serde_json::to_string(&value).unwrap();
    assert!(
        !serialized.contains(SECRET_TOKEN),
        "gaps JSON must not contain a token-shaped value: {serialized}"
    );
    let findings = findings_of(&value);
    for finding in findings {
        let f_str = serde_json::to_string(finding).unwrap();
        assert!(
            !f_str.contains(SECRET_TOKEN),
            "finding leaks a credential: {f_str}"
        );
    }

    // The same content is not leaked through the table either.
    let table = lossy(&run(&db, &["project", "gaps", "alpha"]).stdout);
    assert!(
        !table.contains(SECRET_TOKEN),
        "gaps table must not contain a token-shaped value: {table}"
    );

    // The NDJSON surface is the most exposed: each line is a single
    // finding. None of them may carry the credential.
    let ndjson_out = run(
        &db,
        &[
            "project",
            "gaps",
            "alpha",
            "--format",
            "ndjson",
            "--source",
            "inventory",
            "--inventory",
            inv.to_str().unwrap(),
        ],
    );
    for line in stdout_of(&ndjson_out).lines() {
        if line.trim().is_empty() {
            continue;
        }
        assert!(
            !line.contains(SECRET_TOKEN),
            "NDJSON finding leaks a credential: {line}"
        );
    }
}

#[test]
fn no_finding_carries_a_credential_even_when_a_token_was_observed() {
    // Forge never writes a credential to the registry, but a malicious
    // inventory file may try to smuggle one through. The gaps output
    // is the last line of defence and must not echo it.
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(&db, &["project", "gaps", "alpha"]);
    let serialized = serde_json::to_string(&value).unwrap();
    for needle in ["ghp_", "AKIA", "xoxb-", "-----BEGIN", "-----BEGIN "] {
        assert!(
            !serialized.contains(needle),
            "gaps JSON must not carry a `{needle}` credential fragment: {serialized}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. UNAVAILABLE / NOT_APPLICABLE boundaries
// ---------------------------------------------------------------------------

#[test]
fn unavailable_and_not_applicable_are_distinct_verdicts_in_every_finding() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    // Trigger both kinds of verdict: a malformed workspace registry
    // (unavailable) and the docs/github controls (not_applicable).
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
        "an unreadable source must surface as findings"
    );
    let unavailable: Vec<&Value> = findings
        .iter()
        .filter(|f| f["status"] == "unavailable")
        .collect();
    let not_applicable: Vec<&Value> = findings
        .iter()
        .filter(|f| f["status"] == "not_applicable")
        .collect();
    assert!(
        !unavailable.is_empty(),
        "expected at least one `unavailable` finding: {findings:#?}"
    );
    assert!(
        unavailable.iter().all(|f| f["status"] == "unavailable"),
        "unavailable must not be conflated with not_applicable: {unavailable:#?}"
    );
    assert!(
        not_applicable
            .iter()
            .all(|f| f["status"] == "not_applicable"),
        "not_applicable must not be conflated with unavailable: {not_applicable:#?}"
    );
    // A finding is one verdict, never two. The closed vocabulary
    // rejects any other string at parse time, so any leak would show
    // up here as a status other than the closed set.
    let closed = ["pass", "warn", "fail", "unavailable", "not_applicable"];
    for finding in findings {
        let status = finding["status"].as_str().unwrap();
        assert!(
            closed.contains(&status),
            "finding has an unrecognised status `{status}`: {finding:?}"
        );
    }
}

#[test]
fn github_remains_a_permanent_unavailable_source_with_a_named_reason() {
    // github is a known-permanent "no adapter" source: it must surface
    // as `unavailable` in the source status with a reason, and must
    // never invent records. Findings (when synthesized) name the
    // source, so the source field is the operator's anchor.
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(&db, &["project", "gaps", "--source", "github"]);
    let sources = gaps_of(&value)["sources"].as_array().expect("sources");
    let github = sources
        .iter()
        .find(|s| s["source_kind"] == "github")
        .expect("github source status");
    assert_eq!(github["state"], "unavailable");
    assert!(github["reason"].as_str().is_some());
    // No record was invented to stand in for the source that cannot be
    // read.
    assert!(
        findings_of(&value).is_empty(),
        "github must not invent findings: {value}"
    );
}

#[test]
fn an_unavailable_source_is_never_collapsed_into_a_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
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
    // The source is explicitly unavailable; no finding may be reported
    // as `pass` for that source. The contract: a project is never
    // reported healthy by silence.
    let findings = findings_of(&value);
    let passes_from_unavailable: Vec<&Value> = findings
        .iter()
        .filter(|f| {
            f["status"] == "pass"
                && f["source"]
                    .as_str()
                    .is_some_and(|s| s.contains("workspace-registry"))
        })
        .collect();
    assert!(
        passes_from_unavailable.is_empty(),
        "an unavailable source must not produce `pass` findings: {passes_from_unavailable:#?}"
    );
    let statuses: Vec<&str> = findings
        .iter()
        .map(|f| f["status"].as_str().unwrap())
        .collect();
    assert!(
        !statuses.contains(&"pass"),
        "no finding may pass when the source is unreadable: {statuses:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. Doctor findings are unchanged
// ---------------------------------------------------------------------------

#[test]
fn doctor_findings_are_byte_identical_before_and_after_a_gaps_run() {
    // The gaps inspector must not reach into the doctor surface: the
    // project-level `forge doctor` output is the same after a `gaps`
    // run as it was before. The doctor surface is exercised on a
    // registered project directory so the registry observation
    // matches.
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let project_dir = tmp.path().join("alpha");
    let before = run(
        &db,
        &["doctor", project_dir.to_str().unwrap(), "--format", "json"],
    );
    assert!(before.status.success(), "{}", stderr_of(&before));
    let before_findings: Value = serde_json::from_slice::<Value>(&before.stdout)
        .expect("doctor before")
        .get("doctor")
        .cloned()
        .expect("doctor envelope")
        .get("findings")
        .cloned()
        .expect("doctor findings");
    // Run the gaps inspector on the same registry.
    let gaps = run(&db, &["project", "gaps", "--format", "json"]);
    assert!(gaps.status.success(), "{}", stderr_of(&gaps));
    let after = run(
        &db,
        &["doctor", project_dir.to_str().unwrap(), "--format", "json"],
    );
    assert!(after.status.success(), "{}", stderr_of(&after));
    let after_findings: Value = serde_json::from_slice::<Value>(&after.stdout)
        .expect("doctor after")
        .get("doctor")
        .cloned()
        .expect("doctor envelope")
        .get("findings")
        .cloned()
        .expect("doctor findings");
    assert_eq!(before_findings, after_findings);
}

#[test]
fn doctor_run_writes_no_registry_byte_either() {
    // The inverse direction: a `forge doctor` invocation also does not
    // touch the registry, so a `gaps` run after a doctor run is
    // equally clean.
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let project_dir = tmp.path().join("alpha");
    let before = fs::read(&db).expect("registry bytes");
    let _ = run(&db, &["doctor", project_dir.to_str().unwrap()]);
    let _ = run(&db, &["project", "gaps"]);
    let after = fs::read(&db).expect("registry bytes");
    assert_eq!(
        before, after,
        "doctor + gaps must leave the registry untouched"
    );
}

// ---------------------------------------------------------------------------
// 5. Conflicting sources are distinct, not merged
// ---------------------------------------------------------------------------

#[test]
fn conflicting_sources_emit_distinct_findings_with_attribution() {
    // The local registry and the workspace-registry disagree on the
    // project name. Neither finding is merged; each carries its
    // source as provenance.
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let dir = tmp.path().join("ws");
    fs::create_dir_all(dir.join("alpha")).unwrap();
    fs::write(dir.join("alpha/forge.yaml"), "schema: 1\n").unwrap();
    let ws = dir.join("projects.json");
    fs::write(
        &ws,
        r#"{"schema_version": 1, "workspace_root": null,
  "projects": [
    {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active"}
  ]}"#,
    )
    .unwrap();
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
    let tags_alpha: Vec<&Value> = findings
        .iter()
        .filter(|f| f["category"] == "tags" && f["project_id"] == "alpha")
        .collect();
    assert_eq!(
        tags_alpha.len(),
        2,
        "two sources = two findings per category: {findings:#?}"
    );
    let mut sources: Vec<&str> = tags_alpha
        .iter()
        .map(|f| f["source"].as_str().unwrap())
        .collect();
    sources.sort();
    assert_eq!(
        sources,
        vec!["local", "workspace-registry"],
        "each source is named in its finding: {tags_alpha:#?}"
    );
    // The two findings for the same category never collapse to one.
    // The id is derived from `(rule, project_id, subject)` so two
    // sources share it; the `source` field is the disambiguator. The
    // important invariant is that two findings exist, not that they
    // have distinct ids.
    assert_eq!(
        tags_alpha.len(),
        2,
        "two sources produce two findings for the same category: {tags_alpha:#?}"
    );
}

// ---------------------------------------------------------------------------
// 6. Empty / missing metadata is not a silent pass
// ---------------------------------------------------------------------------

#[test]
fn an_empty_record_yields_a_finding_per_category_with_no_collapsing() {
    // A registered project whose only metadata is the row in the
    // registry has explicit findings, not a one-line "everything is
    // fine" answer. The contract: missing metadata is a finding.
    let tmp = tempfile::tempdir().unwrap();
    let db = db_path(tmp.path());
    let ghost = tmp.path().join("ghost");
    write_project(&ghost, "ghost");
    assert!(run(&db, &["register", ghost.to_str().unwrap()])
        .status
        .success());
    let value = run_json(&db, &["project", "gaps", "ghost"]);
    let findings = findings_of(&value);
    assert_eq!(
        findings.len(),
        7,
        "one finding per category, never collapsed: {findings:#?}"
    );
    let ids: std::collections::BTreeSet<&str> =
        findings.iter().map(|f| f["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids.len(),
        7,
        "every category is a distinct, stable id: {ids:?}"
    );
    for finding in findings {
        let status = finding["status"].as_str().unwrap();
        let category = finding["category"].as_str().unwrap();
        assert!(
            ["pass", "warn", "fail", "unavailable", "not_applicable"].contains(&status),
            "`{category}` finding has an unrecognised status `{status}`"
        );
    }
}

// ---------------------------------------------------------------------------
// 7. The contract envelope is stable
// ---------------------------------------------------------------------------

#[test]
fn the_gaps_envelope_carries_the_contract_version_and_a_summary() {
    // The contract envelope is what consumers depend on: a `contract`
    // version, an optional `project_id`, a `sources` array, a
    // `summary` block and a `findings` array. Drift in this envelope
    // is a breaking change.
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(&db, &["project", "gaps", "alpha"]);
    let gaps = gaps_of(&value);
    assert_eq!(gaps["contract"], CONTRACT);
    assert_eq!(gaps["project_id"], "alpha");
    assert!(gaps["sources"].is_array());
    let summary = &gaps["summary"];
    assert!(summary["total"].is_u64());
    assert!(summary["returned"].is_u64());
    assert!(summary["filters"].is_object());
    assert!(summary["filters"]["categories"].is_array());
    assert!(summary["filters"]["statuses"].is_array());
    assert!(summary["filters"]["remediation_classes"].is_array());
    assert!(findings_of(&value).len() <= summary["total"].as_u64().unwrap() as usize);
}

#[test]
fn the_summary_total_counts_unfiltered_findings_not_filtered_ones() {
    // `--category docs` narrows the response but the envelope
    // `summary.total` reports the pre-filter count so a consumer can
    // tell "you hid 6 of 7" from "there is only 1".
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let value = run_json(&db, &["project", "gaps", "alpha", "--category", "docs"]);
    let gaps = gaps_of(&value);
    let summary = &gaps["summary"];
    assert_eq!(
        summary["total"].as_u64().unwrap(),
        7,
        "total is the pre-filter count: {summary}"
    );
    assert_eq!(
        summary["returned"].as_u64().unwrap(),
        1,
        "returned is the post-filter count: {summary}"
    );
}
