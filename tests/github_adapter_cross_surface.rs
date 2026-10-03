//! Cross-surface regression for the GitHub metadata adapter
//! (`github-project-metadata-adapter`).
//!
//! The per-surface contract verifies the `forge-github-metadata/0.1.0`
//! surface in isolation. This suite verifies the properties a single
//! surface can never prove on its own:
//!
//! - **Reads are side-effect-free.** `forge project list --source github`
//!   does not write a single byte to the registry, does not add a table,
//!   does not append an operations-journal row, and does not create a
//!   project that was not there before.
//! - **The token is never echoed.** A credential-shaped value that flows
//!   into the adapter through the environment never reappears in
//!   stdout, stderr or any human-facing report. The redactor is the
//!   same policy used for GitHub-native fields (description) and for
//!   every other surface (CLI, journal, human pages).
//! - **Namespaces stay separate.** The closed record key set never
//!   gains a GitHub-only field: a GitHub record carries the same
//!   `tags` field as every other record (always empty for a GitHub
//!   record) and never receives a `topics` or `releases` key on the
//!   record itself. Those namespaces live in the `observations` payload
//!   of the JSON document and the human-facing report only.
//! - **No live host is contacted.** Every fixture is a local shell
//!   stub on a controlled `PATH`; the only network-shaped strings are
//!   `github.com` literals and the stub never opens a socket.
//! - **No schema drift.** A GitHub read leaves the registry schema
//!   byte-identical: the same set of tables, the same set of columns,
//!   the same set of indexes. The catalog is a projection, not a
//!   second registry, and a GitHub row lives in memory only.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-github-metadata/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd(bins: &Path) -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("FORGE_INVENTORY_SOURCE")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("FORGE_GITHUB_BIN")
        .env_remove("FORGE_GITHUB_TOKEN");
    // Compose a PATH that exposes the stub bins in front of the
    // parent PATH so the GitHub adapter stub shadows the real
    // `forge-github-metadata-adapter` while the system `git` and
    // other tools remain reachable.
    let parent = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![bins.to_path_buf()];
    for entry in std::env::split_paths(&parent) {
        paths.push(entry);
    }
    let joined = std::env::join_paths(paths).expect("join PATH");
    cmd.env("PATH", joined);
    cmd
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

fn cmd_json(bins: &Path, db: &Path, args: &[&str]) -> Command {
    let mut cmd = clean_cmd(bins);
    cmd.arg("--format").arg("json").arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

fn cmd_human(bins: &Path, db: &Path, args: &[&str]) -> Command {
    let mut cmd = clean_cmd(bins);
    cmd.arg("--format").arg("human").arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

/// Plant an observing stub that echoes the supplied JSON document
/// and a proposing stub that returns a closed-vocabulary success. The
/// observe stub also refuses to echo `FORGE_GITHUB_TOKEN` so a leaked
/// token can be spotted as a stub regression.
fn github_stubs(bins: &Path, observation: &str, proposal: &str) {
    write_stub(
        bins,
        "forge-github-metadata-adapter",
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"observe\" ]; then\n\
             \x20if [ -z \"${{FORGE_GITHUB_TOKEN:-}}\" ]; then exit 65; fi\n\
             \x20if printf '%s' \"${{FORGE_GITHUB_TOKEN}}\" | grep -qE 'gh[ps]_[A-Za-z0-9]{{20,}}'; then\n\
             \x20\x20echo 'token-shape leaked' 1>&2\n\
             \x20\x20exit 66\n\
             \x20fi\n\
             \x20printf '%s' '{observation}'\n\
             \x20exit 0\n\
             fi\n\
             if [ \"$1\" = \"propose\" ]; then\n\
             \x20if [ -z \"${{FORGE_GITHUB_TOKEN:-}}\" ]; then exit 65; fi\n\
             \x20printf '%s' '{proposal}'\n\
             \x20exit 0\n\
             fi\n\
             exit 64\n"
        ),
    );
}

fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let body = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), body).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!("[package]\nname = \"{id}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .unwrap();
}

