//! Upgrade orchestration contract: deterministic upgrade planning and
//! isolated fleet outcomes through the built binary.
//!
//! Covers the `project-upgrade-orchestration` scenarios end to end:
//! successful deterministic plans with exact old/new versions and recovery
//! notes, semantic-conflict handoff when user-owned receipts drift,
//! already-satisfied no-op without rewriting files, successful fleet
//! upgrades with per-project journals, fleet isolation when one project
//! blocks without marking the whole fleet healthy, and retry semantics
//! that skip already-satisfied projects while re-planning on changed
//! preconditions.

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
    let (value, _code) = run_json_with_code(db, args);
    value
}

fn run_json_with_code(db: &Path, args: &[&str]) -> (serde_json::Value, Option<i32>) {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    let code = out.status.code();
    let value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stderr={}", lossy(&out.stderr)));
    (value, code)
}

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_rust_project(dir: &Path, id: &str, features: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n{features}"
    );
    fs::write(dir.join("forge.yaml"), &text).unwrap();
}

fn manifest_text(dir: &Path) -> String {
    fs::read_to_string(dir.join("forge.yaml")).unwrap()
}

fn write_clean_receipt(proj: &Path, feature: &str, version: &str) {
    let descriptor = forge::feature::inspect_feature(feature).unwrap();
    let receipt = proj.join(format!(".forge/features/{feature}.receipt"));
    fs::create_dir_all(receipt.parent().unwrap()).unwrap();
    fs::write(
        &receipt,
        forge::feature::expected_receipt(&descriptor, version),
    )
    .unwrap();
}

#[test]
fn dry_run_plan_describes_old_new_versions_assets_and_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "plan-app", "features:\n  auth: \"0.0.9\"\n");

    let value = run_json(&db, &["upgrade", "--dry-run", proj.to_str().unwrap()]);
    let plan = &value["plan"];
    assert_eq!(plan["project_id"], "plan-app");
    assert_eq!(plan["profile"], "rust-web");
    assert!(!plan["contract"].as_str().unwrap().is_empty());
    let steps = plan["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    let step = &steps[0];
    assert_eq!(step["feature"], "auth");
    assert_eq!(step["old_version"], "0.0.9");
    assert_eq!(step["new_version"], "0.1.0");
    assert_eq!(step["action"], "upgrade");
    assert!(step["kinds"]
        .as_array()
        .unwrap()
        .contains(&"package".into()));
    assert!(step["assets"]
        .as_array()
        .unwrap()
        .contains(&"forge.yaml".into()));
    assert!(step["reversible"].as_bool().unwrap());
    assert!(step["recovery"].as_str().unwrap().contains("reversible"));
    // dry-run must not change the manifest.
    assert!(manifest_text(&proj).contains("auth: \"0.0.9\""));
}

#[test]
fn apply_upgrade_reports_changed_files_and_advances_versions() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "apply-app", "features:\n  auth: \"0.0.9\"\n");
    write_clean_receipt(&proj, "auth", "0.0.9");

    let out = run(&db, &["upgrade", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value = run_json(&db, &["upgrade", "--dry-run", proj.to_str().unwrap()]);
    let plan = &value["plan"];
    let steps = plan["steps"].as_array().unwrap();
    let auth = steps.iter().find(|s| s["feature"] == "auth").unwrap();
    // Already-satisfied: the previous run already moved auth to 0.1.0.
    assert_eq!(auth["action"], "noop");
    assert!(!auth["kinds"]
        .as_array()
        .unwrap()
        .contains(&"package".into()));
    let text = manifest_text(&proj);
    assert!(text.contains("auth: 0.1.0"), "{text}");
    let inspect = run_json(&db, &["inspect", "apply-app"]);
    assert_eq!(inspect["features"]["auth"], "0.1.0");
}

#[test]
fn drifted_receipt_blocks_with_semantic_conflict_handoff() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "drift-app", "features:\n  auth: \"0.0.9\"\n");
    write_clean_receipt(&proj, "auth", "0.0.9");
    let receipt = proj.join(".forge/features/auth.receipt");
    fs::write(&receipt, "operator runbook notes\n").unwrap();
    let before = manifest_text(&proj);

    let out = run(&db, &["upgrade", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr).to_string();
    assert!(
        stderr.contains("error[feature-ownership-conflict]"),
        "{stderr}"
    );
    assert!(stderr.contains("semantic-conflict handoff"), "{stderr}");
    assert!(stderr.contains("forge spec generate"), "{stderr}");
    assert_eq!(manifest_text(&proj), before);
    let receipt_after = fs::read_to_string(&receipt).unwrap();
    assert_eq!(receipt_after, "operator runbook notes\n");
}

