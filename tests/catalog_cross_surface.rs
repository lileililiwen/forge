//! Cross-surface regression for the project catalog.
//!
//! This suite answers the questions a per-surface contract cannot:
//!
//! - **A query changes nothing.** The catalog opens the registry
//!   read-only, so no registry byte, table row or journal row moves —
//!   asserted against the database file itself, not a surface's word.
//! - **A read never creates what it looked for.** A catalog query
//!   against a missing registry leaves it missing.
//! - **No parent directory is scanned implicitly.** A sibling project
//!   that was never declared or registered is never discovered.
//! - **The catalog is not a second registry.** No new table appears, and
//!   the existing `forge list` projection is byte-identical before and
//!   after a catalog read.
//! - **Provenance survives every surface.** The same record content is
//!   answerable from the table, the JSON page and NDJSON, and an
//!   unavailable source is never presented as a healthy one.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut all: Vec<&str> = args.to_vec();
    all.extend(["--format", "json"]);
    let out = run(db, &all);
    assert!(
        out.status.success(),
        "forge {args:?}: {}",
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stdout={}", lossy(&out.stdout)))
}

/// Raw connection: the catalog's read-only promise is checked against
/// the database, not against a rendered surface.
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
        format!("schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"),
    )
    .unwrap();
}

/// A registered project plus an unregistered sibling next to it.
fn fixture(tmp: &Path) -> (PathBuf, PathBuf) {
    let db = tmp.join("registry.db");
    let alpha = tmp.join("alpha");
    let beta = tmp.join("beta");
    let sibling = tmp.join("sibling");
    write_project(&alpha, "alpha");
    write_project(&beta, "beta");
    // A perfectly valid project that was never registered: only an
    // implicit parent-directory scan could surface it.
    write_project(&sibling, "sibling");
    assert!(run(&db, &["register", alpha.to_str().unwrap()])
        .status
        .success());
    assert!(run(&db, &["register", beta.to_str().unwrap()])
        .status
        .success());
    (db, sibling)
}

#[test]
fn a_catalog_query_writes_no_registry_byte_table_or_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _sibling) = fixture(tmp.path());
    let before_bytes = fs::read(&db).expect("registry bytes");
    let before_tables = table_names(&db);
    let before_projects = scalar(&db, "SELECT COUNT(*) FROM projects");
    let before_operations = scalar(&db, "SELECT COUNT(*) FROM operations");
    assert!(before_operations > 0, "registration journalled a row");

    for args in [
        vec!["project", "list"],
        vec!["project", "inspect", "alpha"],
        vec!["project", "tags"],
        vec!["project", "languages"],
        vec!["project", "list", "--source", "local", "--source", "github"],
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
fn the_catalog_adds_no_table_to_the_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _sibling) = fixture(tmp.path());
    let before = table_names(&db);
    let _ = run(&db, &["project", "list"]);
    let _ = run(&db, &["project", "list", "--format", "ndjson"]);
    assert_eq!(table_names(&db), before);
    // The catalog is a projection: it stores nothing, so there is no
    // catalog table to be created, migrated or rolled back.
    assert!(
        !before.iter().any(|name| name.contains("catalog")),
        "a catalog table appeared: {before:?}"
    );
}

#[test]
fn a_read_never_creates_the_registry_it_looked_for() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("absent.db");
    let out = run(&db, &["project", "list", "--format", "json"]);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    let page: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(page["catalog"]["records"].as_array().unwrap().len(), 0);
    assert_eq!(page["catalog"]["sources"][0]["state"], "available");
    assert!(
        !db.exists(),
        "an empty catalog must not create a registry file on disk"
    );
}