/// Initialize a git working tree with an origin remote pointing at a
/// GitHub URL so the local registry's `git_remote` column matches a
/// walk the GitHub source can perform.
fn init_git(dir: &Path, github_url: &str) {
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let init = Command::new("git")
        .arg("-C")
        .arg(&canonical)
        .args(["init", "-q"])
        .output()
        .expect("git init");
    assert!(init.status.success(), "git init: {}", lossy(&init.stderr));
    let config_email = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "user.email", "ci@example.com"])
        .output()
        .expect("git config user.email");
    assert!(
        config_email.status.success(),
        "git config email: {}",
        lossy(&config_email.stderr)
    );
    let config_name = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "user.name", "Forge CI"])
        .output()
        .expect("git config user.name");
    assert!(
        config_name.status.success(),
        "git config name: {}",
        lossy(&config_name.stderr)
    );
    let remote = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["remote", "add", "origin", github_url])
        .output()
        .expect("git remote add");
    assert!(
        remote.status.success(),
        "git remote add: {}",
        lossy(&remote.stderr)
    );
    let add = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["add", "-A"])
        .output()
        .expect("git add");
    assert!(add.status.success(), "git add: {}", lossy(&add.stderr));
    let commit = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["commit", "-q", "-m", "init"])
        .output()
        .expect("git commit");
    assert!(
        commit.status.success(),
        "git commit: {}",
        lossy(&commit.stderr)
    );
}

fn fixture(tmp: &Path) -> (PathBuf, PathBuf) {
    let db = tmp.join("registry.db");
    let alpha = tmp.join("alpha");
    let beta = tmp.join("beta");
    // Registered projects whose `git_remote` points at a GitHub URL
    // so the GitHub source can walk the registry without an explicit
    // `--github-repository` flag.
    write_project(&alpha, "alpha");
    write_project(&beta, "beta");
    init_git(&alpha, "https://github.com/octocat/hello-world.git");
    init_git(&beta, "https://github.com/octocat/spoon-knife.git");
    let mut register = clean_cmd(&alpha.parent().unwrap());
    register
        .arg("--registry")
        .arg(&db)
        .arg("register")
        .arg(&alpha);
    let out = register.output().expect("register alpha");
    assert!(
        out.status.success(),
        "register alpha failed: {}",
        lossy(&out.stderr)
    );
    let mut register = clean_cmd(&alpha.parent().unwrap());
    register
        .arg("--registry")
        .arg(&db)
        .arg("register")
        .arg(&beta);
    let out = register.output().expect("register beta");
    assert!(
        out.status.success(),
        "register beta failed: {}",
        lossy(&out.stderr)
    );
    (db, alpha)
}

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

fn index_names(db: &Path) -> Vec<String> {
    let conn = rusqlite::Connection::open(db).expect("open registry");
    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index' ORDER BY name")
        .expect("prepare");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query");
    rows.map(|row| row.expect("index name")).collect()
}

fn project_columns(db: &Path) -> Vec<String> {
    let conn = rusqlite::Connection::open(db).expect("open registry");
    let mut statement = conn
        .prepare("PRAGMA table_info(projects)")
        .expect("prepare");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query");
    rows.map(|row| row.expect("column name")).collect()
}

/// A GitHub read through `forge project list --source github` does
/// not mutate the registry: same bytes, same tables, same columns,
/// same indexes, same number of project rows, same number of
/// operations-journal rows. The catalog is a projection, not a write.
#[test]
fn a_github_read_writes_no_registry_byte_table_column_index_or_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let observation = format!(
        r#"{{"contract":"{CONTRACT}","host":"github.com","repository":"octocat/hello-world","source_revision":"abc","observed_at":"2026-09-29T11:00:00Z","state":"current","topics":["rust","ci"],"languages":["Rust"],"workflows":["ci.yml"],"releases":["v1.0.0"],"custom_properties":[],"archived":false,"note":""}}"#
    );
    let proposal = r#"{"contract":"forge-github-metadata/0.1.0","mode":"pull-request","repository":"octocat/hello-world","pr_url":"https://github.com/octocat/hello-world/pull/1"}"#;
    github_stubs(&bins, &observation, proposal);
    let (db, _alpha) = fixture(tmp.path());
    let before_bytes = fs::read(&db).expect("registry bytes");
    let before_tables = table_names(&db);
    let before_columns = project_columns(&db);
    let before_indexes = index_names(&db);
    let before_projects = scalar(&db, "SELECT COUNT(*) FROM projects");
    let before_operations = scalar(&db, "SELECT COUNT(*) FROM operations");
    assert!(before_operations > 0, "registration journalled a row");
    for args in [
        vec!["project", "list", "--source", "github"],
        vec!["project", "list", "--source", "local", "--source", "github"],
        vec![
            "project", "list", "--source", "github", "--format", "ndjson",
        ],
        vec![
            "project",
            "list",
            "--source",
            "github",
            "--github-repository",
            "octocat/hello-world",
        ],
    ] {
        let mut cmd = cmd_json(&bins, &db, &args);
        cmd.env(
            "FORGE_GITHUB_BIN",
            bins.join("forge-github-metadata-adapter"),
        )
        .env(
            "FORGE_GITHUB_TOKEN",
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
        );
        let output = cmd.output().expect("run forge");
        assert!(
            output.status.success(),
            "forge {args:?}: {}",
            lossy(&output.stderr)
        );
    }
    // The read also has to be safe through the dedicated
    // `forge project github observe` surface.
    let mut cmd = cmd_human(
        &bins,
        &db,
        &["project", "github", "observe", "octocat/hello-world"],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env(
        "FORGE_GITHUB_TOKEN",
        "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
    );
    let out = cmd.output().expect("run forge");
    assert!(
        out.status.success(),
        "github observe: {}",
        lossy(&out.stderr)
    );
    // And a propose that runs in PR mode must not mutate either.
    let mut cmd = cmd_json(
        &bins,
        &db,
        &[
            "project",
            "github",
            "propose",
            "octocat/hello-world",
            "--set",
            "topic=rust",
        ],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env(
        "FORGE_GITHUB_TOKEN",
        "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
    );
    let out = cmd.output().expect("run forge");
    assert!(
        out.status.success(),
        "github propose: {}",
        lossy(&out.stderr)
    );
    assert_eq!(fs::read(&db).expect("registry bytes"), before_bytes);
    assert_eq!(table_names(&db), before_tables);
    assert_eq!(project_columns(&db), before_columns);
    assert_eq!(index_names(&db), before_indexes);
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM projects"),
        before_projects
    );
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM operations"),
        before_operations
    );
}

