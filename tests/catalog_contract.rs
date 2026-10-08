//! Project catalog and query contract: `forge project
//! list|inspect|tags|languages` over `forge-project-catalog/0.1.0`.
//!
//! This suite answers the questions the capability names:
//!
//! - A record names the source, revision, observation and freshness it
//!   came from, and the field set is closed.
//! - Sources combine without silent merging: the same project id in two
//!   selected sources stays two records, disambiguated by source.
//! - Filters compose as AND, a repeated value is OR within a predicate,
//!   and ordering is stable by `(project_id, source)` whatever order the
//!   sources arrived in.
//! - An empty catalog, a stale observation and an unreadable source are
//!   explicit states, never zero rows read as an answer.
//! - JSON and NDJSON are the machine contract; the table layout never is.
//!
//! Every case runs through the built binary against local fixtures. No
//! network source is contacted: the Git source reads a local working tree
//! and the GitHub source is honestly unavailable (its adapter is a
//! separate package).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-project-catalog/0.1.0";
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

/// One NDJSON line per record, in the order the contract fixed.
fn run_ndjson(db: &Path, args: &[&str]) -> Vec<Value> {
    let out = run(db, &args_with_format(args, "ndjson"));
    assert!(
        out.status.success(),
        "forge {args:?} failed: {}",
        stderr_of(&out)
    );
    stdout_of(&out)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each ndjson line is one record"))
        .collect()
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

/// A local registry with two managed projects.
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

/// A Workspace Governance registry declaring `alpha` (also local) and a
/// workspace-only `gamma`.
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

/// A portable inventory with one compose-ready and one compose-missing
/// project, plus an entry the inventory contract itself refuses.
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
                },
                {
                    "id": "refused",
                    "repository": "https://example.invalid/refused.git",
                    "revision": "not-a-sha",
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

/// A declared Git working tree read by the `git` source.
fn git_repository(tmp: &Path) -> PathBuf {
    let dir = tmp.join("epsilon");
    fs::create_dir_all(&dir).unwrap();
    let run_git = |args: &[&str]| {
        let status = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .status()
            .expect("git must run");
        assert!(status.success(), "git {args:?} failed");
    };
    run_git(&["init", "-q"]);
    run_git(&["config", "user.email", "forge@example.com"]);
    run_git(&["config", "user.name", "Forge Test"]);
    run_git(&["config", "init.defaultBranch", "main"]);
    run_git(&["checkout", "-q", "-b", "main"]);
    fs::write(dir.join("README.md"), "# epsilon\n").unwrap();
    run_git(&["add", "--", "README.md"]);
    run_git(&["commit", "-q", "-m", "initial"]);
    dir
}

fn records_of(page: &Value) -> &Vec<Value> {
    page["catalog"]["records"]
        .as_array()
        .expect("records array")
}

fn sources_of(page: &Value) -> &Vec<Value> {
    page["catalog"]["sources"]
        .as_array()
        .expect("sources array")
}

fn ids_of(records: &[Value]) -> Vec<String> {
    records
        .iter()
        .map(|record| {
            record["project_id"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

fn keys_of(records: &[Value]) -> Vec<String> {
    records
        .iter()
        .map(|record| {
            format!(
                "{}@{}",
                record["project_id"].as_str().unwrap(),
                record["source"].as_str().unwrap()
            )
        })
        .collect()
}

#[test]
fn the_catalog_advertises_its_grammar_without_creating_a_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let db = db_path(tmp.path());
    let out = run(&db, &["project", "--help"]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    let text = stdout_of(&out);
    for needle in ["list", "inspect", "tags", "languages", CONTRACT] {
        assert!(text.contains(needle), "{needle} missing in:\n{text}");
    }
    let out = run(&db, &["project", "list", "--help"]);
    let text = stdout_of(&out);
    for flag in [
        "--source",
        "--tag",
        "--language",
        "--profile",
        "--lifecycle",
        "--repository",
        "--ci",
        "--compose",
        "--evidence",
        "--filter",
        "--limit",
        "--cursor",
        "--max-age",
    ] {
        assert!(text.contains(flag), "{flag} missing in:\n{text}");
    }
    assert!(!db.exists(), "help must not create a registry");
}

#[test]
fn an_empty_catalog_is_an_empty_page_with_exit_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let db = db_path(tmp.path());
    let page = run_json(&db, &["project", "list"]);
    assert_eq!(page["catalog"]["contract"], CONTRACT);
    assert_eq!(records_of(&page).len(), 0);
    assert_eq!(page["catalog"]["total"], 0);
    // An empty catalog is not an error and is never a silent omission:
    // the selected source is named with a zero count.
    let sources = sources_of(&page);
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["state"], "available");
    assert_eq!(sources[0]["records"], 0);
    assert!(
        !db.exists(),
        "an empty catalog must not create a registry file"
    );
}

#[test]
fn a_record_names_its_source_revision_observation_and_freshness() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let page = run_json(&db, &["project", "list"]);
    let records = records_of(&page);
    assert_eq!(records.len(), 2);
    let alpha = &records[0];
    assert_eq!(alpha["project_id"], "alpha");
    assert_eq!(alpha["source"], "local");
    assert_eq!(alpha["source_kind"], "local");
    assert_eq!(alpha["profile"], "rust-web");
    assert_eq!(alpha["lifecycle"], "L1");
    assert_eq!(alpha["languages"], serde_json::json!(["rust"]));
    assert_eq!(alpha["tags"], serde_json::json!(["auth"]));
    assert_eq!(alpha["evidence"], "present");
    assert_eq!(alpha["freshness"], "current");
    // The observation timestamp is carried, never invented at read time.
    assert!(
        alpha["observed_at"].as_str().unwrap().ends_with('Z'),
        "{alpha}"
    );
}

#[test]
fn the_record_field_set_is_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let page = run_json(&db, &["project", "list"]);
    for record in records_of(&page) {
        let object = record.as_object().unwrap();
        for key in object.keys() {
            assert!(
                !matches!(
                    key.as_str(),
                    "metadata" | "extra" | "payload" | "raw" | "attributes"
                ),
                "record carries a non-contract field `{key}`"
            );
        }
        // No free-form map: every value is a declared scalar, list or enum.
        for (key, value) in object {
            assert!(
                value.is_string() || value.is_array() || value.is_number(),
                "record field `{key}` is not a declared shape: {value}"
            );
        }
    }
}

#[test]
fn json_and_ndjson_carry_the_same_records_in_the_same_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let inv = inventory(tmp.path());
    let args = [
        "project",
        "list",
        "--source",
        "local",
        "--source",
        "inventory",
        "--inventory",
        inv.to_str().unwrap(),
    ];
    let page = run_json(&db, &args);
    let lines = run_ndjson(&db, &args);
    let page_records = records_of(&page);
    assert_eq!(page_records.len(), lines.len());
    for (from_json, from_ndjson) in page_records.iter().zip(lines.iter()) {
        assert_eq!(from_json, from_ndjson);
    }
    // The table layout is not the contract: it carries no machine keys.
    let out = run(&db, &args);
    let table = stdout_of(&out);
    assert!(table.contains("PROJECT"), "{table}");
    assert!(!table.contains("\"contract\""), "{table}");
    assert!(!table.contains("project_id"), "{table}");
}

