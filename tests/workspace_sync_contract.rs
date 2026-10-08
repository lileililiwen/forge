//! Contract tests for `forge workspace sync` (`forge-workspace-sync`).
//!
//! These drive the real binary against a throwaway fixture workspace root
//! and a throwaway registry: a mixed root converges in one run with one
//! honest outcome per directory, failures never abort siblings, reruns are
//! idempotent via the identity check (no rewrite, no new manifests), and
//! the JSON envelope carries the pinned contract shape.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::registry::Registry;
use tempfile::TempDir;

const CONTRACT: &str = "forge-workspace-sync/0.1.0";

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
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run forge")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn manifest_text(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    )
}

fn rust_app(dir: &Path) {
    write_file(
        &dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\n[dependencies]\naxum = \"0.7\"\n",
    );
    write_file(&dir.join("src/main.rs"), "fn main() {}\n");
}

fn ambiguous_app(dir: &Path) {
    write_file(&dir.join("Cargo.toml"), "[package]\nname = \"demo\"\n");
    write_file(
        &dir.join("pubspec.yaml"),
        "name: demo\nenvironment:\n  flutter: 3.22\n",
    );
}

/// The mixed fixture root: manifest, importable, ambiguous, undecidable,
/// invalid-manifest, hidden, plain file and symlink-escape entries.
fn mixed_root(base: &Path) -> PathBuf {
    let root = base.join("workspace");
    write_file(
        &root.join("manifest-app/forge.yaml"),
        &manifest_text("manifest-app"),
    );
    rust_app(&root.join("rust-app"));
    ambiguous_app(&root.join("ambiguous-app"));
    fs::create_dir_all(root.join("empty-app")).unwrap();
    rust_app(&root.join(".hidden-app"));
    write_file(&root.join("bad-app/forge.yaml"), "not: [valid, yaml");
    write_file(&root.join("loose-file.txt"), "hello\n");
    #[cfg(unix)]
    std::os::unix::fs::symlink("/tmp", root.join("link-app")).unwrap();
    root
}

fn journal_counts(db: &Path, kind: &str) -> usize {
    Registry::open(db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .iter()
        .filter(|entry| entry.kind == kind)
        .count()
}

#[test]
fn sync_onboards_a_mixed_workspace() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    let root = mixed_root(temp.path());

    // One directory is already registered before the sync runs.
    let pre = temp.path().join("pre-registered");
    write_file(&pre.join("pre-app/forge.yaml"), &manifest_text("pre-app"));
    let register = run(&db, &["register", pre.join("pre-app").to_str().unwrap()]);
    assert!(register.status.success(), "setup register failed");

    let output = run(&db, &["workspace", "sync", root.to_str().unwrap()]);
    // One invalid manifest fails the run, but every sibling is processed.
    assert!(
        !output.status.success(),
        "a failed directory must fail the run"
    );
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines.contains(&"skipped .hidden-app hidden"),
        "hidden entry must skip with a reason: {text}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("skipped ambiguous-app ambiguous")),
        "ambiguous detection must skip, never guess: {text}"
    );
    assert!(
        lines.contains(&"failed bad-app manifest-invalid"),
        "invalid manifest must fail typed: {text}"
    );
    assert!(
        lines.contains(&"skipped empty-app undecidable"),
        "undecidable detection must skip: {text}"
    );
    assert!(
        lines.contains(&"skipped link-app symlink"),
        "symlinks must skip: {text}"
    );
    assert!(
        lines.contains(&"skipped loose-file.txt not-a-directory"),
        "files must skip: {text}"
    );
    assert!(
        lines.contains(&"ok manifest-app manifest-app"),
        "manifest directory must register: {text}"
    );
    assert!(
        lines.contains(&"ok rust-app rust-app"),
        "decidable directory must adopt: {text}"
    );
    assert_eq!(
        lines.last(),
        Some(&"synced 2, already 0, skipped 5, failed 1"),
        "summary counts must pin every outcome: {text}"
    );

    // The registry holds the onboardable set: the manifest, the adoption
    // (with its written manifest) and the pre-registered project.
    let registry = Registry::open(&db).unwrap();
    assert_eq!(
        registry.inspect("manifest-app").unwrap().profile,
        "rust-web"
    );
    assert_eq!(registry.inspect("rust-app").unwrap().profile, "rust-web");
    assert!(root.join("rust-app/forge.yaml").is_file());
    assert_eq!(registry.inspect("pre-app").unwrap().profile, "rust-web");
}

#[test]
fn rerun_is_idempotent_and_converges_new_siblings() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("workspace");
    write_file(
        &root.join("manifest-app/forge.yaml"),
        &manifest_text("manifest-app"),
    );
    rust_app(&root.join("rust-app"));
    ambiguous_app(&root.join("ambiguous-app"));

    let first = run(&db, &["workspace", "sync", root.to_str().unwrap()]);
    assert!(first.status.success(), "clean root must exit 0");
    assert!(stdout(&first).contains("synced 2, already 0, skipped 1, failed 0"));
    let adopted = fs::read(root.join("rust-app/forge.yaml")).unwrap();
    assert_eq!(journal_counts(&db, "register"), 2);
    assert_eq!(journal_counts(&db, "workspace.sync"), 1);

    // The second run over the unchanged root rewrites nothing: previously
    // onboarded directories report `already`, no new `register` journal
    // row appears, and the adopted manifest bytes are untouched.
    let second = run(&db, &["workspace", "sync", root.to_str().unwrap()]);
    assert!(second.status.success());
    let text = stdout(&second);
    assert!(text.contains("already manifest-app manifest-app"), "{text}");
    assert!(text.contains("already rust-app rust-app"), "{text}");
    assert!(
        text.contains("synced 0, already 2, skipped 1, failed 0"),
        "{text}"
    );
    assert_eq!(fs::read(root.join("rust-app/forge.yaml")).unwrap(), adopted);
    assert_eq!(journal_counts(&db, "register"), 2);
    assert_eq!(journal_counts(&db, "workspace.sync"), 2);

    // Adding a sibling onboards only the newcomer.
    rust_app(&root.join("new-app"));
    let third = run(&db, &["workspace", "sync", root.to_str().unwrap()]);
    assert!(third.status.success());
    let text = stdout(&third);
    assert!(text.contains("ok new-app new-app"), "{text}");
    assert!(
        text.contains("synced 1, already 2, skipped 1, failed 0"),
        "{text}"
    );
}

