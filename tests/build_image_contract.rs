//! CLI contract tests for `forge build --image`.
//!
//! A recording adapter speaks `forge-image-builder/0.1.0` and answers in one
//! of the ways the contract allows. Each test asserts the observable
//! outcome: a record with Forge's own SBOM hash, or a typed refusal that
//! writes nothing.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_IMAGE_BUILDER_BIN")
        .env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    cmd
}

fn run(args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run forge")
}

fn run_json(args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--format").arg("json");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge json");
    assert!(
        out.status.success(),
        "forge failed: status={} stderr={}",
        out.status,
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn write_project(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L2\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "v1\n").unwrap();
    for args in [
        vec!["init", "-q", "--initial-branch=main"],
        vec!["add", "."],
        vec!["commit", "-q", "-m", "seed"],
    ] {
        let out = Command::new("git")
            .args([
                "-c",
                "user.name=Forge",
                "-c",
                "user.email=forge@example.com",
            ])
            .args(&args)
            .current_dir(dir)
            .output()
            .expect("git");
        assert!(out.status.success(), "git {args:?}: {}", lossy(&out.stderr));
    }
}

fn revision(dir: &Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(out.status.success());
    lossy(&out.stdout).trim().to_string()
}

/// Install the recording adapter and return its path. Its behaviour is
/// selected by writing `mode` beside it.
fn install_adapter(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("stub-image-builder");
    let body = r#"#!/usr/bin/env python3
import json, os, sys
request = json.load(sys.stdin)
here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "mode")) as handle:
    mode = handle.read().strip()
tag = request["tag"]
response = {
    "contract": "forge-image-builder/0.1.0",
    "builder": "stub",
    "builder_version": "0.1.0",
    "status": "built",
    "image": tag,
    "digest": "sha256:" + "a" * 64,
    "evidence": ["stub built the image"],
}
if mode == "built":
    sbom = os.path.join(request["directory"], "sbom.json")
    with open(sbom, "w") as handle:
        handle.write('{"spdxVersion":"SPDX-2.3"}')
    response["sbom_path"] = "sbom.json"
elif mode == "no_sbom":
    pass
elif mode == "failed":
    response["status"] = "failed"
    response["evidence"] = ["compiler exploded"]
    response.pop("image", None)
    response.pop("digest", None)
elif mode == "wrong_image":
    response["image"] = tag + "-other"
elif mode == "bad_digest":
    response["digest"] = "sha1:abc"
elif mode == "escape":
    response["sbom_path"] = "../outside.json"
elif mode == "missing_sbom":
    response["sbom_path"] = "nope.json"
elif mode == "bad_contract":
    response["contract"] = "forge-image-builder/1.0.0"
elif mode == "newer_minor":
    response["contract"] = "forge-image-builder/0.2.0"
elif mode == "exit":
    sys.exit(3)
json.dump(response, sys.stdout)
sys.stdout.write("\n")
"#;
    std::fs::write(&path, body).unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    std::fs::write(dir.join("mode"), "built").unwrap();
    path
}

fn set_mode(adapter: &Path, mode: &str) {
    std::fs::write(adapter.parent().unwrap().join("mode"), mode).unwrap();
}

fn record_path(project: &Path, id: &str) -> PathBuf {
    let rev = revision(project);
    project
        .join(".forge/build")
        .join(id)
        .join(format!("{}.json", &rev[..12]))
}

#[test]
fn a_built_image_writes_a_revision_bound_record() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-app");
    let adapter = install_adapter(&tmp.path().join("adapter"));

    let json = run_json(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--builder-bin",
        adapter.to_str().unwrap(),
    ]);
    let build = &json["image_build"];
    let rev = revision(&project);
    assert_eq!(build["revision"], rev);
    assert_eq!(build["tag"], format!("forge-image-app:{}", &rev[..12]));
    assert_eq!(build["digest"], format!("sha256:{}", "a".repeat(64)));
    assert_eq!(build["sbom"]["path"], "sbom.json");
    assert_eq!(build["sbom"]["sha256"].as_str().unwrap().len(), 64);

    let path = record_path(&project, "image-app");
    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(
        !raw.contains(project.to_str().unwrap()),
        "the record must not carry an absolute path: {raw}"
    );
    let record: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(record["contract"], "forge-build-image/0.1.0");
    assert_eq!(record["project_id"], "image-app");
    assert_eq!(record["revision"], rev);
    assert_eq!(record["digest"], build["digest"]);
    assert_eq!(record["sbom"]["path"], "sbom.json");
}

