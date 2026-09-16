//! Project import contract: evidence-backed detection and minimal adoption.
//!
//! Covers the `project-import` scenarios end to end through the built
//! binary: read-only inspection identifies inventory and a suggested
//! profile, ambiguous repositories fail before writes until a profile is
//! selected, missing remotes/configurations do not block inspection, and
//! acceptance writes only the manifest plus the registry record with
//! repeatable identity.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn rust_repo(dir: &Path) {
    write_file(
        dir,
        "Cargo.toml",
        "[package]\nname = \"demo\"\n[dependencies]\naxum = \"0.7\"\nsqlx = { version = \"0.8\", features = [\"postgres\"] }\n",
    );
    write_file(dir, "src/main.rs", "fn main() {}\n");
    write_file(dir, "Dockerfile", "FROM rust:1.78\n");
    write_file(dir, ".github/workflows/ci.yml", "on: [push]\n");
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_into(root, &mut out);
    out.sort();
    out
}

fn collect_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        out.push(path.clone());
        if path.is_dir() {
            collect_into(&path, out);
        }
    }
}

fn git_init(dir: &Path) {
    let status = Command::new("git")
        .arg("init")
        .arg(dir)
        .output()
        .expect("git init");
    assert!(status.status.success(), "git init must succeed");
}