/// A token-shaped value that flows into the adapter through the
/// environment never reappears in stdout, stderr, the operations
/// journal or any human-facing report produced by the CLI. The
/// redactor is the single policy shared across every surface.
#[test]
fn a_github_token_never_reaches_stdout_stderr_or_a_human_report() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let token = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
    let observation = format!(
        r#"{{"contract":"{CONTRACT}","host":"github.com","repository":"octocat/hello-world","source_revision":"abc","observed_at":"2026-09-29T11:00:00Z","state":"current","description":"issued by {token}","topics":["rust","ci"],"languages":["Rust"],"workflows":["ci.yml"],"releases":["v1.0.0"],"custom_properties":[],"archived":false,"note":""}}"#
    );
    let proposal = format!(
        r#"{{"contract":"{CONTRACT}","mode":"pull-request","repository":"octocat/hello-world","pr_url":"https://github.com/octocat/hello-world/pull/1","note":"audit {token}"}}"#
    );
    github_stubs(&bins, &observation, &proposal);
    let (db, _alpha) = fixture(tmp.path());
    let cases: &[(&str, &[&str])] = &[
        (
            "json list",
            &["project", "list", "--source", "github", "--format", "json"],
        ),
        (
            "human list",
            &["project", "list", "--source", "github", "--format", "human"],
        ),
        (
            "ndjson list",
            &[
                "project", "list", "--source", "github", "--format", "ndjson",
            ],
        ),
        (
            "inspect",
            &[
                "project",
                "inspect",
                "octocat__hello-world",
                "--source",
                "github",
                "--format",
                "json",
            ],
        ),
        (
            "github observe json",
            &[
                "project",
                "github",
                "observe",
                "octocat/hello-world",
                "--format",
                "json",
            ],
        ),
        (
            "github observe human",
            &["project", "github", "observe", "octocat/hello-world"],
        ),
        (
            "github propose json",
            &[
                "project",
                "github",
                "propose",
                "octocat/hello-world",
                "--set",
                "topic=rust",
                "--format",
                "json",
            ],
        ),
        (
            "github propose human",
            &[
                "project",
                "github",
                "propose",
                "octocat/hello-world",
                "--set",
                "topic=rust",
            ],
        ),
    ];
    for (label, args) in cases {
        let (program_args, format): (Vec<String>, &str) =
            if label.contains("json") || label.contains("ndjson") {
                let mut a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
                if !a.iter().any(|arg| arg == "--format") {
                    a.push("--format".to_string());
                    a.push("json".to_string());
                }
                (a, "json")
            } else {
                (args.iter().map(|s| s.to_string()).collect(), "human")
            };
        let program_args_ref: Vec<&str> = program_args.iter().map(String::as_str).collect();
        let mut cmd = if format == "json" {
            cmd_json(&bins, &db, &program_args_ref)
        } else {
            cmd_human(&bins, &db, &program_args_ref)
        };
        cmd.env(
            "FORGE_GITHUB_BIN",
            bins.join("forge-github-metadata-adapter"),
        )
        .env("FORGE_GITHUB_TOKEN", token);
        let out = cmd.output().expect("run forge");
        assert!(
            out.status.success(),
            "{label}: stderr={}",
            lossy(&out.stderr)
        );
        let stdout = lossy(&out.stdout);
        let stderr = lossy(&out.stderr);
        assert!(
            !stdout.contains(token),
            "{label}: token leaked to stdout: {stdout}"
        );
        assert!(
            !stderr.contains(token),
            "{label}: token leaked to stderr: {stderr}"
        );
    }
    // The operations journal never records the token either.
    let conn = rusqlite::Connection::open(&db).expect("open registry");
    let mut statement = conn
        .prepare("SELECT kind, project_id, detail FROM operations ORDER BY op_id")
        .expect("prepare operations");
    let rows = statement
        .query_map([], |row| {
            let kind: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let detail: Option<String> = row.get(2)?;
            Ok((kind, project_id, detail))
        })
        .expect("query operations");
    for row in rows {
        let (kind, project_id, detail) = row.expect("row");
        let detail = detail.unwrap_or_default();
        assert!(
            !detail.contains(token),
            "journal[{kind}/{project_id}] contains the token: {detail}"
        );
    }
}

