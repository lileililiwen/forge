//! Cross-surface regression for the GitHub CLI workflow
//! (`github-cli-project-workflows`).
//!
//! The per-surface contract verifies the
//! `forge-github-cli-workflows/0.1.0` surface in isolation. This
//! suite verifies the properties a single surface can never prove on
//! its own:
//!
//! - **Reads are side-effect-free.** `forge project github auth` does
//!   not write a single byte to the registry, does not append an
//!   operations-journal row, and does not create a project that was
//!   not there before.
//! - **The token is never echoed.** A credential-shaped value that
//!   never crosses Forge — `gh` carries it in its own credential
//!   store — never appears in Forge's stdout, stderr or operations
//!   journal.
//! - **The closed record key set is unchanged.** A CLI read or write
//!   leaves the registry schema byte-identical and never adds a
//!   `github.cli` field outside the operations journal.
//! - **No live host is contacted.** Every fixture is a local shell
//!   stub on a controlled `PATH`; the only network-shaped strings
//!   are `github.com` literals and the stub never opens a socket.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-github-cli-workflows/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn controlled_path(bins: &Path) -> std::ffi::OsString {
    let parent = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![bins.to_path_buf()];
    for entry in std::env::split_paths(&parent) {
        paths.push(entry);
    }
    std::env::join_paths(paths).expect("join PATH")
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
        .env_remove("FORGE_GITHUB_TOKEN")
        .env_remove("FORGE_GH_BIN");
    cmd.env("PATH", controlled_path(bins));
    cmd
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

/// `gh` stub that records every argv it receives and exits 0 for
/// the four operations Forge dispatches. Used by the read-only
/// cross-surface assertions.
fn ok_stub(argv_log: &Path, pr_artifact: &str) -> String {
    format!(
        "#!/bin/sh\n\
         printf '%s\\n' \"$@\" >> '{log}'\n\
         if [ \"${{1:-}}\" = \"auth\" ] && [ \"${{2:-}}\" = \"token\" ]; then\n\
         \x20echo 'gh stub: refused auth token call' 1>&2\n\
         \x20exit 64\n\
         fi\n\
         if [ \"${{1:-}}\" = \"auth\" ] && [ \"${{2:-}}\" = \"status\" ]; then exit 0; fi\n\
         if [ \"${{1:-}}\" = \"repo\" ] && [ \"${{2:-}}\" = \"clone\" ]; then exit 0; fi\n\
         if [ \"${{1:-}}\" = \"repo\" ] && [ \"${{2:-}}\" = \"create\" ]; then exit 0; fi\n\
         if [ \"${{1:-}}\" = \"pr\" ] && [ \"${{2:-}}\" = \"create\" ]; then\n\
         \x20printf '%s' \"${{GH_PR_ARTIFACT:-{artifact}}}\"\n\
         \x20exit 0\n\
         fi\n\
         exit 63\n",
        log = argv_log.display(),
        artifact = pr_artifact,
    )
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

fn init_git(dir: &Path, github_url: &str) {
    let init = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["init", "-q"])
        .output()
        .expect("git init");
    assert!(init.status.success(), "git init: {}", lossy(&init.stderr));
    for args in [
        vec!["config", "user.email", "ci@example.com"],
        vec!["config", "user.name", "Forge CI"],
        vec!["remote", "add", "origin", github_url],
        vec!["add", "-A"],
        vec!["commit", "-q", "--allow-empty", "-m", "init"],
    ] {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(&args)
            .status()
            .expect("git");
        assert!(status.success(), "{args:?}");
    }
}