#[test]
fn inspect_identifies_inventory_and_suggested_profile_read_only() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo-app");
    fs::create_dir(&proj).unwrap();
    rust_repo(&proj);
    let before = collect_files(&proj);

    let out = run(&db, &["import", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    for field in [
        "Language: rust",
        "Framework: axum",
        "Package manager: cargo",
        "Suggested profile:\nrust-web",
        "Suggested maturity:\nL1",
    ] {
        assert!(text.contains(field), "proposal must show {field}:\n{text}");
    }

    let out = run_json(&db, &["import", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("import must emit JSON");
    let proposal = &value["proposal"];
    assert_eq!(proposal["suggested_profile"], "rust-web");
    assert_eq!(proposal["suggested_maturity"], "L1");
    assert_eq!(proposal["language"]["value"], "rust");
    assert_eq!(proposal["package_manager"]["value"], "cargo");
    assert_eq!(proposal["docker"]["status"], "detected");
    assert_eq!(proposal["ci"]["status"], "detected");

    // Read-only: no manifest created, no registry side effects.
    assert_eq!(collect_files(&proj), before);
    assert!(!proj.join("forge.yaml").exists());
    assert!(!db.exists(), "inspection must not create a registry");
}

#[test]
fn ambiguous_repository_requires_explicit_selection_before_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("mixed-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "Cargo.toml", "[package]\nname = \"mixed\"\n");
    write_file(
        &proj,
        "pubspec.yaml",
        "name: mixed\nenvironment:\n  flutter: 3.22\n",
    );
    let before = collect_files(&proj);

    let out = run(&db, &["import", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[ambiguous-import]"), "{stderr}");
    assert!(stderr.contains("--profile"), "{stderr}");
    assert_eq!(collect_files(&proj), before);
    assert!(!db.exists());

    // Acceptance without selection fails the same way, changing nothing.
    let out = run(&db, &["import", &proj.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(lossy(&out.stderr).contains("error[ambiguous-import]"));
    assert_eq!(collect_files(&proj), before);

    // Explicit selection resolves: inspection first, then adoption.
    let out = run(
        &db,
        &[
            "import",
            &proj.display().to_string(),
            "--profile",
            "rust-web",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let out = run(
        &db,
        &[
            "import",
            &proj.display().to_string(),
            "--profile",
            "rust-web",
            "--accept",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert_eq!(run(&db, &["inspect", "mixed-app"]).status.code(), Some(0));
}

#[test]
fn missing_remote_and_driftwatch_do_not_block_inspection() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("plain-svc");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "Cargo.toml", "[package]\nname = \"plain\"\n");
    git_init(&proj);

    let out = run_json(&db, &["import", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let proposal = &value["proposal"];
    assert_eq!(proposal["suggested_profile"], "rust-web");
    // No origin remote: missing (checked), not unknown and not a failure.
    assert_eq!(proposal["git_remote"]["status"], "missing");
    assert_eq!(proposal["driftwatch"]["status"], "missing");
    assert!(!proj.join("forge.yaml").exists());
}

#[test]
fn accept_writes_only_manifest_and_registers() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("adopt-me");
    fs::create_dir(&proj).unwrap();
    rust_repo(&proj);
    let before = collect_files(&proj);

    let out = run(&db, &["import", &proj.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stdout).contains("imported adopt-me"),
        "{}",
        lossy(&out.stdout)
    );

    // Only the manifest was added to the source tree.
    let mut expected = before;
    expected.push(proj.join("forge.yaml"));
    expected.sort();
    assert_eq!(collect_files(&proj), expected);

    // The project is inspectable with the suggested profile and maturity.
    let out = run_json(&db, &["inspect", "adopt-me"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["profile"], "rust-web");
    assert_eq!(value["maturity"], "L1");
    assert_eq!(value["schema_version"], 1);

    let list = run(&db, &["list"]);
    assert_eq!(list.status.code(), Some(0));
    assert!(lossy(&list.stdout).contains("adopt-me"));
}

#[test]
fn conflicting_identity_fails_without_touching_source() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    // Two different directories whose leaf names derive the same id.
    let first = tmp.path().join("team-a").join("same-name");
    let second = tmp.path().join("team-b").join("same-name");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    rust_repo(&first);
    rust_repo(&second);

    let out = run(&db, &["import", &first.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let original = run_json(&db, &["inspect", "same-name"]);
    assert_eq!(original.status.code(), Some(0));
    let original_value: serde_json::Value = serde_json::from_slice(&original.stdout).unwrap();

    let second_before = collect_files(&second);
    let out = run(&db, &["import", &second.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    assert!(
        lossy(&out.stderr).contains("error[id-collision]"),
        "{}",
        lossy(&out.stderr)
    );
    // No half-adopted manifest left behind; original record unchanged.
    assert_eq!(collect_files(&second), second_before);
    assert!(!second.join("forge.yaml").exists());
    let current: serde_json::Value =
        serde_json::from_slice(&run_json(&db, &["inspect", "same-name"]).stdout).unwrap();
    assert_eq!(current["path"], original_value["path"]);
}

#[test]
fn unwritable_destination_fails_without_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("blocked-app");
    fs::create_dir(&proj).unwrap();
    rust_repo(&proj);
    // A directory where forge.yaml would go makes the write fail portably.
    fs::create_dir(proj.join("forge.yaml")).unwrap();
    let before = collect_files(&proj);

    let out = run(&db, &["import", &proj.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[import-conflict]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(collect_files(&proj), before);
    assert_eq!(run(&db, &["inspect", "blocked-app"]).status.code(), Some(1));
}

#[test]
fn legacy_manifest_is_never_doubled() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("legacy-svc");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "Cargo.toml", "[package]\nname = \"legacy\"\n");
    write_file(
        &proj,
        "platform.yaml",
        "schema: 1\nproject:\n  id: legacy-svc\n  name: Legacy Svc\n  profile: rust-web\n",
    );
    let before = collect_files(&proj);

    let out = run(&db, &["import", &proj.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[import-conflict]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(collect_files(&proj), before);
    assert!(!proj.join("forge.yaml").exists());
}

#[test]
fn repeat_import_keeps_identity_without_duplicates() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("steady-app");
    fs::create_dir(&proj).unwrap();
    rust_repo(&proj);

    for _ in 0..2 {
        let out = run(&db, &["import", &proj.display().to_string(), "--accept"]);
        assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    }

    let list = run_json(&db, &["list"]);
    assert_eq!(list.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    let projects = value["projects"].as_array().expect("projects array");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["id"], "steady-app");

    // Existing manifests are adopted, never overwritten.
    let manifest_before = fs::read(proj.join("forge.yaml")).unwrap();
    let out = run(&db, &["import", &proj.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(fs::read(proj.join("forge.yaml")).unwrap(), manifest_before);
}

#[test]
fn incompatible_existing_manifest_fails_before_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("mobile-app");
    fs::create_dir(&proj).unwrap();
    let manifest = "schema: 1\nproject:\n  id: mobile-app\n  name: Mobile App\n  profile: flutter-app\nfeatures:\n  postgres: 2.0\n";
    write_file(&proj, "forge.yaml", manifest);
    write_file(
        &proj,
        "pubspec.yaml",
        "name: mobile\nenvironment:\n  flutter: 3.22\n",
    );

    // Profile compatibility from profile-registry still gates adoption.
    let out = run(&db, &["import", &proj.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[incompatible-profile]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(
        fs::read_to_string(proj.join("forge.yaml")).unwrap(),
        manifest
    );
    assert_eq!(run(&db, &["inspect", "mobile-app"]).status.code(), Some(1));
}

#[test]
fn unrecognizable_directory_reports_unknown_and_refuses_accept() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("mystery");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "notes.txt", "just some prose\n");
    let before = collect_files(&proj);

    let out = run_json(&db, &["import", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        value["proposal"]["suggested_profile"],
        serde_json::Value::Null
    );
    assert_eq!(value["proposal"]["language"]["status"], "unknown");

    let out = run(&db, &["import", &proj.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[import-conflict]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(collect_files(&proj), before);
}
