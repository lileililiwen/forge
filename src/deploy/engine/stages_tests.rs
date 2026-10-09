//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    use crate::core::manifest::Manifest;
    use crate::deploy::engine::model::AdapterResponse;
    use crate::deploy::engine::stages::{
        apply_deploy, list_deploys, load_config, observe_deploy, prepare_deploy, wait_with_timeout,
    };
    use crate::deploy::{
        deploy_config_from_manifest, load_deploy_state, DeployAdapterConfig, DeployConfig,
        DeployRequest, STATUS_RUNNING, STATUS_UNKNOWN,
    };

    fn write_minimal_project(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("forge.yaml"),
            "schema: 1\nproject:\n  id: dep-test\n  name: dep-test\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n    - name: vps\n      kind: docker-compose\n      service: app\n  health:\n    kind: docker\n    service: app\n",
        )
        .unwrap();
        fs::write(
            dir.join("docker-compose.yml"),
            "services:\n  app:\n    image: app:0.1.0\n",
        )
        .unwrap();
        fs::write(dir.join("README.md"), "v1\n").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("init")
            .arg("-q")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("user.email")
            .arg("forge@example.com")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("user.name")
            .arg("Forge Test")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("init.defaultBranch")
            .arg("main")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("checkout")
            .arg("-q")
            .arg("-b")
            .arg("main")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("add")
            .arg("--")
            .arg("forge.yaml")
            .arg("docker-compose.yml")
            .arg("README.md")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("commit")
            .arg("-q")
            .arg("-m")
            .arg("initial")
            .output()
            .unwrap();
    }

    #[test]
    fn prepare_returns_ready_plan_for_default_target() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: false,
            dry_run: true,
        };
        let plan = prepare_deploy(tmp.path(), &manifest, &config, &req).unwrap();
        assert!(plan.ready);
        assert_eq!(plan.target.name, "home");
        assert!(plan.artifact.is_some());
    }

    #[test]
    fn prepare_refuses_unknown_target() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: "unknown".to_string(),
            confirm: false,
            dry_run: true,
        };
        let err = prepare_deploy(tmp.path(), &manifest, &config, &req).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn apply_refuses_without_confirm() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: false,
            dry_run: true,
        };
        let err = apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: "true".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn list_deploys_is_empty_for_fresh_project() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let entries = list_deploys(tmp.path(), "dep-test").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn observe_refuses_without_prior_state() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: true,
            dry_run: true,
        };
        let err = observe_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: "true".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "deploy-target-stale");
    }

    #[test]
    fn ssh_target_is_unavailable_boundary() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  targets:\n    - name: vps\n      kind: ssh\n",
        )
        .expect("manifest");
        let config = deploy_config_from_manifest(&manifest).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: false,
            dry_run: true,
        };
        // Prepare surfaces the boundary so a manifest cannot
        // silently fall through to a planned adapter.
        let err = prepare_deploy(Path::new("."), &manifest, &config, &req).unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
    }

    /// Write an executable adapter script and return its path.
    fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    const ENVELOPE_OK: &str = r#"{"contract":"forge-deploy-executor/0.1.0","apply_status":"delivered","apply_note":"applied","apply_evidence":[],"observation_status":"running","observation_detail":"up","observation_evidence":[],"recovery":[],"source":"fixture-executor/0.1.0","source_revision":"cafe1234"}"#;

    fn envelope_script(dir: &Path, name: &str, envelope: &str, exit_code: i32) -> PathBuf {
        write_script(
            dir,
            name,
            &format!("#!/bin/sh\ncat >/dev/null\nprintf '%s' '{envelope}'\nexit {exit_code}\n"),
        )
    }

    fn apply_request(manifest: &Manifest, config: &DeployConfig) -> DeployRequest {
        DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: true,
            dry_run: false,
        }
    }

    #[test]
    fn executor_envelope_requires_namespaced_contract() {
        // The frozen discriminator is `forge-deploy-executor/0.1.0`.
        let good = AdapterResponse::parse(ENVELOPE_OK).expect("conformant envelope");
        assert_eq!(good.apply_status, "delivered");
        assert_eq!(good.source.as_deref(), Some("fixture-executor/0.1.0"));
        assert_eq!(good.source_revision.as_deref(), Some("cafe1234"));
        // The bare pre-namespacing `0.1.0` value must be
        // refused and the refusal names both sides.
        let stale = ENVELOPE_OK.replace(
            "\"contract\":\"forge-deploy-executor/0.1.0\"",
            "\"contract\":\"0.1.0\"",
        );
        let err = AdapterResponse::parse(&stale).unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
        let msg = err.to_string();
        assert!(
            msg.contains("forge-deploy-executor/0.1.0") && msg.contains("`0.1.0`"),
            "refusal must name the observed and expected contract: {msg}"
        );
        let err = AdapterResponse::parse("not json at all").unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
    }

    #[test]
    fn attribution_evidence_names_executor_and_revision() {
        let response = AdapterResponse::parse(ENVELOPE_OK).unwrap();
        assert_eq!(
            response.attribution("forge-deployer"),
            "executor=fixture-executor/0.1.0@cafe1234"
        );
        // An envelope without self-identification attributes
        // to the configured binary with an explicit unknown
        // revision — never a claimed version.
        let bare = ENVELOPE_OK.replace(
            ",\"source\":\"fixture-executor/0.1.0\",\"source_revision\":\"cafe1234\"",
            "",
        );
        let response = AdapterResponse::parse(&bare).unwrap();
        assert_eq!(
            response.attribution("/opt/bin/custom-deployer"),
            "executor=custom-deployer@unknown"
        );
    }

    #[test]
    fn non_zero_exit_with_envelope_records_failed_stage_preserving_prior_state() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = apply_request(&manifest, &config);
        // First apply succeeds: the delivered state is persisted.
        let good = envelope_script(tmp.path(), "good.sh", ENVELOPE_OK, 0);
        let report = apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: good.display().to_string(),
            },
        )
        .unwrap();
        assert!(report.healthy);
        let state_path = Path::new(&report.state_path);
        let state_bytes_before = fs::read(state_path).unwrap();
        assert!(String::from_utf8_lossy(&state_bytes_before).contains("delivered"));
        // Second apply: the adapter exits non-zero WITH a
        // contract-conformant envelope naming a failed stage.
        // The failure is recorded exactly as named and the
        // prior DeployState bytes stay untouched.
        let failing = ENVELOPE_OK
            .replace(
                "\"apply_status\":\"delivered\"",
                "\"apply_status\":\"failed\"",
            )
            .replace(
                "\"apply_note\":\"applied\"",
                "\"apply_note\":\"job registration refused\"",
            );
        let bad = envelope_script(tmp.path(), "bad.sh", &failing, 1);
        let report = apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: bad.display().to_string(),
            },
        )
        .unwrap();
        assert!(!report.healthy);
        let apply = report
            .stages
            .iter()
            .find(|s| s.stage == "apply")
            .expect("apply stage");
        assert_eq!(apply.status, "failed");
        assert_eq!(apply.note, "job registration refused");
        assert!(apply
            .evidence
            .iter()
            .any(|e| e.starts_with("executor=fixture-executor/0.1.0@")));
        let state_bytes_after = fs::read(state_path).unwrap();
        assert_eq!(
            state_bytes_before, state_bytes_after,
            "a failed stage must never overwrite the prior state"
        );
    }

    #[test]
    fn non_zero_delivered_claim_is_downgraded_to_failed() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = apply_request(&manifest, &config);
        // A contradictory envelope — claiming `delivered`
        // while exiting non-zero — records `failed`: the exit
        // status is authoritative, so a broken executor can
        // never masquerade as a success.
        let contradictory = envelope_script(tmp.path(), "contradictory.sh", ENVELOPE_OK, 7);
        let report = apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: contradictory.display().to_string(),
            },
        )
        .unwrap();
        let apply = report
            .stages
            .iter()
            .find(|s| s.stage == "apply")
            .expect("apply stage");
        assert_eq!(apply.status, "failed");
        assert!(
            apply.note.contains("authoritative"),
            "note must name the downgrade reason: {}",
            apply.note
        );
        assert!(!report.healthy);
        // Nothing was persisted for the downgraded run.
        assert!(!tmp
            .path()
            .join(".forge/deploy")
            .join(&manifest.project.id)
            .exists());
    }

    #[test]
    fn non_zero_without_envelope_stays_unavailable() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = apply_request(&manifest, &config);
        let noisy = write_script(
            tmp.path(),
            "noisy.sh",
            "#!/bin/sh\ncat >/dev/null\necho 'docker: cannot connect to daemon' >&2\nexit 1\n",
        );
        let err = apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: noisy.display().to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
        assert!(err.to_string().contains("cannot connect to daemon"));
    }

    #[test]
    fn observe_invokes_the_read_only_observe_verb() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = apply_request(&manifest, &config);
        let argv_log = tmp.path().join("argv.log");
        // The fixture records each invocation's argv and
        // always answers with the conformant envelope.
        let script = write_script(
            tmp.path(),
            "logger.sh",
            &format!(
                "#!/bin/sh\necho \"$@\" >> '{}'\ncat >/dev/null\nprintf '%s' '{ENVELOPE_OK}'\n",
                argv_log.display()
            ),
        );
        let adapters = DeployAdapterConfig {
            deployer_bin: script.display().to_string(),
        };
        let report = apply_deploy(tmp.path(), &manifest, &config, &req, &adapters).unwrap();
        assert!(!fs::read(&argv_log).unwrap().is_empty());
        observe_deploy(tmp.path(), &manifest, &config, &req, &adapters).unwrap();
        let log = String::from_utf8_lossy(&fs::read(&argv_log).unwrap()).to_string();
        let lines: Vec<&str> = log.lines().collect();
        assert_eq!(lines.len(), 2, "one apply plus one observe call: {lines:?}");
        assert!(lines[0].starts_with("apply "), "first call: {}", lines[0]);
        assert!(
            lines[1].starts_with("observe ")
                && lines[1].contains("--deploy-id")
                && lines[1].contains(&report.identity.id),
            "observe must use the read-only verb with the deploy id: {}",
            lines[1]
        );
        assert!(
            !lines[1].contains("--artifact"),
            "observe must never carry an apply artifact: {}",
            lines[1]
        );
    }

    #[test]
    fn deploy_executor_runs_from_project_root() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = apply_request(&manifest, &config);
        let marker = tmp.path().join("executor-cwd");
        let script = write_script(
            tmp.path(),
            "cwd.sh",
            &format!(
                "#!/bin/sh\npwd > '{}'\ncat >/dev/null\nprintf '%s' '{}'\n",
                marker.display(),
                ENVELOPE_OK
            ),
        );
        apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: script.display().to_string(),
            },
        )
        .unwrap();
        let actual = fs::read_to_string(marker).unwrap();
        assert_eq!(actual.trim(), tmp.path().to_string_lossy());
    }

    #[test]
    fn unknown_observation_preserves_last_good_running() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = apply_request(&manifest, &config);
        let good = envelope_script(tmp.path(), "good.sh", ENVELOPE_OK, 0);
        let adapters = DeployAdapterConfig {
            deployer_bin: good.display().to_string(),
        };
        let report = apply_deploy(tmp.path(), &manifest, &config, &req, &adapters).unwrap();
        let state = load_deploy_state(Path::new(&report.state_path)).unwrap();
        assert_eq!(state.current_state(), STATUS_RUNNING);
        assert!(state.last_observed_running.is_some());
        // The deployment reports a state outside the mapping
        // table: observe returns `unknown` while the previous
        // good observation remains the recorded history entry.
        let unreachable = ENVELOPE_OK
            .replace(
                "\"observation_status\":\"running\"",
                "\"observation_status\":\"unheard-of-state\"",
            )
            .replace(
                "\"observation_detail\":\"up\"",
                "\"observation_detail\":\"status outside the mapping table\"",
            );
        let flaky = envelope_script(tmp.path(), "flaky.sh", &unreachable, 0);
        let adapters = DeployAdapterConfig {
            deployer_bin: flaky.display().to_string(),
        };
        let report = observe_deploy(tmp.path(), &manifest, &config, &req, &adapters).unwrap();
        assert!(!report.healthy);
        assert_eq!(report.stages[0].status, STATUS_UNKNOWN);
        let state = load_deploy_state(Path::new(&report.state_path)).unwrap();
        assert_eq!(state.current_state(), STATUS_UNKNOWN);
        let preserved = state
            .last_observed_running
            .expect("the prior good observation must survive an unknown re-observation");
        assert_eq!(preserved.status, STATUS_RUNNING);
        assert_eq!(preserved.detail, "up");
    }

    #[test]
    fn legacy_state_without_last_good_observation_still_loads() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("state.json");
        fs::write(
            &path,
            r#"{"contract":"0.1.0","identity":{"project_id":"app","target":"home","source_revision":"deadbeef","id":"app-home-deadbeef"},"target":{"name":"home","kind":"local","host":null,"user":null,"path":null,"service":null,"note":null},"adapter":"local","artifact":null,"health":null,"stage_outcomes":[],"last_observation":null,"last_run_at":""}"#,
        )
        .unwrap();
        let state = load_deploy_state(&path).unwrap();
        assert!(state.last_observed_running.is_none());
        assert_eq!(state.identity.id, "app-home-deadbeef");
    }

    #[test]
    fn bounded_wait_times_out_on_hanged_adapter() {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn sleep");
        let err = wait_with_timeout(child, std::time::Duration::from_millis(200))
            .expect_err("hanged adapter must not be waited on forever");
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    }
}