#[test]
fn sources_combine_without_silent_merging() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let registry = workspace_registry(tmp.path());
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
            registry.to_str().unwrap(),
        ],
    );
    let keys = keys_of(records_of(&page));
    // `alpha` is declared by both sources and stays two records.
    assert_eq!(
        keys,
        vec![
            "alpha@local".to_string(),
            "alpha@workspace-registry".to_string(),
            "beta@local".to_string(),
            "gamma@workspace-registry".to_string(),
        ]
    );
    let alpha_records: Vec<&Value> = records_of(&page)
        .iter()
        .filter(|record| record["project_id"] == "alpha")
        .collect();
    assert_eq!(alpha_records.len(), 2);
    assert_ne!(
        alpha_records[0]["profile"], alpha_records[1]["profile"],
        "the two sources disagree on profile and are never merged"
    );
    // Each selected source reports its own count.
    let counts: Vec<i64> = sources_of(&page)
        .iter()
        .map(|status| status["records"].as_i64().unwrap())
        .collect();
    assert_eq!(counts, vec![2, 2]);
}

#[test]
fn ordering_is_stable_by_project_then_source_whatever_the_selection_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let registry = workspace_registry(tmp.path());
    let inv = inventory(tmp.path());
    let forward = [
        "project",
        "list",
        "--source",
        "local",
        "--source",
        "workspace-registry",
        "--source",
        "inventory",
        "--workspace-registry",
        registry.to_str().unwrap(),
        "--inventory",
        inv.to_str().unwrap(),
    ];
    let mut reverse: Vec<&str> = forward.to_vec();
    // Re-select in the opposite order; the answer must not move.
    let reordered = [
        "project",
        "list",
        "--source",
        "inventory",
        "--source",
        "workspace-registry",
        "--source",
        "local",
        "--workspace-registry",
        registry.to_str().unwrap(),
        "--inventory",
        inv.to_str().unwrap(),
    ];
    let _ = reverse.remove(0);
    let page = run_json(&db, &forward);
    let page_reordered = run_json(&db, &reordered);
    assert_eq!(records_of(&page), records_of(&page_reordered));
    let keys = keys_of(records_of(&page));
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "records are ordered by (project_id, source)");
    assert!(!keys.iter().any(|key| key.starts_with("refused")));
}

