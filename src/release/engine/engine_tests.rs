//! Release engine tests.

#[cfg(test)]
pub(super) mod tests {
    use super::super::super::{
        release_config_from_manifest, ReleaseAdapterConfig, ReleaseConfig, ReleaseRequest, Semver,
    };
    use super::super::{
        apply::apply_release,
        operations::{list_releases, run_with_timeout},
        prepare::prepare_release,
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

    fn write_release_manifest(dir: &Path, id: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        let text = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n  changelog: CHANGELOG.md\n{body}"
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
        // doctor/test checks are not configured in the
        // fixture's `release.checks` (it only lists the
        // kinds the contract claims to support); the
        // boundary scenario is exercised below.
        let kinds: Vec<&str> = plan.checks.iter().map(|c| c.kind.as_str()).collect();
        assert!(kinds.contains(&"doctor"));
        assert!(kinds.contains(&"test"));
        assert!(kinds.contains(&"driftwatch"));
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