#[test]
fn already_satisfied_upgrade_is_a_no_op_without_file_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "sat-app", "features:\n  auth: \"0.1.0\"\n");
    let before = manifest_text(&proj);

    let out = run(&db, &["upgrade", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout).to_string();
    assert!(stdout.contains("already satisfied"), "{stdout}");
    assert_eq!(manifest_text(&proj), before);
}

#[test]
fn requested_unknown_feature_fails_before_any_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "unk-app", "");
    let before = manifest_text(&proj);

    let out = run(
        &db,
        &["upgrade", "--feature", "nosuch", proj.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr).to_string();
    assert!(stderr.contains("error[unknown-feature]"), "{stderr}");
    assert_eq!(manifest_text(&proj), before);
    assert!(!proj.join(".forge/features").exists());
}

#[test]
fn requested_missing_feature_installs_at_pinned_version() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "install-app", "");

    let out = run(
        &db,
        &["upgrade", "--feature", "privacy", proj.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout).to_string();
    assert!(stdout.contains("privacy"), "{stdout}");
    let inspect = run_json(&db, &["inspect", "install-app"]);
    assert_eq!(inspect["features"]["privacy"], "0.1.0");
    assert!(proj.join(".forge/features/privacy.receipt").is_file());
}

#[test]
fn postgres_feature_marks_schema_irreversible_with_declared_strategy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "schema-app", "features:\n  postgres: \"0.0.9\"\n");
    write_clean_receipt(&proj, "postgres", "0.0.9");

    let value = run_json(&db, &["upgrade", "--dry-run", proj.to_str().unwrap()]);
    let steps = value["plan"]["steps"].as_array().unwrap();
    let postgres = steps
        .iter()
        .find(|s| s["feature"] == "postgres")
        .expect("postgres step");
    assert!(postgres["kinds"]
        .as_array()
        .unwrap()
        .contains(&"schema".into()));
    assert_eq!(postgres["reversible"], false);
    assert_eq!(
        postgres["migration_strategy"],
        "manifest-repin+manual-schema-review"
    );
    let recovery = postgres["recovery"].as_str().unwrap();
    assert!(recovery.contains("does NOT reverse"), "{recovery}");
}

