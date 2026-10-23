//! Release DriftWatch check truth contract (`release-driftwatch-check-truth`).
//!
//! The release lane judges a policy report with the doctor's own
//! verdict. This contract drives the real release plan through the
//! public `forge::release::engine::prepare_release` with a fake
//! policy adapter and holds both surfaces to the five verdicts:
//!
//! - an applicable rule failed -> the check fails and the plan is not
//!   ready;
//! - no rule applied -> the check is unavailable and the plan is not
//!   ready;
//! - a report produced against an older source revision -> the check
//!   is stale and the plan is not ready;
//! - an applicable rule warned -> the check passes, its evidence names
//!   the warning and never says healthy;
//! - every applicable rule passed -> the check passes and the
//!   evidence names the report healthy.
//!
//! The doctor is driven on the same five reports, so the release lane
//! and the doctor are pinned to one verdict about one report.
//!
//! Env overrides are process-scoped, so the tests in this file
//! serialize on a mutex.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use forge::core::manifest::{Manifest, Maturity};
use forge::doctor::freshness::observation_is_stale_from_report;
use forge::doctor::{run_doctor, FindingStatus, PolicyVerdict};
use forge::policy::{run_driftwatch, DriftWatchConfig, PolicyOutcome};
use forge::release::engine::{prepare_release, PlanReport};
use forge::release::model::{CapturedCheck, ReleaseConfig, ReleaseRequest, Semver};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// What the captured check's evidence has to name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Evidence {
    /// The ids of the rules that failed.
    Failing,
    /// That no rule applied, so there is no policy evidence.
    Unjudged,
    /// That the report belongs to an older source revision.
    Stale,
    /// The ids of the rules that warned.
    Warning,
    /// That every applicable rule passed.
    Healthy,
}

/// One policy report the fake adapter prints, with the verdict both
/// surfaces must reach for it.
struct Case {
    /// The rule id every report carries.
    id: &'static str,
    /// The severity the report carries.
    severity: &'static str,
    /// Whether the rule applied to the profile.
    applicable: bool,
    /// The source revision the report was produced against.
    source_revision: Option<&'static str>,
    /// The doctor's `driftwatch-policy` finding status.
    doctor_status: FindingStatus,
    /// The release check status.
    release_status: &'static str,
    /// Whether the check blocks the release plan.
    blocks: bool,
    /// What the captured evidence has to name.
    evidence: Evidence,
}

const CASES: [Case; 5] = [
    Case {
        id: "POLICY-DOCS-001",
        severity: "fail",
        applicable: true,
        source_revision: None,
        doctor_status: FindingStatus::Fail,
        release_status: "fail",
        blocks: true,
        evidence: Evidence::Failing,
    },
    Case {
        id: "POLICY-DOCS-001",
        severity: "warn",
        applicable: false,
        source_revision: None,
        doctor_status: FindingStatus::Unavailable,
        release_status: "unavailable",
        blocks: true,
        evidence: Evidence::Unjudged,
    },
    Case {
        id: "POLICY-DOCS-001",
        severity: "warn",
        applicable: true,
        source_revision: Some("2020-01-01T00:00:00Z"),
        doctor_status: FindingStatus::Warn,
        release_status: "stale",
        blocks: true,
        evidence: Evidence::Stale,
    },
    Case {
        id: "POLICY-DOCS-001",
        severity: "warn",
        applicable: true,
        source_revision: None,
        doctor_status: FindingStatus::Warn,
        release_status: "pass",
        blocks: false,
        evidence: Evidence::Warning,
    },
    Case {
        id: "POLICY-DOCS-001",
        severity: "pass",
        applicable: true,
        source_revision: None,
        doctor_status: FindingStatus::Pass,
        release_status: "pass",
        blocks: false,
        evidence: Evidence::Healthy,
    },
];

fn run_git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The trimmed stdout of one git invocation.
fn git_output(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Write the Gate evidence record the shared delivery rule accepts,
/// bound to the project's own HEAD revision. The record is written
/// after the fixture's commit, so the revision it names is the one a
/// release captures. With it in place the DriftWatch check is the
/// only check that can decide the plan's readiness.
fn write_gate_evidence(dir: &Path, project_id: &str) {
    let revision = git_output(dir, &["rev-parse", "HEAD"]);
    let record = serde_json::json!({
        "contract": forge::gate::contract::GATE_CONTRACT_VERSION,
        "project_id": project_id,
        "runtime": "forge",
        "profile": "full",
        "revision": revision,
        "aggregate": "passed",
        "checks": [],
        "observed_at": "2026-01-01T00:00:00Z",
        "dry_run": false
    });
    let path = dir
        .join(".forge/gate")
        .join(project_id)
        .join("evidence.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
}

/// A project that declares the DriftWatch check and nothing else, so
/// the plan's readiness turns on the policy report alone.
fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n  checks:\n    - kind: driftwatch\n  changelog: CHANGELOG.md\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"dw-truth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();
    fs::write(dir.join("lib.rs"), "//! dw-truth.\n").unwrap();
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
    write_gate_evidence(dir, id);
}

/// The legacy adapter report the fake policy adapter prints. The
/// contract is exactly `0.1.0`, the one the adapter accepts.
fn adapter_report(case: &Case) -> String {
    serde_json::json!({
        "tool": "driftwatch",
        "tool_version": "0.9.9",
        "contract": "0.1.0",
        "source_revision": case.source_revision,
        "findings": [{
            "id": case.id,
            "category": "documentation",
            "severity": case.severity,
            "applicable": case.applicable,
            "message": "the documentation is behind the source",
            "evidence": ["docs/README.md was not regenerated"]
        }]
    })
    .to_string()
}

/// A fake policy adapter: the `--version` probe answers without a
/// report, every other invocation prints the legacy adapter report.
fn write_adapter_script(dir: &Path, report: &str) -> PathBuf {
    let report_path = dir.join("report.json");
    fs::write(&report_path, report).unwrap();
    let path = dir.join("fake-driftwatch.sh");
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\necho \"driftwatch 0.9.9\"\nexit 0\nfi\ncat \"{}\"\n",
            report_path.display()
        ),
    )
    .unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// The release plan the CLI renders, prepared through the public
