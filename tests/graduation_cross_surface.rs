//! Cross-surface regression for the graduation import surface
//! (`hypora-graduation-import`).
//!
//! This suite answers the questions a per-command contract cannot:
//!
//! - **A preview spawns no process; a confirmed import spawns no
//!   network tool and no `gh`.** The only child process a confirmed
//!   import adds is the registry's existing best-effort `git` probe,
//!   unchanged from `forge import`.
//! - **A graduation import scaffolds, deploys, publishes and approves
//!   nothing.** The only registry operation is `register`; the only
//!   files are the minimal manifest and the receipt.
//! - **The registry gains exactly one project and one journal row.**
//! - **No evidence excerpt and no original artifact byte reaches
//!   disk.** The receipt carries the mapped brief, the source block and
//!   the evidence count only; the artifact file is left untouched and
//!   never copied.
//! - **Existing surfaces are unchanged.** `forge import`, the manifest
//!   schema and an unrelated registry behave exactly as before.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

const IDEA_CONTRACT: &str = "platform.idea-graduation/0.1.0";

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
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_GITHUB_BIN")
        .env_remove("FORGE_GITHUB_TOKEN");
    let parent = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![bins.to_path_buf()];
    for entry in std::env::split_paths(&parent) {
        paths.push(entry);
    }
    cmd.env("PATH", std::env::join_paths(paths).expect("join PATH"));
    cmd
}

fn run(bins: &Path, db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd(bins);
    cmd.arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run forge")
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_stub(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Plant observing stubs for every network-shaped tool. Each stub
/// records its invocation and fails, so a live call would be visible.
fn install_stubs(bins: &Path) {
    fs::create_dir_all(bins).unwrap();
    for name in ["gh", "curl", "wget", "ssh", "nc"] {
        let marker = bins.join(format!("{name}.invoked"));
        write_stub(
            bins,
            name,
            &format!(
                "#!/bin/sh\necho \"$@\" >> \"{}\"\nexit 1\n",
                marker.display()
            ),
        );
    }
    let git_marker = bins.join("git.invoked");
    write_stub(
        bins,
        "git",
        &format!(
            "#!/bin/sh\necho \"$@\" >> \"{}\"\nexit 1\n",
            git_marker.display()
        ),
    );
}

fn valid_artifact() -> Value {
    json!({
        "contract": IDEA_CONTRACT,
        "hypora_project_id": "prj_cross",
        "hypora_revision": "rev-cross-1",
        "graduated_at": "2026-09-20T00:00:00Z",
        "brief": {
            "title": "Cross Surface App",
            "problem": "A problem.",
            "audience": "Students.",
            "solution": "A solution.",
            "requirements": ["R1."],
            "success_metrics": [ { "name": "m", "target": ">= 1", "window": "30d" } ]
        },
        "experiment": {
            "summary": "A summary.",
            "validated": true,
            "evidence": [
                {
                    "kind": "probe",
                    "excerpt": "EXCERPT-ONLY-MARKER-XYZZY",
                    "observed_at": "2026-09-18T00:00:00Z"
                }
            ]
        }
    })
}

fn write_artifact(tmp: &Path) -> PathBuf {
    let path = tmp.join("artifact.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&valid_artifact()).unwrap(),
    )
    .unwrap();
    path
}

fn collect_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    collect_into(root, root, &mut out);
    out.sort();
    out
}

fn collect_into(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_into(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.display().to_string());
        }
    }
}

fn read_all_under(root: &Path) -> String {
    let mut text = String::new();
    for rel in collect_files(root) {
        if let Ok(bytes) = fs::read(root.join(&rel)) {
            text.push_str(&String::from_utf8_lossy(&bytes));
            text.push('\n');
        }
    }
    text
}

fn scalar(db: &Path, sql: &str) -> i64 {
    let conn = rusqlite::Connection::open(db).expect("open registry");
    conn.query_row(sql, [], |row| row.get(0)).expect("scalar")
}

fn operation_kinds(db: &Path) -> Vec<String> {
    let conn = rusqlite::Connection::open(db).expect("open registry");
    let mut stmt = conn
        .prepare("SELECT kind FROM operations ORDER BY op_id")
        .expect("prepare");
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query");
    rows.map(|row| row.expect("row")).collect()
}

fn register_unrelated(bins: &Path, db: &Path, dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
    let out = run(bins, db, &["register", &dir.display().to_string()]);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
}