#[test]
fn fleet_upgrade_completes_two_projects_with_per_project_journals() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let first = tmp.path().join("first");
    let second = tmp.path().join("second");
    write_rust_project(&first, "fleet-one", "features:\n  auth: \"0.0.9\"\n");
    write_rust_project(&second, "fleet-two", "features:\n  telemetry: \"0.0.9\"\n");
    let out = run(&db, &["register", first.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let out = run(&db, &["register", second.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    write_clean_receipt(&first, "auth", "0.0.9");
    write_clean_receipt(&second, "telemetry", "0.0.9");

    let value = run_json(&db, &["upgrade", "--all"]);
    let fleet = &value["fleet"];
    assert_eq!(fleet["selection"][0], "fleet-one");
    assert_eq!(fleet["selection"][1], "fleet-two");
    assert_eq!(fleet["succeeded"], 2);
    assert_eq!(fleet["failed"], 0);
    assert_eq!(fleet["blocked"], 0);
    assert_eq!(fleet["dry_run"], false);
    let entries = fleet["entries"].as_array().unwrap();
    for entry in entries {
        assert_eq!(entry["status"], "success");
        assert_eq!(entry["changed"], true);
        assert!(!entry["validation"].as_array().unwrap().is_empty());
        assert!(!entry["recovery"].as_array().unwrap().is_empty());
    }
    let inspect_one = run_json(&db, &["inspect", "fleet-one"]);
    assert_eq!(inspect_one["features"]["auth"], "0.1.0");
    let inspect_two = run_json(&db, &["inspect", "fleet-two"]);
    assert_eq!(inspect_two["features"]["telemetry"], "0.1.0");
}

#[test]
fn fleet_isolates_blocked_project_without_wholesale_success() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let good = tmp.path().join("good");
    let bad = tmp.path().join("bad");
    write_rust_project(&good, "fleet-good", "features:\n  auth: \"0.0.9\"\n");
    write_rust_project(
        &bad,
        "fleet-bad",
        "features:\n  auth: \"0.0.9\"\n  telemetry: \"0.0.9\"\n",
    );
    let out = run(&db, &["register", good.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let out = run(&db, &["register", bad.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    write_clean_receipt(&good, "auth", "0.0.9");
    write_clean_receipt(&bad, "auth", "0.0.9");
    write_clean_receipt(&bad, "telemetry", "0.0.9");
    // Drift `bad`'s telemetry receipt so the precondition sweep blocks.
    fs::write(
        bad.join(".forge/features/telemetry.receipt"),
        "operator notes\n",
    )
    .unwrap();

    let (value, code) = run_json_with_code(&db, &["upgrade", "--all"]);
    assert_eq!(code, Some(1), "blocked fleet must exit 1");
    let fleet = &value["fleet"];
    assert_eq!(fleet["failed"], 0);
    let blocked = fleet["blocked"].as_i64().unwrap();
    assert!(blocked >= 1, "{fleet}");
    let bad_entry = fleet["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["project_id"] == "fleet-bad")
        .unwrap();
    assert_eq!(bad_entry["status"], "blocked");
    assert_eq!(bad_entry["changed"], false);
    assert!(!bad_entry["recovery"].as_array().unwrap().is_empty());
    // Healthy sibling still completed.
    let good_entry = fleet["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["project_id"] == "fleet-good")
        .unwrap();
    assert_eq!(good_entry["status"], "success");
    let inspect_good = run_json(&db, &["inspect", "fleet-good"]);
    assert_eq!(inspect_good["features"]["auth"], "0.1.0");
    // `bad` was not touched because the precondition sweep ran before any
    // mutation; auth remains at the older version.
    let bad_manifest = manifest_text(&bad);
    assert!(bad_manifest.contains("auth: \"0.0.9\""), "{bad_manifest}");
}

#[test]
fn fleet_retry_skips_satisfied_projects_and_replans_on_drift() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "retry-app", "features:\n  auth: \"0.0.9\"\n");
    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    write_clean_receipt(&proj, "auth", "0.0.9");

    // First run upgrades auth to 0.1.0 and journals the success.
    let value = run_json(&db, &["upgrade", "--all"]);
    assert_eq!(value["fleet"]["succeeded"], 1);

    // Retry: already satisfied, no journal entry beyond the previous one.
    let value = run_json(&db, &["upgrade", "--all"]);
    let fleet = &value["fleet"];
    assert_eq!(fleet["succeeded"], 0);
    assert_eq!(fleet["skipped"], 1);
    let entry = &fleet["entries"][0];
    assert_eq!(entry["status"], "skipped");
    assert_eq!(entry["changed"], false);

    // Operator ages auth back and drifts the receipt: retry must re-plan
    // and block instead of blindly reporting success.
    let text = manifest_text(&proj);
    let aged = text.replace("auth: 0.1.0", "auth: \"0.0.8\"");
    assert_ne!(aged, text);
    fs::write(proj.join("forge.yaml"), &aged).unwrap();
    write_clean_receipt(&proj, "auth", "0.0.8");
    fs::write(
        proj.join(".forge/features/auth.receipt"),
        "operator notes\n",
    )
    .unwrap();
    let (value, code) = run_json_with_code(&db, &["upgrade", "--all"]);
    assert_eq!(code, Some(1), "blocked retry must exit 1");
    let fleet = &value["fleet"];
    assert_eq!(fleet["blocked"], 1);
    let entry = &fleet["entries"][0];
    assert_eq!(entry["status"], "blocked");
    assert!(entry["conflict"].is_object());
    assert!(entry["conflict"]["suggested_spec"]
        .as_str()
        .unwrap()
        .contains("forge spec generate"));
}

#[test]
fn fleet_dry_run_skips_writes_and_keeps_files_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "dry-app", "features:\n  auth: \"0.0.9\"\n");
    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    write_clean_receipt(&proj, "auth", "0.0.9");
    let before = manifest_text(&proj);

    let value = run_json(&db, &["upgrade", "--all", "--dry-run"]);
    let fleet = &value["fleet"];
    assert_eq!(fleet["dry_run"], true);
    let entry = &fleet["entries"][0];
    assert_eq!(entry["status"], "skipped");
    assert_eq!(entry["changed"], false);
    assert!(entry["note"].as_str().unwrap().contains("plan only"));
    assert_eq!(manifest_text(&proj), before);
    let inspect = run_json(&db, &["inspect", "dry-app"]);
    assert_eq!(inspect["features"]["auth"], "0.0.9");
}

#[test]
fn manifest_sections_survive_upgrade_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let text = "schema: 1\nproject:\n  id: keep-up\n  name: Keep\n  profile: rust-web\n  maturity: L2\nruntime:\n  language: rust\n  version: stable\ndeployment:\n  type: docker\n  target: home-server-01\ndocs:\n  source_language: en\nfeatures:\n  auth: \"0.0.9\"\n";
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("forge.yaml"), text).unwrap();
    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    write_clean_receipt(&proj, "auth", "0.0.9");

    let out = run(&db, &["upgrade", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let after = manifest_text(&proj);
    assert!(after.contains("auth: 0.1.0"), "{after}");
    assert!(after.contains("target: home-server-01"), "{after}");
    assert!(after.contains("source_language: en"), "{after}");
}
