//! Graduation import CLI and in-process library contract
//! (`hypora-graduation-import`).
//!
//! Every case runs through the built `forge` binary. The artifact is
//! the local conformance oracle: `platform.idea-graduation` is not
//! vendored in this repository yet, so the fixtures below are the
//! pinned shape. No Hypora endpoint is contacted.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

const CONTRACT: &str = "forge-graduation-import/0.1.0";
const IDEA_CONTRACT: &str = "platform.idea-graduation/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
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

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut all: Vec<&str> = vec!["--format", "json"];
    all.extend_from_slice(args);
    let out = run(db, &all);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("valid json")
}

fn run_stdin(db: &Path, args: &[&str], input: &str) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait forge")
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_json(dir: &Path, name: &str, value: &Value) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    path
}

fn valid_artifact() -> Value {
    json!({
        "contract": IDEA_CONTRACT,
        "hypora_project_id": "prj_01H",
        "hypora_revision": "rev-2026-09-20-3",
        "graduated_at": "2026-09-20T00:00:00Z",
        "brief": {
            "title": "GPA Simulator",
            "problem": "Students cannot see how a term changes their GPA.",
            "audience": "University students planning a term.",
            "solution": "A planner that projects cumulative GPA per course set.",
            "requirements": [
                "Model terms, courses, credits and grades.",
                "Project cumulative GPA for a hypothetical course set."
            ],
            "success_metrics": [
                { "name": "graded_course_sets_saved", "target": ">= 100", "window": "30d" }
            ]
        },
        "experiment": {
            "summary": "40 students completed the projection task in the probe.",
            "validated": true,
            "evidence": [
                {
                    "kind": "probe-completion",
                    "excerpt": "Aggregate: 40 of 52 completed.",
                    "observed_at": "2026-09-18T00:00:00Z"
                }
            ]
        }
    })
}

fn artifact_path(tmp: &Path, value: &Value) -> PathBuf {
    write_json(tmp, "artifact.json", value)
}

fn assert_refusal(db: &Path, artifact: &Path, needle: &str) {
    let out = run(
        db,
        &["graduation", "preview", &artifact.display().to_string()],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected a refusal; stdout={} stderr={}",
        lossy(&out.stdout),
        lossy(&out.stderr)
    );
    assert!(
        out.stdout.is_empty(),
        "stdout must be empty: {}",
        lossy(&out.stdout)
    );
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[graduation-invalid]"), "{stderr}");
    assert!(stderr.contains(needle), "missing {needle}:\n{stderr}");
}