#[test]
fn a_graduation_import_makes_no_network_or_gh_invocation() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    install_stubs(&bins);
    let db = tmp.path().join("registry.db");
    let artifact = write_artifact(tmp.path());
    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();
    let artifact_s = artifact.display().to_string();

    // A preview touches no registry and spawns no process at all.
    let out = run(&bins, &db, &["graduation", "preview", &artifact_s]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    for name in ["gh", "curl", "wget", "ssh", "nc", "git"] {
        assert!(
            !bins.join(format!("{name}.invoked")).exists(),
            "a preview must not spawn {name}"
        );
    }
    assert!(!db.exists());

    // A confirmed import registers, so the registry's existing `git`
    // probe runs; no network tool or `gh` does.
    let out = run(
        &bins,
        &db,
        &[
            "graduation",
            "import",
            &artifact_s,
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    for name in ["gh", "curl", "wget", "ssh", "nc"] {
        assert!(
            !bins.join(format!("{name}.invoked")).exists(),
            "a graduation import must not spawn {name}"
        );
    }
    let git_log = fs::read_to_string(bins.join("git.invoked")).expect("git probe ran");
    for line in git_log.lines() {
        assert!(
            line.contains("remote get-url origin") || line.contains("rev-parse HEAD"),
            "only the registry's git probe may run, saw: {line}"
        );
    }
}

#[test]
fn a_graduation_import_scaffolds_deploys_and_approves_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    install_stubs(&bins);
    let db = tmp.path().join("registry.db");
    let artifact = write_artifact(tmp.path());
    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();

    let out = run(
        &bins,
        &db,
        &[
            "graduation",
            "import",
            &artifact.display().to_string(),
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // The only files are the minimal manifest and the receipt.
    let files = collect_files(&dest);
    assert_eq!(
        files,
        vec![
            ".forge/graduation/cross-surface-app/import.json".to_string(),
            "forge.yaml".to_string(),
        ],
        "{files:?}"
    );
    // The only journal operation is the registration itself.
    assert_eq!(operation_kinds(&db), vec!["register".to_string()]);
}

#[test]
fn the_registry_gains_exactly_one_project_and_one_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    install_stubs(&bins);
    let db = tmp.path().join("registry.db");
    let artifact = write_artifact(tmp.path());
    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();

    let out = run(
        &bins,
        &db,
        &[
            "graduation",
            "import",
            &artifact.display().to_string(),
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM projects"), 1);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM operations"), 1);
}

#[test]
fn no_evidence_excerpt_or_artifact_byte_reaches_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    install_stubs(&bins);
    let db = tmp.path().join("registry.db");
    let artifact = write_artifact(tmp.path());
    let artifact_bytes = fs::read(&artifact).unwrap();
    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();

    let out = run(
        &bins,
        &db,
        &[
            "graduation",
            "import",
            &artifact.display().to_string(),
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // The excerpt is shown in the preview but never persisted.
    let under_dest = read_all_under(&dest);
    assert!(
        !under_dest.contains("EXCERPT-ONLY-MARKER-XYZZY"),
        "an evidence excerpt must never reach disk"
    );
    // The mapped brief and provenance are persisted.
    assert!(under_dest.contains("Cross Surface App"));
    assert!(under_dest.contains("rev-cross-1"));

    // No file under the project is a copy of the artifact.
    for rel in collect_files(&dest) {
        let bytes = fs::read(dest.join(&rel)).unwrap();
        assert_ne!(bytes, artifact_bytes, "artifact bytes copied to {rel}");
    }
    // The original file is untouched.
    assert_eq!(fs::read(&artifact).unwrap(), artifact_bytes);
}

#[test]
fn the_original_artifact_is_never_copied_into_the_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    install_stubs(&bins);
    let db = tmp.path().join("registry.db");
    let artifact = write_artifact(tmp.path());
    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();

    let out = run(
        &bins,
        &db,
        &[
            "graduation",
            "import",
            &artifact.display().to_string(),
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let artifact_name = artifact.file_name().unwrap().to_string_lossy().to_string();
    assert!(
        !collect_files(&dest)
            .iter()
            .any(|rel| rel.ends_with(&artifact_name)),
        "the artifact file must not be copied into the project"
    );
}

#[test]
fn an_unrelated_registry_is_read_as_is() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    install_stubs(&bins);
    let db = tmp.path().join("registry.db");
    let unrelated = tmp.path().join("unrelated");
    register_unrelated(&bins, &db, &unrelated, "unrelated-app");
    let manifest_before = fs::read(unrelated.join("forge.yaml")).unwrap();
    let projects_before = scalar(&db, "SELECT COUNT(*) FROM projects");

    let artifact = write_artifact(tmp.path());
    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();
    let out = run(
        &bins,
        &db,
        &[
            "graduation",
            "import",
            &artifact.display().to_string(),
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    assert_eq!(
        fs::read(unrelated.join("forge.yaml")).unwrap(),
        manifest_before
    );
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM projects"),
        projects_before + 1
    );
    let list = run(&bins, &db, &["list"]);
    let list = lossy(&list.stdout);
    assert!(list.contains("unrelated-app"), "{list}");
    assert!(list.contains("cross-surface-app"), "{list}");
}

#[test]
fn existing_import_surfaces_are_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    install_stubs(&bins);
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("legacy-app");
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::write(
        proj.join("Cargo.toml"),
        "[package]\nname = \"legacy\"\n[dependencies]\naxum = \"0.7\"\n",
    )
    .unwrap();
    fs::write(proj.join("src/main.rs"), "fn main() {}\n").unwrap();
    let proj_s = proj.display().to_string();

    let out = run(&bins, &db, &["import", &proj_s]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stdout).contains("Suggested profile:\nrust-web"),
        "{}",
        lossy(&out.stdout)
    );

    let out = run(&bins, &db, &["import", &proj_s, "--accept"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let manifest = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    assert!(manifest.contains("schema: 1"), "{manifest}");

    // The graduation surface does not attach itself to `forge import`.
    let out = run(&bins, &db, &["import", "--help"]);
    let help = lossy(&out.stdout);
    assert!(help.contains("--profile"), "{help}");
    assert!(
        !help.contains("graduation"),
        "import must not absorb graduation:\n{help}"
    );
}