/// The closed record key set is never extended with a GitHub-only
/// field. A GitHub record keeps the same `tags` field as every other
/// record (always empty) and never receives a `topics` or `releases`
/// key on the record itself. Those namespaces live in the
/// `observations` payload of the JSON document and the human-facing
/// report only.
#[test]
fn github_records_never_gain_topics_or_releases_on_the_record_field() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let observation = format!(
        r#"{{"contract":"{CONTRACT}","host":"github.com","repository":"octocat/hello-world","source_revision":"abc","observed_at":"2026-09-29T11:00:00Z","state":"current","topics":["rust","ci"],"languages":["Rust"],"workflows":["ci.yml"],"releases":["v1.0.0"],"custom_properties":[],"archived":false,"note":""}}"#
    );
    let proposal = r#"{"contract":"forge-github-metadata/0.1.0","mode":"pull-request","repository":"octocat/hello-world","pr_url":"https://github.com/octocat/hello-world/pull/1"}"#;
    github_stubs(&bins, &observation, proposal);
    let (db, _alpha) = fixture(tmp.path());
    let mut cmd = cmd_json(
        &bins,
        &db,
        &["project", "list", "--source", "local", "--source", "github"],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let records = value["catalog"]["records"].as_array().unwrap();
    let github: Vec<&Value> = records
        .iter()
        .filter(|record| record["source_kind"] == "github")
        .collect();
    assert!(!github.is_empty(), "no github record in the catalog");
    for record in &github {
        // The closed record key set is enforced literally: every
        // record has exactly the keys in `RECORD_KEYS`, no extras.
        let obj = record.as_object().unwrap();
        for key in obj.keys() {
            assert!(
                matches!(
                    key.as_str(),
                    "project_id"
                        | "name"
                        | "source"
                        | "source_kind"
                        | "source_revision"
                        | "observed_at"
                        | "freshness"
                        | "profile"
                        | "lifecycle"
                        | "repository"
                        | "tags"
                        | "languages"
                        | "ci"
                        | "compose"
                        | "evidence"
                ),
                "github record has unexpected field `{key}`: {record}"
            );
        }
        // The portfolio `tags` namespace is the only tag-shaped
        // field on a record: it stays empty for GitHub so the
        // topics/releases namespaces can never be read through it.
        assert_eq!(
            record["tags"].as_array().unwrap().len(),
            0,
            "github record gained a portfolio tag: {record}"
        );
        // Topics and releases never appear as record fields; they
        // belong to the dedicated observation payload.
        assert!(
            record.get("topics").is_none(),
            "github record leaked `topics`: {record}"
        );
        assert!(
            record.get("releases").is_none(),
            "github record leaked `releases`: {record}"
        );
    }
    // The JSON `observations` payload does carry `topics` and
    // `releases` so the namespaces stay visible; the local records
    // never get those keys.
    let local: Vec<&Value> = records
        .iter()
        .filter(|record| record["source_kind"] == "local")
        .collect();
    for record in &local {
        assert!(
            record.get("topics").is_none(),
            "local record leaked `topics`: {record}"
        );
        assert!(
            record.get("releases").is_none(),
            "local record leaked `releases`: {record}"
        );
    }
    // The `forge project github observe` JSON page must keep the
    // namespaces separate too: the observation carries `topics` and
    // `releases`; the record is the closed projection.
    let mut cmd = cmd_json(
        &bins,
        &db,
        &["project", "github", "observe", "octocat/hello-world"],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let observation = &value["observations"][0];
    let topics = observation["topics"].as_array().unwrap();
    assert!(topics.iter().any(|topic| topic == "rust"));
    assert!(topics.iter().any(|topic| topic == "ci"));
    let releases = observation["releases"].as_array().unwrap();
    assert!(releases.iter().any(|release| release == "v1.0.0"));
    let record = &value["records"][0];
    assert!(record.get("topics").is_none());
    assert!(record.get("releases").is_none());
    assert_eq!(record["tags"].as_array().unwrap().len(), 0);
}