#[test]
fn graduation_help_advertises_both_subcommands_and_their_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["graduation", "--help"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let help = lossy(&out.stdout);
    assert!(help.contains("preview"), "{help}");
    assert!(help.contains("import"), "{help}");

    let out = run(&db, &["graduation", "import", "--help"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let help = lossy(&out.stdout);
    for flag in ["--path", "--profile", "--id", "--actor", "--confirm"] {
        assert!(help.contains(flag), "missing {flag}:\n{help}");
    }
}

#[test]
fn a_valid_preview_prints_the_mapped_brief_and_provenance_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let artifact = artifact_path(tmp.path(), &valid_artifact());
    let before = fs::read(&artifact).unwrap();

    let out = run(
        &db,
        &["graduation", "preview", &artifact.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    for needle in [
        "GPA Simulator",
        IDEA_CONTRACT,
        "hypora_project=prj_01H",
        "evidence=1",
        "Aggregate: 40 of 52 completed.",
        "No files were written",
    ] {
        assert!(text.contains(needle), "missing {needle}:\n{text}");
    }

    assert_eq!(fs::read(&artifact).unwrap(), before);
    assert!(!db.exists(), "a preview must not create a registry");
}

#[test]
fn import_without_confirm_is_a_dry_run_with_empty_side_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let artifact = artifact_path(tmp.path(), &valid_artifact());
    let dest = tmp.path().join("gpa-sim");
    let dest_s = dest.display().to_string();

    let out = run(
        &db,
        &[
            "graduation",
            "import",
            &artifact.display().to_string(),
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(text.contains("proposed project:"), "{text}");
    assert!(text.contains("id: gpa-simulator"), "{text}");
    assert!(text.contains("destination:"), "{text}");
    assert!(text.contains("No files were written"), "{text}");
    assert!(!dest.exists(), "a dry run must not create the destination");
    assert!(!db.exists(), "a dry run must not create a registry");
}

#[test]
fn import_with_confirm_creates_one_project_and_a_receipt() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let artifact = artifact_path(tmp.path(), &valid_artifact());
    let dest = tmp.path().join("gpa-sim");
    let dest_s = dest.display().to_string();

    let out = run(
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
    let text = lossy(&out.stdout);
    assert!(text.contains("imported gpa-simulator"), "{text}");
    assert!(
        text.contains("No scaffold, deploy, network call or gate approval"),
        "{text}"
    );
    assert!(dest.join("forge.yaml").is_file());
    assert!(dest
        .join(".forge/graduation/gpa-simulator/import.json")
        .is_file());

    let list = run(&db, &["list"]);
    assert_eq!(list.status.code(), Some(0));
    assert!(lossy(&list.stdout).contains("gpa-simulator"));
}

#[test]
fn every_refusal_is_a_typed_error_with_empty_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    // Malformed JSON.
    let malformed = tmp.path().join("malformed.json");
    fs::write(&malformed, "{ not json").unwrap();
    assert_refusal(&db, &malformed, "not valid JSON");

    // Unknown key at the artifact level.
    let mut unknown = valid_artifact();
    unknown["insight"] = json!("extra");
    assert_refusal(&db, &artifact_path(tmp.path(), &unknown), "insight");

    // Not validated.
    let mut unvalidated = valid_artifact();
    unvalidated["experiment"]["validated"] = json!(false);
    assert_refusal(
        &db,
        &artifact_path(tmp.path(), &unvalidated),
        "graduation-not-validated",
    );

    // PII and secret shapes.
    let mut pii = valid_artifact();
    pii["email"] = json!("ops@example.com");
    assert_refusal(
        &db,
        &artifact_path(tmp.path(), &pii),
        "graduation-identity-refused",
    );
    let mut secret = valid_artifact();
    secret["brief"]["problem"] = json!("token=ghp_abcdefghijklmnopqrstuvwxyz0123456789");
    assert_refusal(
        &db,
        &artifact_path(tmp.path(), &secret),
        "graduation-secret-refused",
    );
}

#[test]
fn an_unsupported_major_and_an_unknown_revision_are_named() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let mut major = valid_artifact();
    major["contract"] = json!("platform.idea-graduation/1.0.0");
    assert_refusal(
        &db,
        &artifact_path(tmp.path(), &major),
        "graduation-major-unsupported",
    );

    let mut revision = valid_artifact();
    revision["contract"] = json!("platform.idea-graduation/0.2.0");
    assert_refusal(
        &db,
        &artifact_path(tmp.path(), &revision),
        "graduation-revision-unsupported",
    );
}

#[test]
fn a_pii_or_secret_field_refuses_the_whole_import() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();

    let mut artifact = valid_artifact();
    artifact["brief"]["audience"] = json!("ops@example.com");
    let path = artifact_path(tmp.path(), &artifact);

    let out = run(
        &db,
        &[
            "graduation",
            "import",
            &path.display().to_string(),
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    assert!(out.stdout.is_empty(), "{}", lossy(&out.stdout));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[graduation-invalid]"), "{stderr}");
    assert!(stderr.contains("graduation-identity-refused"), "{stderr}");
    assert!(
        !stderr.contains("ops@example.com"),
        "value leaked: {stderr}"
    );
    assert!(!dest.exists(), "nothing may be written");
    assert!(!db.exists(), "nothing may be registered");
}

#[test]
fn an_oversized_or_malformed_file_is_refused_before_any_write() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();

    let oversized = tmp.path().join("oversized.json");
    fs::write(&oversized, "x".repeat(1024 * 1024 + 1)).unwrap();
    let out = run(
        &db,
        &[
            "graduation",
            "import",
            &oversized.display().to_string(),
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("larger than"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!dest.exists());
    assert!(!db.exists());

    let missing = tmp.path().join("no-such-file.json");
    let out = run(
        &db,
        &["graduation", "preview", &missing.display().to_string()],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[path-unavailable]"),
        "{}",
        lossy(&out.stderr)
    );
}

#[test]
fn an_unknown_profile_and_a_manifest_bearing_destination_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let artifact = artifact_path(tmp.path(), &valid_artifact());
    let artifact_s = artifact.display().to_string();

    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();
    let out = run(
        &db,
        &[
            "graduation",
            "import",
            &artifact_s,
            "--path",
            &dest_s,
            "--profile",
            "not-a-profile",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[unknown-profile]"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!dest.exists());

    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("forge.yaml"), "schema: 1\n").unwrap();
    let out = run(
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
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[graduation-conflict]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(
        fs::read_to_string(dest.join("forge.yaml")).unwrap(),
        "schema: 1\n"
    );
}

#[test]
fn a_reserved_id_and_a_registered_id_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let artifact = artifact_path(tmp.path(), &valid_artifact());
    let artifact_s = artifact.display().to_string();

    let dest = tmp.path().join("dest");
    let dest_s = dest.display().to_string();
    let out = run(
        &db,
        &[
            "graduation",
            "import",
            &artifact_s,
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--id",
            "Bad_ID",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[graduation-conflict]"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!dest.exists());

    // Register the id at one destination, then reuse it at another.
    let first = tmp.path().join("first");
    let first_s = first.display().to_string();
    let out = run(
        &db,
        &[
            "graduation",
            "import",
            &artifact_s,
            "--path",
            &first_s,
            "--profile",
            "rust-web",
            "--id",
            "taken",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let second = tmp.path().join("second");
    let second_s = second.display().to_string();
    let out = run(
        &db,
        &[
            "graduation",
            "import",
            &artifact_s,
            "--path",
            &second_s,
            "--profile",
            "rust-web",
            "--id",
            "taken",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[id-collision]"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!second.exists(), "nothing may be written on a collision");
}

#[test]
fn stdin_is_accepted_with_and_without_confirm() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let input = serde_json::to_string(&valid_artifact()).unwrap();

    let out = run_stdin(&db, &["graduation", "preview", "-"], &input);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(lossy(&out.stdout).contains("GPA Simulator"));

    let dest = tmp.path().join("gpa-sim");
    let dest_s = dest.display().to_string();
    let out = run_stdin(
        &db,
        &[
            "graduation",
            "import",
            "-",
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
            "--confirm",
        ],
        &input,
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(dest.join("forge.yaml").is_file());
    assert!(dest
        .join(".forge/graduation/gpa-simulator/import.json")
        .is_file());
}

#[test]
fn the_json_envelopes_carry_the_preview_and_the_import() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let artifact = artifact_path(tmp.path(), &valid_artifact());
    let artifact_s = artifact.display().to_string();

    let value = run_json(&db, &["graduation", "preview", &artifact_s]);
    assert_eq!(value["contract"], CONTRACT);
    assert_eq!(value["preview"]["artifact"], artifact_s);
    assert_eq!(value["preview"]["source"]["contract"], IDEA_CONTRACT);
    assert_eq!(value["preview"]["source"]["major"], 0);
    assert_eq!(value["preview"]["source"]["schema_revision"], "0.1.0");
    assert_eq!(value["preview"]["brief"]["title"], "GPA Simulator");
    assert_eq!(
        value["preview"]["experiment"]["evidence"][0]["kind"],
        "probe-completion"
    );
    assert!(value["preview"]["proposed"].is_null());

    let dest = tmp.path().join("gpa-sim");
    let dest_s = dest.display().to_string();
    let value = run_json(
        &db,
        &[
            "graduation",
            "import",
            &artifact_s,
            "--path",
            &dest_s,
            "--profile",
            "rust-web",
        ],
    );
    assert_eq!(value["preview"]["proposed"]["id"], "gpa-simulator");
    assert_eq!(value["preview"]["proposed"]["profile"], "rust-web");

    let value = run_json(
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
    assert_eq!(value["imported"]["id"], "gpa-simulator");
    assert_eq!(value["imported"]["profile"], "rust-web");
    assert_eq!(value["graduation"]["contract"], CONTRACT);
    assert_eq!(value["graduation"]["source"]["contract"], IDEA_CONTRACT);
    assert_eq!(
        value["graduation"]["source"]["hypora_revision"],
        "rev-2026-09-20-3"
    );
    assert!(
        value["graduation"]["receipt"]
            .as_str()
            .unwrap()
            .ends_with(".forge/graduation/gpa-simulator/import.json"),
        "{}",
        value["graduation"]["receipt"]
    );
}

#[test]
fn the_artifact_may_not_name_a_profile() {
    // An architecture cannot be smuggled in: `profile` is not an
    // accepted key, and the required `--profile` is refused when missing.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let mut artifact = valid_artifact();
    artifact["profile"] = json!("rust-web");
    assert_refusal(&db, &artifact_path(tmp.path(), &artifact), "profile");

    let artifact = artifact_path(tmp.path(), &valid_artifact());
    let dest = tmp.path().join("dest");
    let out = run(
        &db,
        &[
            "graduation",
            "import",
            &artifact.display().to_string(),
            "--path",
            &dest.display().to_string(),
        ],
    );
    assert_eq!(out.status.code(), Some(2), "missing --profile must refuse");
    assert!(!dest.exists());
}
