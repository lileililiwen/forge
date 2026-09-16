//! Feature lifecycle contract: versioned resolution/discovery and safe
//! add/remove/upgrade through the built binary.
//!
//! Covers the `feature-lifecycle` scenarios end to end: discovery success
//! (exact versions in dependency order), resolution failure (conflict,
//! missing version, unsupported mapping) before edits, unsupported-profile
//! boundary reporting, add-then-upgrade agreement across source/manifest/
//! registry with validators, removal blocked by reverse dependencies and by
//! user-owned receipt edits with files preserved, exact-reinstall no-op
//! without duplicate registration, and repeatability.

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

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("valid json")
}

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn rust_l1(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
}

fn manifest_text(dir: &Path) -> String {
    fs::read_to_string(dir.join("forge.yaml")).unwrap()
}

#[test]
fn list_exposes_versioned_catalog() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let out = run(&db, &["feature", "list"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    for id in ["auth", "admin", "postgres", "telemetry"] {
        assert!(lossy(&out.stdout).contains(id), "{}", lossy(&out.stdout));
    }

    let value = run_json(&db, &["feature", "list"]);
    let features = value["features"].as_array().unwrap();
    assert_eq!(features.len(), 18);
    for f in features {
        assert_eq!(f["version"], "0.1.0");
        assert!(!f["compatible_profiles"].as_array().unwrap().is_empty());
        assert!(!f["install_strategy"].as_str().unwrap().is_empty());
        assert!(!f["documentation"].as_str().unwrap().is_empty());
    }
}

#[test]
fn inspect_returns_full_descriptor() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let value = run_json(&db, &["feature", "inspect", "admin"]);
    assert_eq!(value["id"], "admin");
    assert_eq!(value["version"], "0.1.0");
    assert!(value["depends"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d == "auth"));

    let out = run(&db, &["feature", "inspect", "nosuch"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[unknown-feature]"),
        "{}",
        lossy(&out.stderr)
    );
}

#[test]
fn resolve_lists_exact_versions_in_dependency_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let value = run_json(
        &db,
        &["feature", "resolve", "rust-web", "--feature", "admin"],
    );
    let steps = value["plan"]["steps"].as_array().unwrap();
    let ids: Vec<&str> = steps
        .iter()
        .map(|s| s["feature"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["auth", "admin"]);
    for s in steps {
        assert_eq!(s["version"], "0.1.0");
        assert_eq!(s["action"], "install");
    }
    assert!(value["plan"]["validators"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "AUTH-001"));
}

#[test]
fn resolve_failure_names_blocking_edges_before_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    // Missing version (unknown id).
    let out = run(
        &db,
        &["feature", "resolve", "rust-web", "--feature", "nosuch"],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[unknown-feature]"),
        "{}",
        lossy(&out.stderr)
    );

    // Unsupported mapping: reports unsupported instead of inventing.
    let out = run(
        &db,
        &["feature", "resolve", "flutter-app", "--feature", "postgres"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr).to_string();
    assert!(stderr.contains("error[incompatible-feature]"), "{stderr}");
    assert!(stderr.contains("no implementation"), "{stderr}");
}

#[test]
fn add_then_upgrade_keeps_manifest_registry_agreeing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    rust_l1(&proj, "feat-app");

    let out = run(
        &db,
        &["feature", "add", "admin", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout).to_string();
    assert!(stdout.contains("added feature 'admin'"), "{stdout}");
    assert!(proj.join(".forge/features/auth.receipt").is_file());
    assert!(proj.join(".forge/features/admin.receipt").is_file());
    let text = manifest_text(&proj);
    assert!(text.contains("auth:"), "{text}");
    assert!(text.contains("admin:"), "{text}");

    let inspect = run_json(&db, &["inspect", "feat-app"]);
    assert_eq!(inspect["features"]["auth"], "0.1.0");
    assert_eq!(inspect["features"]["admin"], "0.1.0");

    // Exact reinstall is a no-op with no duplicate registration.
    let out = run(
        &db,
        &["feature", "add", "admin", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stdout).contains("already installed"),
        "{}",
        lossy(&out.stdout)
    );

    // Older spelling upgrades to the tested version. Age both the manifest
    // and the receipt to model a clean older install, then upgrade.
    let aged = text
        .replace("auth: 0.1.0", "auth: 0.0.9")
        .replace("auth: \"0.1.0\"", "auth: 0.0.9");
    assert_ne!(aged, text);
    fs::write(proj.join("forge.yaml"), &aged).unwrap();
    let receipt = proj.join(".forge/features/auth.receipt");
    let receipt_text = fs::read_to_string(&receipt).unwrap();
    let aged_receipt = receipt_text.replace("version: 0.1.0", "version: 0.0.9");
    assert_ne!(aged_receipt, receipt_text);
    fs::write(&receipt, &aged_receipt).unwrap();

    let out = run(
        &db,
        &["feature", "upgrade", "auth", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stdout).contains("upgraded feature 'auth'"),
        "{}",
        lossy(&out.stdout)
    );
    let inspect = run_json(&db, &["inspect", "feat-app"]);
    assert_eq!(inspect["features"]["auth"], "0.1.0");
    assert_eq!(inspect["features"]["admin"], "0.1.0");
    let reparsed = manifest_text(&proj);
    assert!(reparsed.contains("auth:"), "{reparsed}");

    // Already-current upgrade is a no-op.
    let out = run(
        &db,
        &["feature", "upgrade", "auth", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stdout).contains("already at version"),
        "{}",
        lossy(&out.stdout)
    );
}

#[test]
fn unknown_version_fails_before_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    rust_l1(&proj, "ver-app");
    let before = manifest_text(&proj);

    let out = run(
        &db,
        &[
            "feature",
            "add",
            "auth",
            &proj.display().to_string(),
            "--version",
            "9.9.9",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[incompatible-feature]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(manifest_text(&proj), before);
    assert!(!proj.join(".forge").exists());
}

#[test]
fn remove_blocked_by_reverse_dependency_preserves_files() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    rust_l1(&proj, "dep-app");

    let out = run(
        &db,
        &["feature", "add", "admin", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let before = manifest_text(&proj);

    let out = run(
        &db,
        &["feature", "remove", "auth", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr).to_string();
    assert!(stderr.contains("error[incompatible-feature]"), "{stderr}");
    assert!(stderr.contains("admin"), "{stderr}");
    assert_eq!(manifest_text(&proj), before);
    assert!(proj.join(".forge/features/auth.receipt").is_file());

    // Removing the dependent first unblocks the dependency.
    let out = run(
        &db,
        &["feature", "remove", "admin", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let out = run(
        &db,
        &["feature", "remove", "auth", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(!proj.join(".forge/features/auth.receipt").exists());
    let inspect = run_json(&db, &["inspect", "dep-app"]);
    assert!(inspect["features"].as_object().unwrap().is_empty());
}

#[test]
fn remove_blocked_by_user_owned_edits_preserves_files() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    rust_l1(&proj, "owned-app");

    let out = run(
        &db,
        &["feature", "add", "telemetry", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let receipt = proj.join(".forge/features/telemetry.receipt");
    fs::write(&receipt, "operator runbook notes\n").unwrap();
    let before = manifest_text(&proj);

    let out = run(
        &db,
        &[
            "feature",
            "remove",
            "telemetry",
            &proj.display().to_string(),
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[feature-ownership-conflict]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(manifest_text(&proj), before);
    assert_eq!(
        fs::read_to_string(&receipt).unwrap(),
        "operator runbook notes\n"
    );
}

#[test]
fn new_records_dependency_closure() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("closed-app");

    let out = run(
        &db,
        &[
            "new",
            &dest.display().to_string(),
            "--profile",
            "rust-web",
            "--feature",
            "admin",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let text = manifest_text(&dest);
    assert!(text.contains("auth:"), "{text}");
    assert!(text.contains("admin:"), "{text}");
    let inspect = run_json(&db, &["inspect", "closed-app"]);
    assert_eq!(inspect["features"]["auth"], "0.1.0");
    assert_eq!(inspect["features"]["admin"], "0.1.0");
}

#[test]
fn operations_are_repeatable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    rust_l1(&proj, "repeat-app");

    let out = run(
        &db,
        &["feature", "add", "telemetry", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let first_manifest = manifest_text(&proj);
    let first_receipt = fs::read(proj.join(".forge/features/telemetry.receipt")).unwrap();

    // Second add is a no-op: bytes identical, registry agrees.
    let out = run(
        &db,
        &["feature", "add", "telemetry", &proj.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert_eq!(manifest_text(&proj), first_manifest);
    assert_eq!(
        fs::read(proj.join(".forge/features/telemetry.receipt")).unwrap(),
        first_receipt
    );
    let inspect = run_json(&db, &["inspect", "repeat-app"]);
    assert_eq!(inspect["features"]["telemetry"], "0.1.0");
}