#[test]
fn a_build_without_an_sbom_records_none() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-nosbom");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    set_mode(&adapter, "no_sbom");

    let json = run_json(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--builder-bin",
        adapter.to_str().unwrap(),
    ]);
    assert!(json["image_build"]["sbom"].is_null());
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(record_path(&project, "image-nosbom")).unwrap(),
    )
    .unwrap();
    assert!(record.get("sbom").is_none());
}

#[test]
fn a_newer_contract_minor_is_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-newer");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    set_mode(&adapter, "newer_minor");

    let json = run_json(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--builder-bin",
        adapter.to_str().unwrap(),
    ]);
    assert!(json["image_build"]["digest"].is_string());
}

#[test]
fn a_builder_failure_is_typed_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-failed");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    set_mode(&adapter, "failed");

    let out = run(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--builder-bin",
        adapter.to_str().unwrap(),
    ]);
    assert!(!out.status.success(), "a failed build must exit non-zero");
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("build-failed"), "{stderr}");
    assert!(stderr.contains("compiler exploded"), "{stderr}");
    assert!(!record_path(&project, "image-failed").exists());
}

#[test]
fn an_adapter_that_exits_non_zero_is_a_build_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-exit");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    set_mode(&adapter, "exit");

    let out = run(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--builder-bin",
        adapter.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(lossy(&out.stderr).contains("build-failed"));
    assert!(!record_path(&project, "image-exit").exists());
}

#[test]
fn a_claim_forge_cannot_check_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-invalid");
    let adapter = install_adapter(&tmp.path().join("adapter"));

    for (mode, needle) in [
        ("wrong_image", "was requested"),
        ("bad_digest", "sha256"),
        ("bad_contract", "major versions differ"),
        ("escape", "component other than a plain name"),
        ("missing_sbom", "does not name a readable file"),
    ] {
        set_mode(&adapter, mode);
        let out = run(&[
            "build",
            project.to_str().unwrap(),
            "--image",
            "--builder-bin",
            adapter.to_str().unwrap(),
        ]);
        assert!(!out.status.success(), "mode {mode} must be refused");
        let stderr = lossy(&out.stderr);
        assert!(stderr.contains("build-invalid"), "{mode}: {stderr}");
        assert!(stderr.contains(needle), "{mode}: {stderr}");
        assert!(
            !record_path(&project, "image-invalid").exists(),
            "mode {mode} wrote a record"
        );
    }
}

#[test]
fn a_missing_builder_names_the_variable_and_never_installs_one() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-missing");
    let missing = tmp.path().join("no-such-builder");

    let out = run(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--builder-bin",
        missing.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("build-invalid"), "{stderr}");
    assert!(stderr.contains("FORGE_IMAGE_BUILDER_BIN"), "{stderr}");
    assert!(stderr.contains("never installs"), "{stderr}");
    assert!(!record_path(&project, "image-missing").exists());
}

#[test]
fn a_project_without_a_commit_has_no_image() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("forge.yaml"),
        "schema: 1\nproject:\n  id: image-nogit\n  name: image-nogit\n  profile: rust-web\n  maturity: L2\nruntime:\n  language: rust\n",
    )
    .unwrap();
    std::fs::write(project.join("README.md"), "v1\n").unwrap();
    let adapter = install_adapter(&tmp.path().join("adapter"));

    let out = run(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--builder-bin",
        adapter.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("build-invalid"), "{stderr}");
    assert!(stderr.contains("no Git HEAD"), "{stderr}");
}

#[test]
fn image_flags_are_refused_without_the_image_path() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-flags");

    let out = run(&["build", project.to_str().unwrap(), "--tag", "app:test"]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("build-invalid"), "{stderr}");
    assert!(stderr.contains("pass `--image`"), "{stderr}");
}

#[test]
fn an_image_platform_is_not_selected_by_target_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-platform");
    let adapter = install_adapter(&tmp.path().join("adapter"));

    let out = run(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--os",
        "linux",
        "--arch",
        "x86_64",
        "--builder-bin",
        adapter.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("build-invalid"), "{stderr}");
    assert!(stderr.contains("platform"), "{stderr}");
}

