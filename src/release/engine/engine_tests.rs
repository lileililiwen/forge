//! Release engine tests.

#[cfg(test)]
pub(super) mod tests {
    use super::super::super::{
        release_config_from_manifest, CapturedCheck, ReleaseAdapterConfig, ReleaseConfig,
        ReleaseRequest, Semver,
    };
    use super::super::{
        apply::apply_release,
        operations::{list_releases, run_with_timeout},
        prepare::{check_readiness, prepare_release},
    };
    use crate::core::manifest::{Manifest, Maturity};
    use crate::policy::DriftWatchConfig;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(cwd: &Path, args: &[&str]) {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd);
        for a in args {
            cmd.arg(a);
        }
        let out = cmd.output().expect("git invocation");
        assert!(
            out.status.success(),
            "git {:?} failed: stderr={}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// The trimmed stdout of one git invocation.
    fn git_output(cwd: &Path, args: &[&str]) -> String {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd);
        for a in args {
            cmd.arg(a);
        }
        let out = cmd.output().expect("git invocation");
        assert!(
            out.status.success(),
            "git {:?} failed: stderr={}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// Write the Gate evidence record the shared delivery rule
    /// accepts, bound to the project's own HEAD revision. The record
    /// is written after the fixture's commit, so the revision it
    /// names is the one a release captures.
    fn write_gate_evidence(dir: &Path, project_id: &str) {
        let revision = git_output(dir, &["rev-parse", "HEAD"]);
        let record = serde_json::json!({
            "contract": crate::gate::contract::GATE_CONTRACT_VERSION,
            "project_id": project_id,
            "runtime": "forge",
            "profile": "full",
            "revision": revision,
            "aggregate": "passed",
            "checks": [],
            "observed_at": "2026-10-21T00:00:00Z",
            "dry_run": false
        });
        let path = dir
            .join(".forge/gate")
            .join(project_id)
            .join("evidence.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
    }

    /// The `release.checks` list the shared fixture declares.
    const FIXTURE_CHECKS: &str = "  checks:\n    - kind: doctor\n    - kind: test\n";

    /// Write a release fixture whose manifest declares a
    /// `release:` block with the given `checks:` block. An empty
    /// block declares no release check at all, so the release has
    /// no applicable evidence.
    fn write_release_fixture(dir: &Path, id: &str, checks_yaml: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        let text = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n{checks_yaml}  changelog: CHANGELOG.md\n{body}"
        );
        fs::write(dir.join("forge.yaml"), text).unwrap();
        fs::write(dir.join("README.md"), "v1\n").unwrap();
        fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
        // Provide a minimal but valid Cargo workspace so
        // the doctor build-config check and the `cargo
        // test` command both pass without contacting the
        // network. The tests are the only thing the
        // contract claims about the test command; an empty
        // test set reports as a passing test run.
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"forge-rel-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"lib.rs\"\n",
        )
        .unwrap();
        fs::write(dir.join("lib.rs"), "//! Forge release fixture crate.\n").unwrap();
        git(dir, &["init", "-q"]);
        git(dir, &["config", "user.email", "forge@example.com"]);
        git(dir, &["config", "user.name", "Forge Test"]);
        git(dir, &["config", "init.defaultBranch", "main"]);
        git(
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
        git(dir, &["commit", "-q", "-m", "initial"]);
    }

    /// Write a fixture whose manifest declares the `doctor` and
    /// `test` checks and whose project has Gate evidence bound to
    /// its own revision, so the Gate check passes.
    fn write_release_manifest(dir: &Path, id: &str, body: &str) {
        write_release_fixture(dir, id, FIXTURE_CHECKS, body);
        write_gate_evidence(dir, id);
    }

    /// Write a fixture whose manifest declares a `release:` block
    /// with no `checks:` list and no Gate evidence: the release has
    /// no applicable evidence at all.
    fn write_release_manifest_without_checks(dir: &Path, id: &str, body: &str) {
        write_release_fixture(dir, id, "", body);
    }

    /// Build one captured check for the readiness judgement.
    fn captured_check(kind: &str, status: &str, applicable: bool) -> CapturedCheck {
        CapturedCheck {
            kind: kind.to_string(),
            status: status.to_string(),
            applicable,
            evidence: Vec::new(),
            detail: format!("{kind} is {status}"),
            source_revision: "abc123456789".to_string(),
        }
    }

    fn read_manifest(dir: &Path) -> Manifest {
        let (manifest, _) = Manifest::load_from_dir(dir, None).expect("manifest");
        manifest
    }

    fn read_release_config(dir: &Path) -> ReleaseConfig {
        let manifest = read_manifest(dir);
        release_config_from_manifest(&manifest).expect("release config")
    }

    #[test]
    fn prepare_captures_changelog_and_disabled_checks() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-prep", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.2.3").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        assert!(plan.changelog.is_some());
        // doctor/test run because the fixture declares them;
        // driftwatch is not declared, so it is reported as
        // disabled; the Gate check always applies.
        let kinds: Vec<&str> = plan.checks.iter().map(|c| c.kind.as_str()).collect();
        assert!(kinds.contains(&"doctor"));
        assert!(kinds.contains(&"test"));
        assert!(kinds.contains(&"driftwatch"));
        assert!(kinds.contains(&"gate"));
        let gate = plan
            .checks
            .iter()
            .find(|c| c.kind == "gate")
            .expect("gate check");
        assert!(gate.applicable, "the Gate check always applies");
        assert_eq!(gate.status, "pass");
    }