#[test]
fn no_parent_directory_or_sibling_checkout_is_scanned_implicitly() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, sibling) = fixture(tmp.path());
    // The sibling is a valid Forge project in the parent directory and
    // is not registered anywhere the catalog was told to read.
    assert!(sibling.join("forge.yaml").is_file());
    let page = run_json(&db, &["project", "list"]);
    let ids: Vec<&str> = page["catalog"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["project_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["alpha", "beta"]);
    assert!(!ids.contains(&"sibling"));
    // The only source consulted is the one that was selected.
    let sources = page["catalog"]["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["source"], "local");
}

#[test]
fn the_existing_registry_projection_is_byte_identical_after_a_catalog_read() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _sibling) = fixture(tmp.path());
    let before = run(&db, &["list", "--format", "json"]).stdout;
    let _ = run(&db, &["project", "list", "--format", "ndjson"]);
    let _ = run(&db, &["project", "inspect", "alpha"]);
    let after = run(&db, &["list", "--format", "json"]).stdout;
    assert_eq!(lossy(&before), lossy(&after));
    // The catalog adds no catalog fields to the existing projection.
    assert!(!lossy(&after).contains("catalog"));
    assert!(!lossy(&after).contains("source_kind"));
}

#[test]
fn the_same_records_are_answerable_from_every_output_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _sibling) = fixture(tmp.path());
    let mut json_args = vec!["project", "list", "--format", "json"];
    let page: Value = serde_json::from_slice(&run(&db, &json_args).stdout).expect("json");
    let ndjson = run(&db, &["project", "list", "--format", "ndjson"]);
    let lines: Vec<Value> = lossy(&ndjson.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("one record per line"))
        .collect();
    json_args.clear();
    assert_eq!(page["catalog"]["records"].as_array().unwrap(), &lines);
    // The table names the same projects and the same contract.
    let table = lossy(&run(&db, &["project", "list"]).stdout);
    assert!(table.contains("forge-project-catalog/0.1.0"), "{table}");
    for record in &lines {
        assert!(
            table.contains(record["project_id"].as_str().unwrap()),
            "the table is missing {}: {table}",
            record["project_id"]
        );
    }
}

#[test]
fn an_unavailable_source_is_never_presented_as_a_healthy_one() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _sibling) = fixture(tmp.path());
    let missing = tmp.path().join("no-such-registry.json");
    let page = run_json(
        &db,
        &[
            "project",
            "list",
            "--source",
            "local",
            "--source",
            "workspace-registry",
            "--workspace-registry",
            missing.to_str().unwrap(),
        ],
    );
    let statuses = page["catalog"]["sources"].as_array().unwrap();
    let healthy: Vec<&Value> = statuses
        .iter()
        .filter(|status| status["state"] == "available")
        .collect();
    let unhealthy: Vec<&Value> = statuses
        .iter()
        .filter(|status| status["state"] == "unavailable")
        .collect();
    assert_eq!(healthy.len(), 1);
    assert_eq!(healthy[0]["source"], "local");
    assert_eq!(unhealthy.len(), 1);
    assert_eq!(unhealthy[0]["records"], 0);
    assert!(unhealthy[0]["reason"].as_str().is_some());
    // No record is invented to stand in for the source that could not be
    // read, and the readable source still answers.
    assert_eq!(page["catalog"]["records"].as_array().unwrap().len(), 2);
    assert!(!page["catalog"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|record| record["source_kind"] == "workspace-registry"));
}

#[test]
fn repeated_reads_of_unchanged_sources_are_byte_identical() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _sibling) = fixture(tmp.path());
    let first = run(&db, &["project", "list", "--format", "json"]).stdout;
    let second = run(&db, &["project", "list", "--format", "json"]).stdout;
    // The page carries the query instant; everything it answers with is
    // a stable projection of unchanged sources.
    let normalize = |bytes: &[u8]| {
        let mut value: Value = serde_json::from_slice(bytes).expect("json");
        value["catalog"]["observed_at"] = Value::String("<query-instant>".to_string());
        value.to_string()
    };
    assert_eq!(normalize(&first), normalize(&second));
}

#[test]
fn a_catalog_refusal_writes_nothing_either() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _sibling) = fixture(tmp.path());
    let before = fs::read(&db).expect("registry bytes");
    let out = run(&db, &["project", "list", "--filter", "colour=red"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "refusal printed stdout");
    assert!(lossy(&out.stderr).contains("catalog-invalid"));
    assert_eq!(fs::read(&db).expect("registry bytes"), before);
}
