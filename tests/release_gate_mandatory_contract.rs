//! Contract tests for the mandatory Gate evidence on a release
//! (`release-gate-mandatory`).
//!
//! The Gate check always applies to a release, the release lane asks
//! the shared delivery rule for its verdict, and the delivery lane
//! refuses for the same reason on the same evidence. Each test pins
//! one observable behaviour:
//!
//! - the plan always carries an applicable Gate check, even when the
//!   manifest declares no check at all;
//! - a manifest that declares `kind: gate` is refused;
//! - the release reports the shared rule's own reason, unchanged;
//! - the release lane and the delivery lane refuse for the same
//!   reason on the same evidence;
//! - the apply step re-judges the evidence at the applied revision
//!   and refuses before any stage runs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::core::manifest::Manifest;
use forge::core::ForgeError;
use forge::delivery::handlers::run_preflight;
use forge::gate::contract::GATE_CONTRACT_VERSION;
use forge::policy::DriftWatchConfig;
use forge::registry::Registry;
use forge::release::engine::apply_release;
use forge::release::engine::model::PlanReport;
use forge::release::engine::prepare_release;
use forge::release::model::{CapturedCheck, ReleaseAdapterConfig, ReleaseRequest, Semver};
use forge::release::release_config_from_manifest;
use tempfile::tempdir;

/// The `release.checks` block of a manifest that declares the two
/// extra checks.
const DECLARED_CHECKS: &str = "  checks:\n    - kind: doctor\n    - kind: test\n";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stderr={}",
        args,
        out.status,
        lossy(&out.stderr)
    );
}

/// The trimmed stdout of one git invocation.
fn git_output(dir: &Path, args: &[&str]) -> String {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stderr={}",
        args,
        out.status,
        lossy(&out.stderr)
    );
    lossy(&out.stdout).trim().to_string()
}

