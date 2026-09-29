use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(target: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("FORGE_INVENTORY_SOURCE")
        .env(
            "FORGE_REGISTRY",
            target.parent().unwrap().join("registry.db"),
        );
    cmd.args(args).arg("--target").arg(target);
    cmd.output().expect("run forge")
}

fn project(tmp: &Path) -> PathBuf {
    let target = tmp.join("demo");
    fs::create_dir_all(target.join(".standard")).unwrap();
    fs::write(
        target.join("forge.yaml"),
        "schema: 1\nproject:\n  id: demo\n  name: Demo\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    target
}

#[test]
fn help_advertises_the_four_remediation_verbs() {
    let out = Command::new(forge_bin())
        .args(["remediate", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for verb in ["scan", "plan", "diff", "apply"] {
        assert!(text.contains(verb), "missing {verb}: {text}");
    }
}

#[test]
fn scan_reports_missing_ci_without_a_pack_or_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let out = run(&target, &["remediate", "scan", "--format", "json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["findings"][0]["finding_id"], "gaps.ci.demo.ci");
    assert_eq!(value["findings"][0]["class"], "automatic");
    assert!(!target.join(".standard/ci/verify.yml").exists());
    assert!(!tmp.path().join("registry.db").exists());
}

#[test]
fn plan_for_automatic_ci_finding_is_preview_only() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let before = fs::read_dir(&target).unwrap().count();
    let out = run(
        &target,
        &[
            "remediate",
            "plan",
            "--finding",
            "gaps.ci.demo.ci",
            "--pack",
            "baseline-service@1.1.0",
            "--format",
            "json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["contract"], "forge-remediation-plan/0.1.0");
    assert_eq!(
        value["plan"]["actions"][0]["kind"],
        "install_standard_asset"
    );
    assert_eq!(fs::read_dir(&target).unwrap().count(), before);
    assert!(!target.join(".standard/ci/verify.yml").exists());
}

#[test]
fn semantic_finding_is_refused_without_a_plan() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let out = run(
        &target,
        &[
            "remediate",
            "plan",
            "--finding",
            "gaps.description.demo.name",
            "--format",
            "json",
        ],
    );
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("remediation-invalid"));
    assert!(!target.join(".standard/ci/verify.yml").exists());
}

#[test]
fn finding_for_another_project_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let out = run(
        &target,
        &[
            "remediate",
            "plan",
            "--finding",
            "gaps.ci.other.ci",
            "--pack",
            "baseline-service@1.1.0",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("remediation-invalid"));
    assert!(!target.join(".standard/ci/verify.yml").exists());
}

#[test]
fn missing_pack_and_incompatible_profile_are_typed_refusals() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let missing = run(
        &target,
        &[
            "remediate",
            "plan",
            "--finding",
            "gaps.ci.demo.ci",
            "--pack",
            "baseline-service@9.9.9",
        ],
    );
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("remediation-invalid"));

    fs::write(
        target.join("forge.yaml"),
        "schema: 1\nproject:\n  id: demo\n  name: Demo\n  profile: flutter-app\n  maturity: L1\n",
    )
    .unwrap();
    let incompatible = run(
        &target,
        &[
            "remediate",
            "plan",
            "--finding",
            "gaps.ci.demo.ci",
            "--pack",
            "baseline-service@1.1.0",
        ],
    );
    assert!(!incompatible.status.success());
    assert!(String::from_utf8_lossy(&incompatible.stderr).contains("remediation-invalid"));
}

#[test]
fn saved_plan_refuses_changed_file_preconditions() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let preview = run(
        &target,
        &[
            "remediate",
            "plan",
            "--finding",
            "gaps.ci.demo.ci",
            "--pack",
            "baseline-service@1.1.0",
            "--format",
            "json",
        ],
    );
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let value: Value = serde_json::from_slice(&preview.stdout).unwrap();
    let plan_path = tmp.path().join("plan.json");
    fs::write(&plan_path, serde_json::to_vec(&value["plan"]).unwrap()).unwrap();
    fs::create_dir_all(target.join(".standard/ci")).unwrap();
    fs::write(
        target.join(".standard/ci/verify.yml"),
        "changed after planning\n",
    )
    .unwrap();
    let plan_arg = plan_path.to_str().unwrap();
    let out = run(
        &target,
        &["remediate", "apply", "--plan", plan_arg, "--confirm"],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("remediation-conflict"));
    assert_eq!(
        fs::read(target.join(".standard/ci/verify.yml")).unwrap(),
        b"changed after planning\n"
    );
}

#[test]
fn saved_plan_refuses_changed_git_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    for args in [
        vec!["init", target.to_str().unwrap()],
        vec![
            "-C",
            target.to_str().unwrap(),
            "config",
            "user.name",
            "Test",
        ],
        vec![
            "-C",
            target.to_str().unwrap(),
            "config",
            "user.email",
            "test@example.invalid",
        ],
        vec!["-C", target.to_str().unwrap(), "add", "forge.yaml"],
        vec!["-C", target.to_str().unwrap(), "commit", "-m", "initial"],
    ] {
        let output = Command::new("git").args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let preview = run(
        &target,
        &[
            "remediate",
            "plan",
            "--finding",
            "gaps.ci.demo.ci",
            "--pack",
            "baseline-service@1.1.0",
            "--format",
            "json",
        ],
    );
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let value: Value = serde_json::from_slice(&preview.stdout).unwrap();
    let plan_path = tmp.path().join("plan-revision.json");
    fs::write(&plan_path, serde_json::to_vec(&value["plan"]).unwrap()).unwrap();
    let changed = Command::new("git")
        .args([
            "-C",
            target.to_str().unwrap(),
            "commit",
            "--allow-empty",
            "-m",
            "revision changed",
        ])
        .output()
        .unwrap();
    assert!(
        changed.status.success(),
        "{}",
        String::from_utf8_lossy(&changed.stderr)
    );
    let out = run(
        &target,
        &[
            "remediate",
            "apply",
            "--plan",
            plan_path.to_str().unwrap(),
            "--confirm",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("remediation-conflict"));
    assert!(!target.join(".standard/ci/verify.yml").exists());
}

#[test]
fn apply_requires_confirmation_and_then_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let args = [
        "remediate",
        "apply",
        "--finding",
        "gaps.ci.demo.ci",
        "--pack",
        "baseline-service@1.1.0",
    ];
    let refused = run(&target, &args);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--confirm"));
    let applied = run(
        &target,
        &[
            &args[0],
            &args[1],
            &args[2],
            &args[3],
            &args[4],
            &args[5],
            "--confirm",
            "--format",
            "json",
        ],
    );
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(target.join(".standard/ci/verify.yml").is_file());
    let again = run(
        &target,
        &[
            &args[0],
            &args[1],
            &args[2],
            &args[3],
            &args[4],
            &args[5],
            "--confirm",
            "--format",
            "json",
        ],
    );
    assert!(
        again.status.success(),
        "{}",
        String::from_utf8_lossy(&again.stderr)
    );
    let value: Value = serde_json::from_slice(&again.stdout).unwrap();
    assert_eq!(value["outcome"]["status"], "already_applied");
}