#[test]
fn failures_do_not_stop_siblings() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");

    // Occupy the `taken-id` identity from a different path first.
    let elsewhere = temp.path().join("elsewhere");
    write_file(
        &elsewhere.join("taken-id/forge.yaml"),
        &manifest_text("taken-id"),
    );
    assert!(run(
        &db,
        &["register", elsewhere.join("taken-id").to_str().unwrap()]
    )
    .status
    .success());

    let root = temp.path().join("workspace");
    rust_app(&root.join("taken-id"));
    write_file(&root.join("bad-app/forge.yaml"), "not: [valid, yaml");
    rust_app(&root.join("good-app"));

    let output = run(&db, &["workspace", "sync", root.to_str().unwrap()]);
    assert!(!output.status.success());
    let text = stdout(&output);
    assert!(
        text.contains("failed taken-id id-collision"),
        "identity collision must fail typed without touching: {text}"
    );
    assert!(
        text.contains("failed bad-app manifest-invalid"),
        "invalid manifest must fail typed: {text}"
    );
    assert!(
        text.contains("ok good-app good-app"),
        "the healthy sibling must still onboard: {text}"
    );
    assert!(
        text.contains("synced 1, already 0, skipped 0, failed 2"),
        "{text}"
    );

    // The collision touched nothing: the original row still points at the
    // first path and no manifest was written into the colliding folder.
    let registry = Registry::open(&db).unwrap();
    assert_eq!(
        registry.inspect("taken-id").unwrap().path,
        elsewhere
            .join("taken-id")
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
    assert!(!root.join("taken-id/forge.yaml").exists());
}

#[test]
fn json_shape_is_stable() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    let root = mixed_root(temp.path());

    let output = run(
        &db,
        &[
            "workspace",
            "sync",
            "--format",
            "json",
            root.to_str().unwrap(),
        ],
    );
    assert!(!output.status.success(), "failed entries must fail the run");
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["contract"], CONTRACT);
    assert_eq!(
        body["root"],
        root.canonicalize().unwrap().display().to_string()
    );
    let entries = body["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 8, "every child gets exactly one outcome");
    let by_dir = |name: &str| {
        entries
            .iter()
            .find(|entry| entry["directory"] == name)
            .unwrap_or_else(|| panic!("missing entry for {name}"))
    };
    assert_eq!(by_dir("manifest-app")["outcome"], "ok");
    assert_eq!(by_dir("manifest-app")["id"], "manifest-app");
    assert_eq!(by_dir("rust-app")["outcome"], "ok");
    assert_eq!(by_dir("ambiguous-app")["outcome"], "skipped");
    assert_eq!(by_dir("ambiguous-app")["reason"], "ambiguous");
    assert_eq!(by_dir("empty-app")["reason"], "undecidable");
    assert_eq!(by_dir(".hidden-app")["reason"], "hidden");
    assert_eq!(by_dir("loose-file.txt")["reason"], "not-a-directory");
    assert_eq!(by_dir("link-app")["reason"], "symlink");
    assert_eq!(by_dir("bad-app")["outcome"], "failed");
    assert_eq!(by_dir("bad-app")["reason"], "manifest-invalid");
    assert_eq!(
        body["summary"],
        serde_json::json!({
            "ok": 2, "already": 0, "skipped": 5, "failed": 1,
        })
    );
}

#[test]
fn skips_only_root_exits_zero() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("workspace");
    ambiguous_app(&root.join("ambiguous-app"));
    fs::create_dir_all(root.join("empty-app")).unwrap();
    rust_app(&root.join(".hidden-app"));
    write_file(&root.join("loose-file.txt"), "hello\n");

    let output = run(&db, &["workspace", "sync", root.to_str().unwrap()]);
    assert!(output.status.success(), "skips must not fail the run");
    assert!(
        stdout(&output).contains("synced 0, already 0, skipped 4, failed 0"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn empty_root_reports_zero_counts() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("workspace");
    fs::create_dir_all(&root).unwrap();

    let output = run(&db, &["workspace", "sync", root.to_str().unwrap()]);
    assert!(output.status.success());
    assert!(
        stdout(&output).contains("synced 0, already 0, skipped 0, failed 0"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn missing_root_is_a_typed_error_and_touches_nothing() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    let missing = temp.path().join("no-such-workspace");

    let output = run(&db, &["workspace", "sync", missing.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8");
    assert!(
        stderr.contains("path-unavailable"),
        "typed error expected: {stderr}"
    );
    assert!(!db.exists(), "a bad ROOT must not even create the registry");
}