#[test]
fn filters_compose_as_and_and_repeated_values_as_or() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let inv = inventory(tmp.path());
    let sources = [
        "project",
        "list",
        "--source",
        "local",
        "--source",
        "inventory",
        "--inventory",
        inv.to_str().unwrap(),
    ];
    // OR inside a predicate.
    let mut args: Vec<&str> = sources.to_vec();
    args.extend(["--profile", "rust-web", "--profile", "rust-product"]);
    assert_eq!(
        ids_of(records_of(&run_json(&db, &args))),
        vec![
            "alpha".to_string(),
            "alpha".to_string(),
            "delta".to_string()
        ]
    );
    // AND across predicates narrows the same set.
    let mut args: Vec<&str> = sources.to_vec();
    args.extend([
        "--profile",
        "rust-product",
        "--compose",
        "compose_ready",
        "--evidence",
        "present",
    ]);
    assert_eq!(ids_of(records_of(&run_json(&db, &args))), vec!["alpha"]);
    // A predicate nothing satisfies is an empty page, not an error.
    let mut args: Vec<&str> = sources.to_vec();
    args.extend(["--lifecycle", "archived"]);
    let page = run_json(&db, &args);
    assert!(records_of(&page).is_empty());
    assert_eq!(page["catalog"]["total"], 0);
    // The generic form is the same predicate.
    let mut args: Vec<&str> = sources.to_vec();
    args.extend(["--filter", "compose=compose_missing"]);
    assert_eq!(ids_of(records_of(&run_json(&db, &args))), vec!["delta"]);
}

#[test]
fn pagination_is_deterministic_and_walks_every_record_once() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let inv = inventory(tmp.path());
    let base = [
        "project",
        "list",
        "--source",
        "local",
        "--source",
        "inventory",
        "--inventory",
        inv.to_str().unwrap(),
    ];
    let page = run_json(&db, &[&base[..], &["--limit", "2"]].concat());
    assert_eq!(page["catalog"]["total"], 4);
    assert_eq!(records_of(&page).len(), 2);
    let cursor = page["catalog"]["next_cursor"]
        .as_str()
        .expect("a next cursor")
        .to_string();
    // The cursor is opaque: the CLI never asks the caller to build one.
    let second = run_json(
        &db,
        &[&base[..], &["--limit", "2", "--cursor", &cursor]].concat(),
    );
    let mut walked = ids_of(records_of(&page));
    walked.extend(ids_of(records_of(&second)));
    // Four records over two pages of two: the last page carries no cursor.
    assert_eq!(walked, vec!["alpha", "alpha", "beta", "delta"]);
    assert!(second["catalog"]["next_cursor"].is_null());
    // Re-reading the same cursor is stable (deterministic pagination).
    let second_again = run_json(
        &db,
        &[&base[..], &["--limit", "2", "--cursor", &cursor]].concat(),
    );
    assert_eq!(records_of(&second), records_of(&second_again));
}

#[test]
fn an_unreadable_source_is_named_and_the_others_still_report() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let broken = tmp.path().join("broken.json");
    fs::write(&broken, "{not json").unwrap();
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
            broken.to_str().unwrap(),
        ],
    );
    // The readable source still answers.
    assert_eq!(records_of(&page).len(), 2);
    // The unreadable one is named with its reason and contributes no rows.
    let statuses = sources_of(&page);
    let unavailable = statuses
        .iter()
        .find(|status| status["source_kind"] == "workspace-registry")
        .expect("the workspace source is reported");
    assert_eq!(unavailable["state"], "unavailable");
    assert_eq!(unavailable["records"], 0);
    assert!(
        unavailable["reason"]
            .as_str()
            .unwrap()
            .contains("malformed JSON"),
        "{unavailable}"
    );
    // No placeholder record is invented for the unreadable source.
    assert!(!records_of(&page)
        .iter()
        .any(|record| record["source_kind"] == "workspace-registry"));
}

