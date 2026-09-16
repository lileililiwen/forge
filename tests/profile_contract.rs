//! Profile registry contract: discovery, compatibility and preflight.
//!
//! Covers the `profile-registry` scenarios end to end through the built
//! binary: list/inspect show all five MVP descriptors, malformed input
//! names its missing field (Core level), incompatible resolution fails
//! before file changes with a backend hint, and missing toolchains are
//! reported without claiming the profile was tested.

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

const MVP_IDS: [&str; 5] = [
    "aspnet-web",
    "flutter-app",
    "nextjs-web",
    "python-service",
    "rust-web",
];

#[test]
fn profile_list_shows_all_five_ids_and_versions() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");

    let out = run(&db, &["profile", "list"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    for id in MVP_IDS {
        assert!(text.contains(id), "list must show {id}:\n{text}");
    }
    assert!(text.contains("0.1.0"), "{text}");

    // Repeatability: second run renders the same collection.
    let again = run(&db, &["profile", "list"]);
    assert_eq!(again.status.code(), Some(0));
    assert_eq!(lossy(&again.stdout), text);

    let out = run_json(&db, &["profile", "list"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("profile list must emit JSON");
    let profiles = value["profiles"].as_array().expect("profiles array");
    assert_eq!(profiles.len(), 5);
    let mut ids: Vec<&str> = profiles
        .iter()
        .map(|p| p["id"].as_str().expect("profile id"))
        .collect();
    ids.sort_unstable();
    let mut expected = MVP_IDS.to_vec();
    expected.sort_unstable();
    assert_eq!(ids, expected);
    for p in profiles {
        assert!(!p["version"].as_str().unwrap_or("").is_empty());
        assert!(!p["adapter"].as_str().unwrap_or("").is_empty());
        assert!(!p["build_command"].as_str().unwrap_or("").is_empty());
        assert!(!p["test_command"].as_str().unwrap_or("").is_empty());
    }
}

#[test]
fn profile_inspect_returns_full_descriptor() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");

    let out = run(&db, &["profile", "inspect", "rust-web"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    for field in [
        "adapter-rust",
        "cargo build",
        "cargo test",
        "requires_database: yes",
    ] {
        assert!(text.contains(field), "inspect must show {field}:\n{text}");
    }

    let out = run_json(&db, &["profile", "inspect", "flutter-app"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("inspect must emit JSON");
    assert_eq!(value["id"], "flutter-app");
    assert_eq!(value["requires_database"], false);
    let caps = value["capabilities"].as_array().expect("capabilities");
    assert!(!caps.iter().any(|c| c == "postgres"));

    // Boundary: a valid profile without a database requirement forces none.
    let out = run_json(&db, &["profile", "inspect", "nextjs-web"]);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["requires_database"], false);
}

#[test]
fn unknown_profile_returns_structured_error() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");

    let out = run(&db, &["profile", "inspect", "react-web"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[unknown-profile]"),
        "{}",
        lossy(&out.stderr)
    );

    let out = run_json(&db, &["profile", "inspect", "react-web"]);
    assert_eq!(out.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&out.stderr).expect("error must be JSON");
    assert_eq!(value["error"]["code"], "unknown-profile");
}

#[test]
fn resolve_returns_exact_version_and_adapter() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");

    let out = run(
        &db,
        &["profile", "resolve", "rust-web", "--feature", "auth"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(text.contains("0.1.0"), "{text}");
    assert!(text.contains("adapter-rust"), "{text}");

    let out = run_json(
        &db,
        &["profile", "resolve", "rust-web", "--feature", "auth"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["resolved"]["version"], "0.1.0");
    assert_eq!(value["resolved"]["adapter"], "adapter-rust");
}

#[test]
fn flutter_postgres_resolution_fails_before_file_changes() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let before: Vec<PathBuf> = collect_files(dir.path());

    let out = run(
        &db,
        &["profile", "resolve", "flutter-app", "--feature", "postgres"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[incompatible-profile]"), "{stderr}");
    assert!(stderr.contains("backend"), "{stderr}");
    assert!(stderr.contains("no files were changed"), "{stderr}");

    // Read-only failure: no new files and no registry row side effects.
    assert_eq!(collect_files(dir.path()), before);
    let list = run(&db, &["list"]);
    assert_eq!(list.status.code(), Some(0));
    assert!(lossy(&list.stdout).contains("No projects registered"));
}

#[test]
fn preflight_reports_missing_toolchain_without_claiming_tested() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let empty_path = tempfile::tempdir().unwrap();

    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("profile")
        .arg("preflight")
        .arg("rust-web")
        .env("PATH", empty_path.path());
    let out = cmd.output().expect("run forge");
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[toolchain-missing]"), "{stderr}");
    assert!(stderr.contains("cargo"), "{stderr}");
    assert!(stderr.contains("not tested"), "{stderr}");
}

#[test]
fn register_rejects_unknown_profile_and_incompatible_features() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    // Unknown profile: registration fails, manifest untouched, nothing stored.
    let proj = tmp.path().join("unknown-profile-proj");
    fs::create_dir(&proj).unwrap();
    let manifest =
        "schema: 1\nproject:\n  id: unknown-prof\n  name: Unknown Prof\n  profile: react-web\n";
    fs::write(proj.join("forge.yaml"), manifest).unwrap();
    let out = run(&db, &["register", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stderr).contains("error[unknown-profile]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(
        fs::read_to_string(proj.join("forge.yaml")).unwrap(),
        manifest
    );
    assert_eq!(
        run(&db, &["inspect", "unknown-prof"]).status.code(),
        Some(1)
    );

    // Flutter + server-side postgres: registration fails before mutation.
    let mobile = tmp.path().join("mobile-proj");
    fs::create_dir(&mobile).unwrap();
    let mobile_manifest = "schema: 1\nproject:\n  id: mobile-app\n  name: Mobile App\n  profile: flutter-app\nfeatures:\n  postgres: 2.0\n";
    fs::write(mobile.join("forge.yaml"), mobile_manifest).unwrap();
    let out = run(&db, &["register", &mobile.display().to_string()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stderr).contains("error[incompatible-profile]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(
        fs::read_to_string(mobile.join("forge.yaml")).unwrap(),
        mobile_manifest
    );
    assert_eq!(run(&db, &["inspect", "mobile-app"]).status.code(), Some(1));

    // Compatible registration still succeeds after the rejections.
    let ok = tmp.path().join("ok-proj");
    fs::create_dir(&ok).unwrap();
    fs::write(
        ok.join("forge.yaml"),
        "schema: 1\nproject:\n  id: ok-svc\n  name: Ok Svc\n  profile: rust-web\nfeatures:\n  postgres: 2.0\n",
    )
    .unwrap();
    let out = run(&db, &["register", &ok.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert_eq!(run(&db, &["inspect", "ok-svc"]).status.code(), Some(0));
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

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}
