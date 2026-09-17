//! Spec generation and remediation routing contract
//! (`specification-remediation`).
//!
//! Covers the new `forge spec` command surface end to end through the
//! built binary: traceable generation with provenance (R1 success),
//! idempotent re-run on the same finding set (R1 boundary),
//! ambiguous/empty requests refused before any file change (R1
//! failure), deterministic routing for known lifecycle findings
//! (R2 success), semantic routing that produces a spec (R2 success
//! via the spec handoff), and manual routing recorded without
//! claiming an AI fix (R2 boundary). Existing contracts (doctor,
//! upgrade, feature) must still hold after the new commands run
//! against a generated project.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_DRIFTWATCH_BIN");
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

fn run_json(db: &Path, args: &[&str]) -> (serde_json::Value, Option<i32>) {
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

fn write_rust_l1(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let body = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), body).unwrap();
}

#[test]
fn spec_generate_writes_traceable_bounded_proposal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("trace-app");
    write_rust_l1(&proj, "trace-app");

    let out = run_json(
        &db,
        &[
            "spec",
            "generate",
            proj.to_str().unwrap(),
            "--finding",
            "f1",
            "--finding",
            "f2",
            "--reason",
            "doctor pass surfaced a semantic gap",
        ],
    );
    let value = &out.0;
    assert_eq!(out.1, Some(0), "spec generate must exit 0");
    assert_eq!(value["status"], "generated");
    let spec = &value["spec"];
    assert_eq!(spec["provenance"]["project_id"], "trace-app");
    assert_eq!(spec["provenance"]["profile"], "rust-web");
    let findings = spec["findings"].as_array().unwrap();
    let finding_strs: Vec<&str> = findings.iter().map(|f| f.as_str().unwrap()).collect();
    assert!(finding_strs.contains(&"f1"));
    assert!(finding_strs.contains(&"f2"));
    let acceptance = spec["acceptance"].as_array().unwrap();
    assert!(
        !acceptance.is_empty(),
        "spec must carry acceptance scenarios"
    );
    let files = value["files_written"].as_array().unwrap();
    let file_strs: Vec<&str> = files.iter().map(|f| f.as_str().unwrap()).collect();
    let spec_dir_name = spec["id"]["dir_name"]
        .as_str()
        .expect("spec id must be human-readable");
    let expected_proposal = format!(".forge/specs/{spec_dir_name}/proposal.md");
    let expected_manifest = format!(".forge/specs/{spec_dir_name}/manifest.json");
    assert!(
        file_strs.contains(&expected_proposal.as_str()),
        "files {file_strs:?} should contain {expected_proposal}"
    );
    assert!(
        file_strs.contains(&expected_manifest.as_str()),
        "files {file_strs:?} should contain {expected_manifest}"
    );
    let written = proj.join(".forge/specs");
    assert!(written.is_dir());
    let spec_dir_name = spec["id"]["dir_name"]
        .as_str()
        .expect("spec id must be human-readable");
    let proposal = fs::read_to_string(written.join(spec_dir_name).join("proposal.md")).unwrap();
    assert!(proposal.contains("# "), "proposal.md must have a title");
    assert!(proposal.contains("## Traceable provenance"));
    assert!(proposal.contains("`trace-app`"));
    let manifest_json =
        fs::read_to_string(written.join(spec_dir_name).join("manifest.json")).unwrap();
    let manifest_value: serde_json::Value = serde_json::from_str(&manifest_json).unwrap();
    assert_eq!(manifest_value["id"]["project_id"], "trace-app");
}