#[test]
fn an_unselected_or_unconfigured_source_is_never_zero_rows_read_as_an_answer() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    // The GitHub adapter belongs to a separate package: selecting it is
    // an explicit unavailable state, never an empty success.
    let page = run_json(&db, &["project", "list", "--source", "github"]);
    assert!(records_of(&page).is_empty());
    let statuses = sources_of(&page);
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0]["source_kind"], "github");
    assert_eq!(statuses[0]["state"], "unavailable");
    assert!(statuses[0]["reason"].as_str().unwrap().contains("adapter"));
    // Same for a selected source with nothing declared behind it.
    let page = run_json(&db, &["project", "list", "--source", "git"]);
    assert_eq!(sources_of(&page)[0]["state"], "unavailable");
    assert!(sources_of(&page)[0]["reason"]
        .as_str()
        .unwrap()
        .contains("--git-repository"));
}

#[test]
fn the_git_source_reads_only_a_declared_working_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let repo = git_repository(tmp.path());
    let page = run_json(
        &db,
        &[
            "project",
            "list",
            "--source",
            "git",
            "--git-repository",
            repo.to_str().unwrap(),
        ],
    );
    let records = records_of(&page);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["project_id"], "epsilon");
    assert_eq!(records[0]["source_kind"], "git");
    assert_eq!(records[0]["evidence"], "present");
    assert_eq!(records[0]["source_revision"].as_str().unwrap().len(), 40);
    // A declared path that is not a working tree is named as unavailable,
    // never guessed into a record.
    let page = run_json(
        &db,
        &[
            "project",
            "list",
            "--source",
            "git",
            "--git-repository",
            tmp.path().join("beta").to_str().unwrap(),
        ],
    );
    assert!(records_of(&page).is_empty());
    let status = &sources_of(&page)[0];
    assert_eq!(status["source_kind"], "git");
    assert_eq!(status["state"], "unavailable");
    assert_eq!(status["records"], 0);
    assert!(status["reason"].as_str().unwrap().contains("beta"));
}