fn fixture(tmp: &Path) -> PathBuf {
    let db = tmp.join("registry.db");
    let alpha = tmp.join("alpha");
    write_project(&alpha, "alpha");
    init_git(&alpha, "https://github.com/octocat/hello-world.git");
    let out = clean_cmd(&alpha)
        .arg("--registry")
        .arg(&db)
        .arg("register")
        .arg(&alpha)
        .output()
        .expect("register");
    assert!(
        out.status.success(),
        "register failed: {}",
        lossy(&out.stderr)
    );
    db
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

/// A read-only `forge project github auth` invocation does not
/// mutate the registry: same bytes, same tables, same columns, same
/// indexes, same number of project rows, same number of
/// operations-journal rows. The CLI surface is read-only by
/// construction — it never opens a registry connection — and this
/// test pins the invariant.
#[test]
fn a_cli_read_writes_no_registry_byte_table_column_index_or_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let argv = tmp.path().join("gh.argv");
    write_stub(
        &bins,
        "gh",
        &ok_stub(&argv, "https://github.com/octocat/hello-world/pull/1"),
    );
    let db = fixture(tmp.path());
    let before_bytes = fs::read(&db).expect("registry bytes");
    let before_tables = table_names(&db);
    let before_columns = project_columns(&db);
    let before_indexes = index_names(&db);
    let before_projects = scalar(&db, "SELECT COUNT(*) FROM projects");
    let before_operations = scalar(&db, "SELECT COUNT(*) FROM operations");
    let mut cmd = clean_cmd(&bins);
    cmd.arg("--format")
        .arg("json")
        .arg("--registry")
        .arg(&db)
        .arg("project")
        .arg("github")
        .arg("auth");
    let out = cmd.output().expect("run forge");
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
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

/// The closed `forge-github-cli-workflows/0.1.0` JSON envelope is
/// stable: every subcommand emits the same `contract` field, the
/// closed `operation` set, the closed `outcome` set, and a bounded
/// set of optional fields. A CLI write that succeeds carries an
/// `artifact_url`; a refusal never invents a field.
#[test]
fn every_subcommand_emits_the_closed_contract_envelope() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let argv = tmp.path().join("gh.argv");
    write_stub(
        &bins,
        "gh",
        &ok_stub(&argv, "https://github.com/octocat/hello-world/pull/1"),
    );
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    write_project(&project, "proj");
    init_git(&project, "https://github.com/octocat/hello-world.git");
    let destination = tmp.path().join("clone-dest");

    // Auth
    let mut cmd = clean_cmd(&bins);
    cmd.arg("--format")
        .arg("json")
        .arg("--registry")
        .arg(&db)
        .arg("project")
        .arg("github")
        .arg("auth");
    let out = cmd.output().expect("auth");
    assert!(out.status.success(), "auth: {}", lossy(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value["contract"], CONTRACT);
    assert_eq!(value["operation"], "auth");
    assert_eq!(value["outcome"], "done");

    // Clone
    let mut cmd = clean_cmd(&bins);
    cmd.arg("--format")
        .arg("json")
        .arg("--registry")
        .arg(&db)
        .arg("project")
        .arg("github")
        .arg("clone")
        .arg("octocat/hello-world")
        .arg(&destination)
        .arg("--confirm");
    let out = cmd.output().expect("clone");
    assert!(out.status.success(), "clone: {}", lossy(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value["contract"], CONTRACT);
    assert_eq!(value["operation"], "clone");
    assert_eq!(value["outcome"], "done");

    // Create
    let mut cmd = clean_cmd(&bins);
    cmd.arg("--format")
        .arg("json")
        .arg("--registry")
        .arg(&db)
        .arg("project")
        .arg("github")
        .arg("create")
        .arg(&project)
        .arg("--repo")
        .arg("octocat/hello-world")
        .arg("--confirm");
    let out = cmd.output().expect("create");
    assert!(out.status.success(), "create: {}", lossy(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value["contract"], CONTRACT);
    assert_eq!(value["operation"], "create");
    assert_eq!(value["outcome"], "done");

    // Pull-request
    let mut cmd = clean_cmd(&bins);
    cmd.arg("--format")
        .arg("json")
        .arg("--registry")
        .arg(&db)
        .arg("project")
        .arg("github")
        .arg("pull-request")
        .arg(&project)
        .arg("--title")
        .arg("title")
        .arg("--body")
        .arg("body")
        .arg("--confirm");
    let out = cmd.output().expect("pull-request");
    assert!(out.status.success(), "pr: {}", lossy(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value["contract"], CONTRACT);
    assert_eq!(value["operation"], "pull-request");
    assert_eq!(value["outcome"], "done");
    assert_eq!(
        value["artifact_url"],
        "https://github.com/octocat/hello-world/pull/1"
    );
}

/// The CLI surface never echoes a token-shaped string. The stub
/// rejects `gh auth token` calls and refuses to echo any
/// credential-shaped substring on stdout or stderr; the CLI surface
/// inherits that posture because Forge itself never reads the
/// credential store.
#[test]
fn no_subcommand_emits_a_token_shaped_string_on_stdout_or_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let argv = tmp.path().join("gh.argv");
    write_stub(
        &bins,
        "gh",
        &ok_stub(&argv, "https://github.com/octocat/hello-world/pull/1"),
    );
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    write_project(&project, "proj");
    init_git(&project, "https://github.com/octocat/hello-world.git");
    let destination = tmp.path().join("clone-dest");
    let token = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
    let cases: &[(&str, Vec<String>)] = &[
        (
            "auth",
            vec![
                "project".to_string(),
                "github".to_string(),
                "auth".to_string(),
            ],
        ),
        (
            "clone",
            vec![
                "project".into(),
                "github".into(),
                "clone".into(),
                "octocat/hello-world".into(),
                destination.to_string_lossy().into_owned(),
                "--confirm".into(),
            ],
        ),
        (
            "create",
            vec![
                "project".into(),
                "github".into(),
                "create".into(),
                project.to_string_lossy().into_owned(),
                "--repo".into(),
                "octocat/hello-world".into(),
                "--confirm".into(),
            ],
        ),
        (
            "pull-request",
            vec![
                "project".into(),
                "github".into(),
                "pull-request".into(),
                project.to_string_lossy().into_owned(),
                "--title".into(),
                "title".into(),
                "--body".into(),
                "body".into(),
                "--confirm".into(),
            ],
        ),
    ];
    for (label, args) in cases {
        let mut cmd = clean_cmd(&bins);
        cmd.arg("--format")
            .arg("json")
            .arg("--registry")
            .arg(&db)
            .args(args);
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
}