#[test]
fn spec_generate_is_idempotent_on_unchanged_finding_set() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("idem-app");
    write_rust_l1(&proj, "idem-app");

    let first = run_json(
        &db,
        &[
            "spec",
            "generate",
            proj.to_str().unwrap(),
            "--finding",
            "only",
        ],
    );
    assert_eq!(first.1, Some(0));
    assert_eq!(first.0["status"], "generated");
    let first_spec_id = first.0["spec"]["id"]["dir_name"]
        .as_str()
        .unwrap()
        .to_string();
    let first_mtime = fs::metadata(
        proj.join(".forge/specs")
            .join(&first_spec_id)
            .join("proposal.md"),
    )
    .unwrap()
    .modified()
    .unwrap();

    // Sleep so a re-write would visibly bump the mtime.
    std::thread::sleep(std::time::Duration::from_millis(50));

    let second = run_json(
        &db,
        &[
            "spec",
            "generate",
            proj.to_str().unwrap(),
            "--finding",
            "only",
        ],
    );
    assert_eq!(second.1, Some(0));
    assert_eq!(second.0["status"], "existing");
    let second_files = second.0["files_written"].as_array().unwrap();
    assert!(
        second_files.is_empty(),
        "no files should be rewritten on boundary re-run"
    );
    let second_mtime = fs::metadata(
        proj.join(".forge/specs")
            .join(&first_spec_id)
            .join("proposal.md"),
    )
    .unwrap()
    .modified()
    .unwrap();
    assert_eq!(
        first_mtime, second_mtime,
        "boundary re-run must leave the original spec untouched"
    );
    let note = second.0["note"].as_str().unwrap();
    assert!(note.contains("already exists"), "{note}");
}

#[test]
fn spec_generate_refuses_empty_or_oversized_finding_set() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("rej-app");
    write_rust_l1(&proj, "rej-app");

    let empty = run(&db, &["spec", "generate", proj.to_str().unwrap()]);
    assert_eq!(
        empty.status.code(),
        Some(1),
        "empty finding set must exit 1"
    );
    let stderr = lossy(&empty.stderr);
    assert!(stderr.contains("error[spec-invalid]"), "{stderr}");
    assert!(stderr.contains("at least one finding id"), "{stderr}");
    let before = project_files(&proj);
    let mut args: Vec<String> = vec!["spec".into(), "generate".into(), proj.display().to_string()];
    for i in 0..40 {
        args.push("--finding".into());
        args.push(format!("f{i}"));
    }
    let str_args: Vec<&str> = args.iter().map(String::as_str).collect();
    let oversized = run(&db, &str_args);
    assert_eq!(oversized.status.code(), Some(1));
    let stderr = lossy(&oversized.stderr);
    assert!(stderr.contains("error[spec-invalid]"), "{stderr}");
    assert!(stderr.contains("at most"), "{stderr}");
    assert_eq!(
        project_files(&proj),
        before,
        "refusal must not create any spec files"
    );
}

fn project_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out.sort();
    out
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if path.strip_prefix(root).unwrap().starts_with(".forge/specs") {
            out.push(path);
        }
    }
}

#[test]
fn spec_route_classifies_finding_into_deterministic_semantic_or_manual() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("route-app");
    write_rust_l1(&proj, "route-app");

    let det = run_json(
        &db,
        &["spec", "route", "dependency-drift", proj.to_str().unwrap()],
    );
    assert_eq!(det.0["route"], "deterministic");
    assert_eq!(det.0["action"], "forge upgrade");

    let man = run_json(
        &db,
        &["spec", "route", "manifest-valid", proj.to_str().unwrap()],
    );
    assert_eq!(man.0["route"], "manual");
    assert!(man.0["action"].is_null());
    assert!(man.0["suggested_spec"].is_null());

    let pol = run_json(
        &db,
        &[
            "spec",
            "route",
            "driftwatch-AUTH-001",
            proj.to_str().unwrap(),
        ],
    );
    assert_eq!(pol.0["route"], "semantic");
    assert_eq!(pol.0["suggested_spec"]["project_id"], "route-app");
}

#[test]
fn spec_apply_records_deterministic_action_and_emits_no_files() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("apply-det-app");
    write_rust_l1(&proj, "apply-det-app");

    let value = run_json(
        &db,
        &["spec", "apply", "dependency-drift", proj.to_str().unwrap()],
    );
    assert_eq!(value.0["apply"]["status"], "applied");
    assert_eq!(value.0["apply"]["decision"]["route"], "deterministic");
    let evidence = value.0["apply"]["evidence"].as_array().unwrap();
    assert!(!evidence.is_empty());
    assert!(value.0["apply"]["files_changed"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!proj.join(".forge/specs").exists());
}