#[test]
fn compose_and_ci_are_observed_from_each_registered_project_directory() {
    // `compose` was hard-coded to `None` for the local source even though the
    // detection already existed in `import::detect_docker`, so the documented
    // `--compose` predicate could never match a registered project. `ci` was
    // fed from the registry's `quality_status` — a quality verdict like
    // `warn`/`fail`, not a CI fact — so `--ci` could not match either. Both
    // are facts about the project's own directory and are now observed there.
    let tmp = tempfile::tempdir().unwrap();
    let db = db_path(tmp.path());

    // docker + compose, with GitHub Actions.
    let both = tmp.path().join("both");
    write_project(&both, "both", "rust-web", "L2", "rust");
    fs::write(both.join("Dockerfile"), "FROM scratch").unwrap();
    fs::write(both.join("compose.yaml"), "services: {}").unwrap();
    fs::create_dir_all(both.join(".github/workflows")).unwrap();
    fs::write(both.join(".github/workflows/ci.yml"), "on: push").unwrap();

    // a container file alone is not compose.
    let docker_only = tmp.path().join("docker-only");
    write_project(&docker_only, "docker-only", "rust-web", "L2", "rust");
    fs::write(docker_only.join("Dockerfile"), "FROM scratch").unwrap();

    // neither.
    let bare = tmp.path().join("bare");
    write_project(&bare, "bare", "rust-web", "L1", "rust");

    for dir in [&both, &docker_only, &bare] {
        assert!(run(&db, &["register", dir.to_str().unwrap()])
            .status
            .success());
    }

    let page = run_json(&db, &["project", "list"]);
    let records = records_of(&page);
    let by_id = |id: &str| {
        records
            .iter()
            .find(|r| r["project_id"] == id)
            .unwrap_or_else(|| panic!("no record for {id}: {page}"))
            .clone()
    };

    assert_eq!(by_id("both")["compose"], "docker+compose");
    assert_eq!(by_id("both")["ci"], "github-actions");
    // A Dockerfile is not compose.
    assert_eq!(by_id("docker-only")["compose"], "docker");
    assert!(by_id("docker-only")["ci"].is_null());
    // Neither is reported as absent rather than as an empty string.
    assert!(by_id("bare")["compose"].is_null());
    assert!(by_id("bare")["ci"].is_null());

    // The documented predicate now matches, and only the compose project.
    let page = run_json(&db, &["project", "list", "--compose", "docker+compose"]);
    let ids: Vec<&str> = records_of(&page)
        .iter()
        .map(|r| r["project_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["both"]);

    // …and the CI predicate matches the project that declares CI.
    let page = run_json(&db, &["project", "list", "--ci", "github-actions"]);
    let ids: Vec<&str> = records_of(&page)
        .iter()
        .map(|r| r["project_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["both"]);
}

#[test]
fn inspect_returns_every_record_for_one_project_and_refuses_an_unknown_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let registry = workspace_registry(tmp.path());
    let sources = [
        "project",
        "inspect",
        "alpha",
        "--source",
        "local",
        "--source",
        "workspace-registry",
        "--workspace-registry",
        registry.to_str().unwrap(),
    ];
    let page = run_json(&db, &sources);
    assert_eq!(page["catalog"]["project_id"], "alpha");
    let records = page["catalog"]["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(
        keys_of(records),
        vec!["alpha@local", "alpha@workspace-registry"]
    );
    expect_refusal(
        &db,
        &["project", "inspect", "ghost", "--source", "local"],
        "unknown-project",
    );
}

#[test]
fn tags_and_languages_summarize_the_filtered_catalog() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let tags = run_json(&db, &["project", "tags"]);
    assert_eq!(tags["catalog"]["contract"], CONTRACT);
    let entries = tags["catalog"]["tags"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "auth");
    assert_eq!(entries[0]["count"], 2);
    let languages = run_json(&db, &["project", "languages"]);
    let values: Vec<&str> = languages["catalog"]["languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["value"].as_str().unwrap())
        .collect();
    assert_eq!(values, vec!["python", "rust"]);
    // Filters compose into the summary too.
    let filtered = run_json(&db, &["project", "languages", "--profile", "rust-web"]);
    let values: Vec<&str> = filtered["catalog"]["languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["value"].as_str().unwrap())
        .collect();
    assert_eq!(values, vec!["rust"]);
}

#[test]
fn malformed_filters_and_bounds_are_typed_refusals_with_empty_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    expect_refusal(
        &db,
        &["project", "list", "--filter", "colour=red"],
        "catalog-invalid",
    );
    expect_refusal(
        &db,
        &["project", "list", "--filter", "no-equals"],
        "catalog-invalid",
    );
    expect_refusal(
        &db,
        &["project", "list", "--source", "nope"],
        "catalog-invalid",
    );
    expect_refusal(&db, &["project", "list", "--limit", "0"], "catalog-invalid");
    expect_refusal(
        &db,
        &["project", "list", "--limit", "100000"],
        "catalog-invalid",
    );
    expect_refusal(
        &db,
        &["project", "list", "--cursor", "made-up"],
        "catalog-invalid",
    );
    expect_refusal(
        &db,
        &["project", "list", "--max-age", "0"],
        "catalog-invalid",
    );
}

#[test]
fn a_credential_shaped_value_never_reaches_any_output() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let dir = tmp.path().join("secret-ws");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("projects.json");
    fs::write(
        &path,
        r#"{"schema_version": 1, "projects": [
  {"id": "leak", "path": "leak",
   "profile": "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
   "lifecycle": "active"}
]}"#,
    )
    .unwrap();
    for format in ["table", "json", "ndjson"] {
        let out = run(
            &db,
            &args_with_format(
                &[
                    "project",
                    "list",
                    "--source",
                    "workspace-registry",
                    "--workspace-registry",
                    path.to_str().unwrap(),
                ],
                format,
            ),
        );
        assert!(out.status.success(), "{}", stderr_of(&out));
        let text = stdout_of(&out);
        assert!(
            !text.contains("ghp_abcdefghijklmnopqrstuvwxyz"),
            "{format} leaked a credential shape: {text}"
        );
        assert!(
            text.contains("[REDACTED]"),
            "{format} did not mark the redaction: {text}"
        );
    }
}

#[test]
fn a_stale_observation_is_reported_as_stale_not_as_current() {
    let tmp = tempfile::tempdir().unwrap();
    let db = local_fleet(tmp.path());
    let registry = workspace_registry(tmp.path());
    let args = [
        "project",
        "list",
        "--source",
        "workspace-registry",
        "--workspace-registry",
        registry.to_str().unwrap(),
        "--max-age",
        "1",
    ];
    let page = run_json(&db, &args);
    let statuses = sources_of(&page);
    assert_eq!(statuses[0]["state"], "available");
    // A registry document written "now" is fresh only inside the window.
    let stale = run_json(
        &db,
        &[
            "project",
            "list",
            "--source",
            "workspace-registry",
            "--workspace-registry",
            registry.to_str().unwrap(),
            "--max-age",
            "86400",
        ],
    );
    assert_eq!(sources_of(&stale)[0]["state"], "available");
    assert_eq!(
        records_of(&stale).len(),
        records_of(&page).len(),
        "widening the window never adds or drops a record"
    );
}