/// entry point.
fn prepare_plan(proj: &Path, id: &str) -> PlanReport {
    let (manifest, _) = Manifest::load_from_dir(proj, None).expect("manifest");
    let meta = manifest.release.clone().expect("release section");
    let config = ReleaseConfig::from_manifest_meta(&meta).expect("release config");
    let request = ReleaseRequest {
        project_id: id.to_string(),
        version: Semver::parse("1.0.0").expect("semver"),
        confirm: false,
        dry_run: true,
        retry: false,
        stages: Vec::new(),
    };
    let policy = DriftWatchConfig::from_env();
    prepare_release(proj, &manifest, &config, &request, &policy).expect("prepare release")
}

/// Whether the captured check calls the report healthy anywhere.
fn mentions_healthy(check: &CapturedCheck) -> bool {
    check.evidence.iter().any(|line| line.contains("healthy")) || check.detail.contains("healthy")
}

#[test]
fn the_release_lane_and_the_doctor_reach_one_verdict_about_a_policy_report() {
    let _guard = ENV_LOCK.lock().unwrap();
    for (index, case) in CASES.iter().enumerate() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("proj");
        let id = format!("dw-truth-{index}");
        write_project(&proj, &id);
        let script = write_adapter_script(dir.path(), &adapter_report(case));
        std::env::set_var("FORGE_DRIFTWATCH_BIN", &script);

        let plan = prepare_plan(&proj, &id);
        let check = plan
            .checks
            .iter()
            .find(|c| c.kind == "driftwatch")
            .expect("the driftwatch check is captured")
            .clone();

        // The doctor judges the same report with the same rule.
        let policy = DriftWatchConfig::from_env();
        let outcome = run_driftwatch(&proj, &policy);
        let PolicyOutcome::Reported(report) = &outcome else {
            std::env::remove_var("FORGE_DRIFTWATCH_BIN");
            panic!("the adapter must produce a report for case {index}");
        };
        let stale = observation_is_stale_from_report(&proj, report);
        let verdict = PolicyVerdict::judge(report, stale);
        let doctor = run_doctor(&proj, Some(Maturity::L1), None, Some(&outcome))
            .expect("the doctor runs over the fixture");
        let finding = doctor
            .findings
            .iter()
            .find(|f| f.id == "driftwatch-policy")
            .expect("the doctor reports the policy rollup");

        std::env::remove_var("FORGE_DRIFTWATCH_BIN");

        assert_eq!(
            finding.status, case.doctor_status,
            "the doctor's verdict for case {index}"
        );
        assert_eq!(
            verdict.conclusion.to_finding_status(),
            case.doctor_status,
            "the shared verdict for case {index}"
        );
        assert_eq!(
            check.status, case.release_status,
            "the release check status for case {index}"
        );
        assert_eq!(
            plan.ready, !case.blocks,
            "the plan readiness for case {index}: {}",
            plan.note
        );
        if case.blocks {
            assert!(
                plan.note
                    .contains(&format!("driftwatch:{}", case.release_status)),
                "the blocked plan names the driftwatch check: {}",
                plan.note
            );
        } else {
            assert!(
                !plan.note.contains("driftwatch:"),
                "the driftwatch check does not block this plan: {}",
                plan.note
            );
        }
        match case.evidence {
            Evidence::Failing => {
                assert_eq!(check.evidence, vec![case.id.to_string()]);
            }
            Evidence::Unjudged => {
                assert!(
                    check
                        .evidence
                        .iter()
                        .any(|line| line.contains("no driftwatch rule applied")),
                    "the evidence names that no rule applied: {:?}",
                    check.evidence
                );
                assert!(
                    !mentions_healthy(&check),
                    "an unjudged report is never called healthy: {:?}",
                    check.evidence
                );
            }
            Evidence::Stale => {
                assert!(
                    check
                        .evidence
                        .iter()
                        .any(|line| line.contains("older source revision")),
                    "the evidence names the older source revision: {:?}",
                    check.evidence
                );
                assert!(
                    !mentions_healthy(&check),
                    "a stale report is never called healthy: {:?}",
                    check.evidence
                );
            }
            Evidence::Warning => {
                assert!(
                    check.evidence.iter().any(|line| line.contains(case.id)),
                    "the evidence names the warning rule: {:?}",
                    check.evidence
                );
                assert!(
                    !mentions_healthy(&check),
                    "a warninging report is never called healthy: {:?}",
                    check.evidence
                );
            }
            Evidence::Healthy => {
                assert_eq!(
                    check.evidence,
                    vec!["driftwatch report healthy".to_string()]
                );
                assert_eq!(check.detail, "driftwatch report is healthy");
            }
        }
    }
}