#[test]
fn spec_apply_records_manual_route_without_claiming_ai_fix() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("apply-manual-app");
    write_rust_l1(&proj, "apply-manual-app");

    let value = run_json(
        &db,
        &["spec", "apply", "manifest-valid", proj.to_str().unwrap()],
    );
    assert_eq!(value.0["apply"]["status"], "manual");
    assert_eq!(value.0["apply"]["decision"]["route"], "manual");
    let note = value.0["apply"]["note"].as_str().unwrap();
    assert!(note.contains("manual boundary"), "{note}");
    assert!(value.0["apply"]["files_changed"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!proj.join(".forge/specs").exists());
}

#[test]
fn spec_apply_semantic_route_generates_a_bounded_proposal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("apply-sem-app");
    write_rust_l1(&proj, "apply-sem-app");

    let value = run_json(
        &db,
        &[
            "spec",
            "apply",
            "driftwatch-AUTH-001",
            proj.to_str().unwrap(),
            "--reason",
            "policy finding needs a bounded change",
        ],
    );
    assert_eq!(value.0["apply"]["status"], "spec-generated");
    assert_eq!(value.0["apply"]["decision"]["route"], "semantic");
    let files = value.0["apply"]["files_changed"].as_array().unwrap();
    assert!(!files.is_empty());
    let spec_dir = proj.join(".forge/specs");
    assert!(spec_dir.is_dir());
    let entries: Vec<PathBuf> = fs::read_dir(&spec_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(entries.len(), 1);
    let manifest = fs::read_to_string(entries[0].join("manifest.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(parsed["provenance"]["project_id"], "apply-sem-app");
    assert!(parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f.as_str() == Some("driftwatch-AUTH-001")));
}

#[test]
fn spec_list_and_inspect_round_trip_a_generated_spec() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("round-app");
    write_rust_l1(&proj, "round-app");

    let _ = run_json(
        &db,
        &[
            "spec",
            "generate",
            proj.to_str().unwrap(),
            "--finding",
            "f1",
            "--finding",
            "f2",
        ],
    );
    let list = run_json(&db, &["spec", "list", proj.to_str().unwrap()]);
    let specs = list.0["specs"].as_array().unwrap();
    assert_eq!(specs.len(), 1);
    let dir_name = specs[0]["id"]["dir_name"].as_str().unwrap().to_string();
    let inspect = run_json(&db, &["spec", "inspect", &dir_name, proj.to_str().unwrap()]);
    let draft = &inspect.0["spec"];
    assert_eq!(draft["id"]["dir_name"], dir_name);
    assert_eq!(draft["provenance"]["project_id"], "round-app");
    let proposal_md = inspect.0["proposal"].as_str().unwrap();
    assert!(proposal_md.contains("## Traceable provenance"));
}

#[test]
fn spec_operations_do_not_break_existing_doctor_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("regress-app");
    write_rust_l1(&proj, "regress-app");
    fs::write(proj.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
    fs::write(proj.join("README.md"), "# regress\n").unwrap();

    // Two semantic specs: the doctor verdict must stay the same as
    // before the spec operations ran.
    let doctor_before = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    let _ = run_json(
        &db,
        &[
            "spec",
            "apply",
            "driftwatch-AUTH-001",
            proj.to_str().unwrap(),
        ],
    );
    let _ = run_json(
        &db,
        &["spec", "apply", "semantic-auth", proj.to_str().unwrap()],
    );
    let doctor_after = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    let findings_before: Vec<String> = doctor_before.0["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_string())
        .collect();
    let findings_after: Vec<String> = doctor_after.0["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        findings_before, findings_after,
        "doctor finding inventory must be unchanged by spec operations"
    );
    assert_eq!(
        doctor_after.0["doctor"]["healthy"], false,
        "regress-app stays not healthy: deployment/ci/driftwatch missing"
    );
}