fn build_once(project: &Path, adapter: &Path) {
    let out = run(&[
        "build",
        project.to_str().unwrap(),
        "--image",
        "--builder-bin",
        adapter.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "build: {}", lossy(&out.stderr));
}

#[test]
fn a_fresh_record_verifies() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-verify");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    build_once(&project, &adapter);

    let json = run_json(&["build", project.to_str().unwrap(), "--verify-image"]);
    assert_eq!(json["verification"]["verified"], true);
    assert_eq!(json["verification"]["revision"], revision(&project));
    assert_eq!(
        json["image_build"]["tag"],
        format!("forge-image-verify:{}", &revision(&project)[..12])
    );
    assert_eq!(
        json["image_build"]["digest"],
        format!("sha256:{}", "a".repeat(64))
    );
}

#[test]
fn verification_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-readonly");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    build_once(&project, &adapter);
    let record = record_path(&project, "image-readonly");
    let before_record = std::fs::read(&record).unwrap();
    let before_files = file_names(&project);

    let out = run(&["build", project.to_str().unwrap(), "--verify-image"]);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    assert_eq!(std::fs::read(&record).unwrap(), before_record);
    assert_eq!(
        file_names(&project),
        before_files,
        "verification wrote a file"
    );
}

fn file_names(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            names.push(path.strip_prefix(root).unwrap().display().to_string());
        }
    }
    names.sort();
    names
}

#[test]
fn a_moved_head_is_not_verified() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-stale");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    build_once(&project, &adapter);
    std::fs::write(project.join("README.md"), "v2\n").unwrap();
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "commit",
            "-q",
            "-am",
            "next",
        ])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(out.status.success());

    let out = run(&["build", project.to_str().unwrap(), "--verify-image"]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("build-invalid"), "{stderr}");
    assert!(
        stderr.contains("no readable image build record"),
        "{stderr}"
    );
    assert!(stderr.contains("forge build --image"), "{stderr}");
}

#[test]
fn an_edited_sbom_invalidates_the_record() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-sbom-edited");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    build_once(&project, &adapter);
    std::fs::write(
        project.join("sbom.json"),
        b"{\"spdxVersion\":\"SPDX-2.3\",\"edited\":true}",
    )
    .unwrap();

    let out = run(&["build", project.to_str().unwrap(), "--verify-image"]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("build-failed"), "{stderr}");
    assert!(
        stderr.contains("no longer matches the recorded build"),
        "{stderr}"
    );
}

#[test]
fn a_tampered_record_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-tampered");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    build_once(&project, &adapter);
    let record = record_path(&project, "image-tampered");

    let mut value: Value = serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
    value["digest"] = serde_json::json!("sha1:abc");
    std::fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
    let out = run(&["build", project.to_str().unwrap(), "--verify-image"]);
    assert!(!out.status.success());
    assert!(lossy(&out.stderr).contains("invalid digest"));

    value["digest"] = serde_json::json!(format!("sha256:{}", "a".repeat(64)));
    value["project_id"] = serde_json::json!("another-project");
    std::fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
    let out = run(&["build", project.to_str().unwrap(), "--verify-image"]);
    assert!(!out.status.success());
    assert!(lossy(&out.stderr).contains("names project `another-project`"));
}

#[test]
fn a_missing_record_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-no-record");

    let out = run(&["build", project.to_str().unwrap(), "--verify-image"]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("build-invalid"), "{stderr}");
    assert!(
        stderr.contains(".forge/build/image-no-record/"),
        "refusal names the record: {stderr}"
    );
}

#[test]
fn verification_is_exclusive_with_build_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write_project(&project, "image-verify-flags");
    let adapter = install_adapter(&tmp.path().join("adapter"));
    let adapter_path = adapter.to_str().unwrap().to_string();

    for args in [
        vec!["--verify-image", "--image"],
        vec!["--verify-image", "--tag", "app:test"],
        vec!["--verify-image", "--builder-bin", adapter_path.as_str()],
        vec!["--verify-image", "--os", "linux", "--arch", "x86_64"],
    ] {
        let mut full = vec!["build", project.to_str().unwrap()];
        full.extend(args.iter().copied());
        let out = run(&full);
        assert!(!out.status.success(), "{args:?} must be refused");
        let stderr = lossy(&out.stderr);
        assert!(stderr.contains("build-invalid"), "{args:?}: {stderr}");
        assert!(stderr.contains("--verify-image"), "{args:?}: {stderr}");
    }
}