/// A project the release engine accepts: a schema-1 manifest with a
/// `release:` section, a present changelog, a minimal Cargo
/// workspace so the doctor and test checks pass, and a git working
/// tree with a committed HEAD.
fn write_project(dir: &Path, id: &str, checks_yaml: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n{checks_yaml}  changelog: CHANGELOG.md\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"rel-gate-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();
    fs::write(dir.join("lib.rs"), "//! rel-gate-fixture.\n").unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(
        dir,
        &[
            "add",
            "--",
            "forge.yaml",
            "README.md",
            "CHANGELOG.md",
            "Cargo.toml",
            "lib.rs",
        ],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

/// Write one Gate evidence record under the project's evidence
/// directory. The record is written after the fixture's commit, so
/// the revision it names is the one a release captures.
fn write_evidence(dir: &Path, project_id: &str, record: serde_json::Value) {
    let path = dir
        .join(".forge/gate")
        .join(project_id)
        .join("evidence.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
}

/// A record that satisfies every condition of the shared delivery
/// rule.
fn passing_record(project_id: &str, revision: &str) -> serde_json::Value {
    serde_json::json!({
        "contract": GATE_CONTRACT_VERSION,
        "project_id": project_id,
        "runtime": "forge",
        "profile": "full",
        "revision": revision,
        "aggregate": "passed",
        "checks": [],
        "observed_at": "2026-10-21T00:00:00Z",
        "dry_run": false
    })
}

/// The Gate check row of a prepared plan.
fn gate_row(plan: &PlanReport) -> &CapturedCheck {
    plan.checks
        .iter()
        .find(|c| c.kind == "gate")
        .expect("gate check")
}

/// Prepare a release plan for `dir`.
fn prepare(dir: &Path) -> PlanReport {
    let (manifest, _) = Manifest::load_from_dir(dir, None).expect("manifest");
    let config = release_config_from_manifest(&manifest).expect("release config");
    let request = ReleaseRequest {
        project_id: manifest.project.id.clone(),
        version: Semver::parse("1.0.0").expect("semver"),
        confirm: false,
        dry_run: true,
        retry: false,
        stages: config.stages.clone(),
    };
    let policy = DriftWatchConfig::from_env();
    prepare_release(dir, &manifest, &config, &request, &policy).expect("plan")
}

/// The reason the delivery lane refuses with, taken from the shared
/// rule's own verdict.
fn delivery_refusal(dir: &Path, db: &Path, project_id: &str) -> String {
    let mut registry = Registry::open(db).expect("registry");
    registry.register(dir, None).expect("register");
    let err = run_preflight(&registry, project_id, chrono::Utc::now())
        .expect_err("the delivery lane must refuse");
    match err {
        ForgeError::DeliveryConflict { reason } => reason,
        other => panic!("expected a delivery conflict, got: {other}"),
    }
}

#[test]
fn the_gate_check_is_always_applicable() {
    let tmp = tempdir().unwrap();
    let proj = tmp.path().join("proj");
    // The manifest declares no check at all and the project has no
    // Gate evidence: the Gate check is still captured, and it is
    // applicable.
    write_project(&proj, "rel-gate-always", "");
    let plan = prepare(&proj);
    let gate = gate_row(&plan);
    assert!(gate.applicable, "the Gate check always applies");
    assert_eq!(gate.status, "fail");
    assert!(!plan.ready, "plan must not be ready: {}", plan.note);
    // The same manifest is ready once the Gate evidence satisfies
    // the shared rule.
    let revision = git_output(&proj, &["rev-parse", "HEAD"]);
    write_evidence(
        &proj,
        "rel-gate-always",
        passing_record("rel-gate-always", &revision),
    );
    let plan = prepare(&proj);
    let gate = gate_row(&plan);
    assert!(gate.applicable, "the Gate check always applies");
    assert_eq!(gate.status, "pass");
    assert!(plan.ready, "plan must be ready: {}", plan.note);
}

#[test]
fn a_manifest_that_declares_the_gate_check_is_refused() {
    let tmp = tempdir().unwrap();
    let proj = tmp.path().join("proj");
    write_project(&proj, "rel-gate-declared", "  checks:\n    - kind: gate\n");
    let (manifest, _) = Manifest::load_from_dir(&proj, None).expect("manifest");
    let err = release_config_from_manifest(&manifest).expect_err("the manifest must be refused");
    assert_eq!(err.code(), "release-invalid");
    assert!(
        err.to_string().contains(
            "release.checks kind `gate` is not configurable; the Gate check always applies to a release"
        ),
        "the refusal must name the truth: {err}"
    );
    // The surface refuses the same way.
    let out = Command::new(forge_bin())
        .arg("--registry")
        .arg(tmp.path().join("registry.db"))
        .arg("release")
        .arg("prepare")
        .arg(&proj)
        .arg("--version")
        .arg("1.0.0")
        .output()
        .expect("run forge");
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("release-invalid"),
        "stderr must report release-invalid: {stderr}"
    );
    assert!(
        stderr.contains("the Gate check always applies to a release"),
        "stderr must name the truth: {stderr}"
    );
}

#[test]
fn the_release_refuses_with_the_shared_rule_reason() {
    let tmp = tempdir().unwrap();
    let proj = tmp.path().join("proj");
    write_project(&proj, "rel-gate-absent", DECLARED_CHECKS);
    // The project has no Gate evidence at all.
    let plan = prepare(&proj);
    let gate = gate_row(&plan);
    assert_eq!(gate.status, "fail");
    assert_eq!(
        gate.detail,
        "Forge Gate evidence is absent for `rel-gate-absent`; run `forge gate --profile full` from the project directory and retry"
    );
    assert!(!plan.ready, "plan must not be ready: {}", plan.note);
}

/// One evidence case: a label and the record to write for the
/// project, or `None` for the absent case.
type EvidenceCase = (
    &'static str,
    Box<dyn Fn(&str, &str) -> Option<serde_json::Value>>,
);

#[test]
fn the_release_and_the_delivery_lane_refuse_for_the_same_reason() {
    // Every case the shared rule owns: the evidence is absent, it
    // belongs to another revision, it is a fast-profile run, it is a
    // rehearsal, or it belongs to another project.
    let cases: Vec<EvidenceCase> = vec![
        ("absent", Box::new(|_: &str, _: &str| None)),
        (
            "stale",
            Box::new(|project_id: &str, _: &str| {
                Some(passing_record(
                    project_id,
                    "fedcba9876543210fedcba9876543210fedcba98",
                ))
            }),
        ),
        (
            "fast profile",
            Box::new(|project_id: &str, revision: &str| {
                let mut record = passing_record(project_id, revision);
                record["profile"] = serde_json::json!("fast");
                Some(record)
            }),
        ),
        (
            "rehearsal",
            Box::new(|project_id: &str, revision: &str| {
                let mut record = passing_record(project_id, revision);
                record["dry_run"] = serde_json::json!(true);
                Some(record)
            }),
        ),
        (
            "another project",
            Box::new(|_: &str, revision: &str| Some(passing_record("other", revision))),
        ),
    ];
    for (case, record) in cases {
        let tmp = tempdir().unwrap();
        let proj = tmp.path().join("proj");
        let db = tmp.path().join("registry.db");
        let project_id = "rel-gate-shared";
        write_project(&proj, project_id, "");
        let revision = git_output(&proj, &["rev-parse", "HEAD"]);
        if let Some(record) = record(project_id, &revision) {
            // Another project's record lives under its own id.
            let owner = if case == "another project" {
                "other"
            } else {
                project_id
            };
            write_evidence(&proj, owner, record);
        }
        // The release lane reports the shared rule's reason.
        let plan = prepare(&proj);
        let gate = gate_row(&plan);
        assert_eq!(gate.status, "fail", "case {case}");
        assert!(!plan.ready, "case {case}: {}", plan.note);
        // The delivery lane refuses for the same reason.
        let delivery_reason = delivery_refusal(&proj, &db, project_id);
        assert_eq!(
            gate.detail, delivery_reason,
            "case {case}: the two lanes must refuse for the same reason"
        );
    }
}

#[test]
fn the_apply_step_refuses_a_stale_gate_evidence_before_any_stage() {
    let tmp = tempdir().unwrap();
    let proj = tmp.path().join("proj");
    write_project(&proj, "rel-gate-stale", DECLARED_CHECKS);
    let revision = git_output(&proj, &["rev-parse", "HEAD"]);
    write_evidence(
        &proj,
        "rel-gate-stale",
        passing_record("rel-gate-stale", &revision),
    );
    // Move HEAD past the revision the evidence is bound to.
    fs::write(proj.join("README.md"), "v2\n").unwrap();
    run_git(&proj, &["add", "--", "README.md"]);
    run_git(&proj, &["commit", "-q", "-m", "second"]);
    let head = git_output(&proj, &["rev-parse", "HEAD"]);

    let (manifest, _) = Manifest::load_from_dir(&proj, None).expect("manifest");
    let config = release_config_from_manifest(&manifest).expect("release config");
    let request = ReleaseRequest {
        project_id: manifest.project.id.clone(),
        version: Semver::parse("1.0.0").expect("semver"),
        confirm: true,
        dry_run: false,
        retry: false,
        stages: config.stages.clone(),
    };
    let adapters = ReleaseAdapterConfig::from_env();
    let err = apply_release(&proj, &manifest, &config, &request, &adapters)
        .expect_err("the apply step must refuse");
    assert_eq!(err.code(), "release-check-failed");
    assert!(
        err.to_string().contains("gate:fail"),
        "the refusal must name the failing Gate check: {err}"
    );
    // The refusal happens before any stage runs: no tag, no commit.
    let tags = Command::new("git")
        .arg("-C")
        .arg(&proj)
        .arg("tag")
        .arg("--list")
        .output()
        .expect("git tag --list");
    assert!(
        lossy(&tags.stdout).trim().is_empty(),
        "no tag may be created: {}",
        lossy(&tags.stdout)
    );
    assert_eq!(
        git_output(&proj, &["rev-parse", "HEAD"]),
        head,
        "no stage may commit"
    );
}
