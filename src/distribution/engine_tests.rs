//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::core::manifest::Manifest;
    use crate::distribution::contract::PRIMARY_REMOTE;
    use crate::distribution::engine::test_fixtures::*;
    use crate::distribution::engine::{
        apply_mirror, distribution_config_from_manifest, load_mirror_state, mirror_support_status,
        parse_mirror_provider, plan_mirror, redact_distribution_evidence, save_mirror_state,
    };
    use crate::distribution::model::{
        DistributionConfig, MirrorConfigEntry, MirrorProvider, MirrorRequest, MirrorState,
        MirrorSupportStatus,
    };
    use std::path::Path;
    use std::process::Command;

    fn manifest_with_distribution(text: &str) -> Manifest {
        Manifest::parse(Path::new("forge.yaml"), text.as_bytes()).expect("manifest")
    }

    #[test]
    fn parse_provider_recognises_supported_and_planned() {
        assert!(matches!(
            parse_mirror_provider("github").unwrap(),
            MirrorProvider::Github
        ));
        assert!(matches!(
            parse_mirror_provider("gitee").unwrap(),
            MirrorProvider::Gitee
        ));
        assert!(matches!(
            parse_mirror_provider("gitlab").unwrap(),
            MirrorProvider::Gitlab
        ));
        assert!(matches!(
            parse_mirror_provider("codeberg").unwrap(),
            MirrorProvider::Codeberg
        ));
        let err = parse_mirror_provider("sourceforge").unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn support_status_is_supported_for_github_gitee_only() {
        assert_eq!(
            mirror_support_status(MirrorProvider::Github),
            MirrorSupportStatus::Supported
        );
        assert_eq!(
            mirror_support_status(MirrorProvider::Gitee),
            MirrorSupportStatus::Supported
        );
        assert_eq!(
            mirror_support_status(MirrorProvider::Gitlab),
            MirrorSupportStatus::Planned
        );
        assert_eq!(
            mirror_support_status(MirrorProvider::Codeberg),
            MirrorSupportStatus::Planned
        );
    }

    #[test]
    fn config_from_manifest_uses_first_mirror_as_primary_fallback() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  mirrors:\n    - gitee\n    - github\n",
        );
        let config = distribution_config_from_manifest(&manifest).unwrap();
        assert!(config.primary.is_none(), "no primary declared");
        assert_eq!(config.mirrors.len(), 2);
        assert_eq!(config.enabled_targets().len(), 2);
    }

    #[test]
    fn config_from_manifest_keeps_declared_primary_separate() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  primary: github\n  mirrors:\n    - gitee\n",
        );
        let config = distribution_config_from_manifest(&manifest).unwrap();
        assert_eq!(
            config.primary.as_ref().map(|p| p.provider),
            Some(MirrorProvider::Github)
        );
        assert_eq!(config.primary.as_ref().unwrap().remote_name, "origin");
        assert_eq!(config.mirrors[0].provider, MirrorProvider::Gitee);
        assert_eq!(config.mirrors[0].remote_name, "mirror-gitee");
    }

    #[test]
    fn config_from_manifest_rejects_duplicate_enabled_mirrors() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  mirrors:\n    - gitee\n    - provider: gitee\n      enabled: true\n",
        );
        let err = distribution_config_from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn config_from_manifest_rejects_unknown_provider() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  primary: bogus\n",
        );
        let err = distribution_config_from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn config_from_manifest_rejects_empty_distribution() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\n",
        );
        let err = distribution_config_from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn request_refuses_empty_refs_and_dash_prefixed_refs() {
        let bad = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["-evil".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let err = bad.validate().unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
        let bad = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["main with space".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let err = bad.validate().unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn request_refuses_implicit_remote_write() {
        let bad = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["main".to_string()],
            confirm: false,
            dry_run: false,
            retry_failed: false,
        };
        let err = bad.validate().unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn plan_reports_disabled_mirror_as_boundary_outcome() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  primary: github\n  mirrors:\n    - provider: gitee\n      enabled: false\n",
        );
        let config = distribution_config_from_manifest(&manifest).unwrap();
        let req = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: true,
            retry_failed: false,
        };
        let state = MirrorState::default();
        let report = plan_mirror(&config, &req, &state, Path::new("/tmp/state.json")).unwrap();
        let disabled = report
            .outcomes
            .iter()
            .find(|o| o.provider == "gitee" && o.status == "disabled")
            .expect("disabled boundary");
        assert_eq!(disabled.role, "mirror");
    }

    #[test]
    fn plan_marks_planned_providers_as_unavailable() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  primary: github\n  mirrors:\n    - gitlab\n",
        );
        let config = distribution_config_from_manifest(&manifest).unwrap();
        let req = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: true,
            retry_failed: false,
        };
        let state = MirrorState::default();
        let report = plan_mirror(&config, &req, &state, Path::new("/tmp/state.json")).unwrap();
        let planned = report
            .outcomes
            .iter()
            .find(|o| o.provider == "gitlab")
            .expect("planned");
        assert_eq!(planned.status, "unavailable");
    }

    #[test]
    fn apply_records_primary_and_mirror_independently() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "distrib-app");
        let primary_bare = bare_repository(tmp.path(), "origin");
        let mirror_bare = bare_repository(tmp.path(), "gitee");
        add_remote(&dir, "origin", &primary_bare);
        add_remote(&dir, "mirror-gitee", &mirror_bare);

        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: true,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "distrib-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let report = apply_mirror(&dir, &config, &req).unwrap();
        let primary = report
            .outcomes
            .iter()
            .find(|o| o.role == "primary")
            .expect("primary outcome");
        let mirror = report
            .outcomes
            .iter()
            .find(|o| o.role == "mirror")
            .expect("mirror outcome");
        assert_eq!(primary.status, "delivered");
        assert_eq!(mirror.status, "delivered");
        assert!(primary.commit_sha.is_some());
        assert!(mirror.commit_sha.is_some());
        assert!(report.healthy());
    }

    #[test]
    fn apply_records_partial_failure_when_mirror_unavailable() {
        // Primary push works (real bare repo), mirror push
        // fails because the mirror-gitee remote is not
        // configured. The contract records primary as
        // delivered and mirror as failed without rolling
        // back the primary.
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "partial-app");
        let primary_bare = bare_repository(tmp.path(), "origin");
        add_remote(&dir, "origin", &primary_bare);
        // mirror-gitee is intentionally not added.

        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: true,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "partial-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let report = apply_mirror(&dir, &config, &req).unwrap();
        let primary = report
            .outcomes
            .iter()
            .find(|o| o.role == "primary")
            .expect("primary outcome");
        let mirror = report
            .outcomes
            .iter()
            .find(|o| o.role == "mirror")
            .expect("mirror outcome");
        assert_eq!(primary.status, "delivered");
        assert_eq!(mirror.status, "failed");
        // Partial outcome: the report must surface the
        // primary state and the per-remote failure without
        // misreporting one for the other.
        assert!(!report.healthy(), "primary delivered but mirror failed");
        // State file records only the primary delivery.
        let state = load_mirror_state(Path::new(&report.state_path)).unwrap();
        assert!(state.primary_delivered.contains_key("main"));
        assert!(state.mirrors_delivered.is_empty());
    }

    #[test]
    fn apply_refuses_diverged_mirror_with_recovery_note() {
        // The mirror's history is forced to diverge from the
        // primary so a non-fast-forward push is required.
        // The contract refuses the force and surfaces
        // `diverged` with the diverging detail (redacted).
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "diverged-app");
        let primary_bare = bare_repository(tmp.path(), "origin");
        let mirror_bare = bare_repository(tmp.path(), "gitee");
        add_remote(&dir, "origin", &primary_bare);
        add_remote(&dir, "mirror-gitee", &mirror_bare);
        // Push primary and mirror first so both bare
        // repositories hold the `main` branch; a subsequent
        // `git clone` of the mirror would otherwise land on
        // a detached HEAD with no branches.
        let run = |args: &[&str]| {
            let mut cmd = Command::new("git");
            cmd.arg("-C").arg(&dir);
            for a in args {
                cmd.arg(a);
            }
            let out = cmd.output().expect("git");
            assert!(
                out.status.success(),
                "git {:?}: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["push", "origin", "main"]);
        run(&["push", "mirror-gitee", "main"]);
        // Build a divergent history on the mirror: clone it,
        // add a commit, push back.
        let mirror_clone = tmp.path().join("mirror-clone");
        run_via(&[&format!("git clone {mirror_bare:?} {mirror_clone:?}")]);
        let commit_in_clone = |args: &[&str]| {
            let mut cmd = Command::new("git");
            cmd.arg("-C").arg(&mirror_clone);
            for a in args {
                cmd.arg(a);
            }
            let out = cmd.output().expect("git in clone");
            assert!(
                out.status.success(),
                "git clone {:?}: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        commit_in_clone(&["config", "user.email", "forge@example.com"]);
        commit_in_clone(&["config", "user.name", "Forge Test"]);
        // The bare repo's HEAD was set to `main` so the
        // clone lands on the same branch name as the
        // working tree; checkout explicitly to anchor the
        // diverging commit.
        commit_in_clone(&["checkout", "main"]);
        std::fs::write(mirror_clone.join("README.md"), "divergent\n").unwrap();
        commit_in_clone(&["add", "README.md"]);
        commit_in_clone(&["commit", "-q", "-m", "divergent history"]);
        commit_in_clone(&["push", "origin", "main"]);
        // Move the local primary forward so the next push
        // is non-fast-forward from the mirror's perspective.
        std::fs::write(dir.join("README.md"), "v2\n").unwrap();
        run(&["add", "README.md"]);
        run(&["commit", "-q", "-m", "primary ahead"]);
        run(&["push", "origin", "main"]);

        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: true,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "diverged-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let report = apply_mirror(&dir, &config, &req).unwrap();
        let mirror = report
            .outcomes
            .iter()
            .find(|o| o.role == "mirror")
            .expect("mirror outcome");
        assert_eq!(mirror.status, "diverged");
        assert!(!mirror.recovery.is_empty());
    }

    #[test]
    fn retry_skips_already_delivered_refs() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "retry-app");
        let primary_bare = bare_repository(tmp.path(), "origin");
        let mirror_bare = bare_repository(tmp.path(), "gitee");
        add_remote(&dir, "origin", &primary_bare);
        add_remote(&dir, "mirror-gitee", &mirror_bare);
        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: true,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "retry-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let first = apply_mirror(&dir, &config, &req).unwrap();
        let primary_first = first.outcomes.iter().find(|o| o.role == "primary").unwrap();
        let mirror_first = first.outcomes.iter().find(|o| o.role == "mirror").unwrap();
        assert_eq!(primary_first.status, "delivered");
        assert_eq!(mirror_first.status, "delivered");

        // Retry must skip both remotes; the state file is the
        // source of truth for what was already delivered.
        let retry = MirrorRequest {
            project_id: "retry-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: true,
        };
        let second = apply_mirror(&dir, &config, &retry).unwrap();
        let primary_second = second
            .outcomes
            .iter()
            .find(|o| o.role == "primary")
            .unwrap();
        let mirror_second = second.outcomes.iter().find(|o| o.role == "mirror").unwrap();
        assert_eq!(primary_second.status, "skipped");
        assert_eq!(mirror_second.status, "skipped");
    }

    #[test]
    fn disabled_mirror_is_reported_without_writing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "disabled-app");
        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: false,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "disabled-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: true,
            retry_failed: false,
        };
        let state = MirrorState::default();
        let report = plan_mirror(&config, &req, &state, &dir.join("state.json")).unwrap();
        let disabled = report
            .outcomes
            .iter()
            .find(|o| o.provider == "gitee" && o.status == "disabled")
            .expect("disabled");
        assert_eq!(disabled.role, "mirror");
    }

    #[test]
    fn redaction_strips_credentials_from_evidence() {
        let redacted =
            redact_distribution_evidence("auth: ghp_abcdefghijklmnopqrstuvwxyz0123456789");
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("ghp_"));
    }

    #[test]
    fn state_round_trip_preserves_delivered_refs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");
        let mut state = MirrorState::default();
        state.record_primary("main", "abc123");
        state.record_mirror("gitee", "main", "abc123");
        save_mirror_state(&path, &state).unwrap();
        let restored = load_mirror_state(&path).unwrap();
        assert_eq!(restored.primary_delivered.get("main").unwrap(), "abc123");
        assert_eq!(
            restored
                .mirrors_delivered
                .get("gitee")
                .and_then(|m| m.get("main"))
                .unwrap(),
            "abc123"
        );
    }

    fn run_via(args: &[&str]) {
        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        let joined = args.join(" ");
        cmd.arg(joined);
        let out = cmd.output().expect("sh -c");
        assert!(
            out.status.success(),
            "sh -c failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