    #[test]
    fn prepare_with_a_passing_check_is_ready() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-ready", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        assert!(plan.ready, "plan must be ready: {}", plan.note);
        assert!(
            plan.note.contains("check(s) passed"),
            "note must report the passing check count: {}",
            plan.note
        );
    }

    #[test]
    fn prepare_refuses_a_project_without_a_declared_check() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest_without_checks(dir, "rel-nocheck", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        // The manifest declares no check, so the Gate check is the
        // only applicable one. With no Gate evidence the release
        // has no evidence at all: that is the absence of evidence,
        // not a pass.
        assert!(!plan.ready, "plan must not be ready: {}", plan.note);
        let gate = plan
            .checks
            .iter()
            .find(|c| c.kind == "gate")
            .expect("gate check");
        assert!(gate.applicable, "the Gate check always applies");
        assert_eq!(gate.status, "fail");
        assert!(
            gate.detail.contains("Forge Gate evidence is absent"),
            "the plan must name the missing Gate evidence: {}",
            gate.detail
        );
        assert!(
            !plan.note.contains("check(s) passed"),
            "note must not claim a passing check count: {}",
            plan.note
        );
    }

    #[test]
    fn check_readiness_requires_an_applicable_passing_check() {
        // No check at all is the absence of evidence.
        let empty = check_readiness(&[]);
        assert!(!empty.ready);
        assert_eq!(
            empty.reason,
            "no applicable release check; declare at least one of doctor, test, driftwatch in release.checks"
        );

        // A slice whose checks are all disabled has no
        // applicable check either.
        let disabled = check_readiness(&[
            captured_check("doctor", "disabled", false),
            captured_check("test", "disabled", false),
        ]);
        assert!(!disabled.ready);
        assert!(
            disabled.reason.contains("no applicable release check"),
            "reason: {}",
            disabled.reason
        );

        // One applicable passing check is enough.
        let passing = check_readiness(&[
            captured_check("doctor", "disabled", false),
            captured_check("test", "pass", true),
        ]);
        assert!(passing.ready, "reason: {}", passing.reason);
        assert!(passing.reason.is_empty());

        // An applicable check that is not passing is named with
        // its status, in the order the checks appear.
        let failing = check_readiness(&[
            captured_check("doctor", "fail", true),
            captured_check("test", "stale", true),
        ]);
        assert!(!failing.ready);
        assert_eq!(failing.reason, "failing check(s) doctor:fail, test:stale");
    }

    #[test]
    fn prepare_marks_check_unavailable_when_doctor_fails() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        fs::create_dir_all(dir).unwrap();
        // Empty working tree (no Cargo.toml, no
        // forge.yaml); init a git repo with a commit so
        // the source revision capture can succeed.
        git(dir, &["init", "-q"]);
        git(dir, &["config", "user.email", "forge@example.com"]);
        git(dir, &["config", "user.name", "Forge Test"]);
        git(dir, &["config", "init.defaultBranch", "main"]);
        git(dir, &["checkout", "-q", "-b", "main"]);
        fs::write(dir.join("README.md"), "v1\n").unwrap();
        git(dir, &["add", "--", "README.md"]);
        git(dir, &["commit", "-q", "-m", "initial"]);
        // The Gate evidence is bound to this commit, so the doctor
        // is the only failing check in the plan.
        write_gate_evidence(dir, "rel-nochk");
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: rel-nochk\n  name: rel-nochk\n  profile: rust-web\nrelease:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("forge.yaml"),
            "schema: 1\nproject:\n  id: rel-nochk\n  name: rel-nochk\n  profile: rust-web\nrelease:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n",
        )
        .unwrap();
        let config = release_config_from_manifest(&manifest).unwrap();
        let semver = Semver::parse("1.2.3").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        // Doctor needs a forge.yaml; without a changelog the
        // load_changelog call would fail, so create one.
        fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        // Plan is not ready because doctor reports a
        // `build-config` finding (no Cargo.toml in the
        // empty project).
        assert!(!plan.healthy());
        let doctor = plan
            .checks
            .iter()
            .find(|c| c.kind == "doctor")
            .expect("doctor check");
        assert!(doctor.status != "pass" || !plan.ready);
        // The Gate evidence is fresh for this revision, so the
        // doctor is the only failing check.
        let gate = plan
            .checks
            .iter()
            .find(|c| c.kind == "gate")
            .expect("gate check");
        assert_eq!(gate.status, "pass");
    }

    #[test]
    fn prepare_omits_docs_stage_when_no_locale_configured() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-bare", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("0.1.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        assert!(plan.docs_locales.is_empty());
    }

    #[test]
    fn apply_refuses_without_confirm() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-noconf", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: false,
            retry: false,
            stages: config.stages.clone(),
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let err = apply_release(dir, &manifest, &config, &request, &adapters).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn apply_refuses_a_project_without_a_declared_check() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest_without_checks(dir, "rel-nocheck-apply", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: true,
            dry_run: false,
            retry: false,
            stages: config.stages.clone(),
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let err = apply_release(dir, &manifest, &config, &request, &adapters).unwrap_err();
        assert_eq!(err.code(), "release-check-failed");
        assert!(
            err.to_string().contains("gate:fail"),
            "the refusal must name the failing Gate check: {err}"
        );
        // The refusal happens before any stage runs, so the
        // project has no tag.
        let tags = Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("tag")
            .arg("--list")
            .output()
            .expect("git tag --list");
        assert!(
            String::from_utf8_lossy(&tags.stdout).trim().is_empty(),
            "no tag may be created: {}",
            String::from_utf8_lossy(&tags.stdout)
        );
    }

    #[test]
    fn the_gate_check_always_applies_to_a_release() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        // The manifest declares no check at all and the project has
        // no Gate evidence: the Gate check is still captured, and
        // it is applicable.
        write_release_manifest_without_checks(dir, "rel-gate-always", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        let gate = plan
            .checks
            .iter()
            .find(|c| c.kind == "gate")
            .expect("gate check");
        assert!(gate.applicable, "the Gate check always applies");
        assert_eq!(gate.status, "fail");
        assert!(!plan.ready, "plan must not be ready: {}", plan.note);
        // With Gate evidence bound to this revision the same
        // manifest is ready: the Gate check is the only applicable
        // one and it passes.
        write_gate_evidence(dir, "rel-gate-always");
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        let gate = plan
            .checks
            .iter()
            .find(|c| c.kind == "gate")
            .expect("gate check");
        assert!(gate.applicable, "the Gate check always applies");
        assert_eq!(gate.status, "pass");
        assert!(plan.ready, "plan must be ready: {}", plan.note);
    }

    #[test]
    fn the_gate_check_reports_the_shared_rule_reason() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-gate-absent", "");
        // The project has no Gate evidence at all.
        fs::remove_dir_all(dir.join(".forge/gate")).unwrap();
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        let gate = plan
            .checks
            .iter()
            .find(|c| c.kind == "gate")
            .expect("gate check");
        assert_eq!(gate.status, "fail");
        assert_eq!(
            gate.detail,
            "Forge Gate evidence is absent for `rel-gate-absent`; run `forge gate --profile full` from the project directory and retry"
        );
    }

    #[test]
    fn apply_refuses_a_stale_gate_evidence_before_any_stage() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-gate-stale", "");
        // Move HEAD past the revision the Gate evidence is bound
        // to: the evidence is now stale for the applied revision.
        fs::write(dir.join("README.md"), "v2\n").unwrap();
        git(dir, &["add", "--", "README.md"]);
        git(dir, &["commit", "-q", "-m", "second"]);
        let head = git_output(dir, &["rev-parse", "HEAD"]);
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: true,
            dry_run: false,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        // The plan names the stale evidence.
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        let gate = plan
            .checks
            .iter()
            .find(|c| c.kind == "gate")
            .expect("gate check");
        assert_eq!(gate.status, "fail");
        assert!(
            gate.detail.contains("freshness=stale"),
            "the plan must name the stale evidence: {}",
            gate.detail
        );
        // The apply step re-judges the evidence at the applied
        // revision and refuses before any stage runs.
        let adapters = ReleaseAdapterConfig::from_env();
        let err = apply_release(dir, &manifest, &config, &request, &adapters).unwrap_err();
        assert_eq!(err.code(), "release-check-failed");
        assert!(
            err.to_string().contains("gate:fail"),
            "the refusal must name the failing Gate check: {err}"
        );
        let tags = Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("tag")
            .arg("--list")
            .output()
            .expect("git tag --list");
        assert!(
            String::from_utf8_lossy(&tags.stdout).trim().is_empty(),
            "no tag may be created: {}",
            String::from_utf8_lossy(&tags.stdout)
        );
        assert_eq!(
            git_output(dir, &["rev-parse", "HEAD"]),
            head,
            "no stage may commit"
        );
    }

    #[test]
    fn apply_records_tag_conflict_on_existing_different_commit() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-conflict", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        // First commit on top of initial.
        fs::write(dir.join("README.md"), "v2\n").unwrap();
        git(dir, &["add", "--", "README.md"]);
        git(dir, &["commit", "-q", "-m", "second"]);
        // Create a `v1.0.0` tag at the original commit.
        let original = String::from_utf8_lossy(
            &Command::new("git")
                .arg("-C")
                .arg(dir)
                .arg("rev-parse")
                .arg("HEAD~1")
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        git(
            dir,
            &["tag", "-a", "v1.0.0", "-m", "pre-existing", &original],
        );
        // Move HEAD to a new commit so the requested revision
        // is different from the tagged one.
        fs::write(dir.join("README.md"), "v3\n").unwrap();
        git(dir, &["add", "--", "README.md"]);
        git(dir, &["commit", "-q", "-m", "third"]);
        // The Gate evidence is re-bound to the new revision, the
        // way an owner re-runs the Gate before releasing again.
        write_gate_evidence(dir, "rel-conflict");
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: true,
            dry_run: false,
            retry: false,
            stages: config.stages.clone(),
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let report = apply_release(dir, &manifest, &config, &request, &adapters).unwrap();
        let tag = report
            .stage_outcomes
            .iter()
            .find(|s| s.stage == "tag")
            .expect("tag stage");
        assert_eq!(tag.status, "conflict");
    }

    #[test]
    fn apply_skip_when_tag_points_at_current_commit() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-skip", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let sha = String::from_utf8_lossy(
            &Command::new("git")
                .arg("-C")
                .arg(dir)
                .arg("rev-parse")
                .arg("HEAD")
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        git(dir, &["tag", "-a", "v1.0.0", "-m", "preexisting", &sha]);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: true,
            dry_run: false,
            retry: false,
            // Override the stage list to just the tag
            // stage so the commit stage does not move
            // HEAD before the tag check.
            stages: vec!["tag".to_string()],
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let report = apply_release(dir, &manifest, &config, &request, &adapters).unwrap();
        let tag = report
            .stage_outcomes
            .iter()
            .find(|s| s.stage == "tag")
            .expect("tag stage");
        assert_eq!(tag.status, "skipped");
    }

    #[test]
    fn apply_persists_release_state_for_retry() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-state", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: true,
            dry_run: false,
            retry: false,
            stages: config.stages.clone(),
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let report = apply_release(dir, &manifest, &config, &request, &adapters).unwrap();
        let state_path = std::path::PathBuf::from(&report.state_path);
        assert!(state_path.is_file(), "state path missing: {state_path:?}");
    }

    #[test]
    fn list_reports_zero_entries_for_fresh_project() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-list", "");
        let entries = list_releases(dir, "rel-list").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn maturity_in_manifest_is_preserved() {
        // Sanity: the manifest fixture preserves the
        // maturity field so the release contract can use
        // it for doctor without re-reading the manifest.
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-maturity", "");
        let (manifest, _) = Manifest::load_from_dir(dir, None).unwrap();
        assert!(matches!(manifest.project.maturity, Some(Maturity::L1)));
    }

    #[test]
    fn run_with_timeout_returns_output_when_adapter_completes() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("echo receipt-123")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        let output =
            run_with_timeout(cmd, std::time::Duration::from_secs(10), "test adapter").unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("receipt-123"));
    }

    #[test]
    fn run_with_timeout_kills_and_reaps_child_on_timeout() {
        let tmp = TempDir::new().unwrap();
        let pid_file = tmp.path().join("adapter.pid");
        // Adapter records its pid, then sleeps past the deadline.
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(format!("echo $$ > {} && exec sleep 30", pid_file.display()))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        let start = std::time::Instant::now();
        let err = run_with_timeout(cmd, std::time::Duration::from_millis(300), "test adapter")
            .expect_err("timeout must fail");
        assert!(err.contains("timeout"), "unexpected error: {err}");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(10),
            "timeout must return promptly"
        );
        let pid_text = std::fs::read_to_string(&pid_file).expect("pid file");
        let pid = pid_text.trim().to_string();
        // The direct child must be terminated and reaped: `kill -0`
        // fails when the process no longer exists.
        std::thread::sleep(std::time::Duration::from_millis(200));
        let probe = Command::new("kill")
            .arg("-0")
            .arg(&pid)
            .output()
            .expect("kill probe");
        assert!(
            !probe.status.success(),
            "timed-out adapter child {pid} is still alive"
        );
    }

    #[test]
    fn run_with_timeout_reports_nonzero_exit_without_success() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("echo boom >&2; exit 3")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        let output =
            run_with_timeout(cmd, std::time::Duration::from_secs(10), "test adapter").unwrap();
        assert!(
            !output.status.success(),
            "non-zero exit must not read as success"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("boom"));
    }
}
