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
        DeployRequest, DeployTargetSpec, STATUS_RUNNING, STATUS_UNKNOWN,
    };
    use crate::extension::kinds::DEPLOY_TARGET_KINDS;

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

    /// The project's committed HEAD. The shared delivery rule binds
    /// the evidence to the revision that is about to be delivered.
    fn head_revision(dir: &Path) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .unwrap();
        assert!(out.status.success(), "git rev-parse HEAD");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// Write Gate evidence that satisfies the shared delivery rule for
    /// the project's committed HEAD.
    fn write_gate_evidence(dir: &Path, id: &str, revision: &str) {
        let evidence = serde_json::json!({
            "contract": crate::gate::GATE_CONTRACT_VERSION,
            "project_id": id,
            "runtime": "forge",
            "profile": "full",
            "revision": revision,
            "aggregate": "passed",
            "checks": [],
            "observed_at": "2026-10-18T00:00:00Z",
            "dry_run": false
        });
        let path = dir.join(".forge/gate").join(id).join("evidence.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(&evidence).unwrap()).unwrap();
    }

    /// The project the deploy engine accepts, with Gate evidence that
    /// satisfies the shared delivery rule.
    fn write_gated_project(dir: &Path) {
        write_minimal_project(dir);
        write_gate_evidence(dir, "dep-test", &head_revision(dir));
    }

    #[test]
    fn prepare_returns_ready_plan_for_default_target() {
        let tmp = TempDir::new().unwrap();
        write_gated_project(tmp.path());
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
    fn prepare_reports_an_unsatisfied_ship_gate() {
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
        assert!(!plan.ready, "an unsatisfied rule leaves the plan not ready");
        assert!(!plan.ship_gate.satisfied);
        assert!(
            plan.note
                .contains("Forge Gate evidence is absent for `dep-test`"),
            "the note names the missing evidence: {}",
            plan.note
        );
    }

    #[test]
    fn apply_refuses_without_gate_evidence() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = apply_request(&manifest, &config);
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
        assert!(
            err.to_string()
                .contains("Forge Gate evidence is absent for `dep-test`"),
            "the refusal names the missing evidence: {}",
            err
        );
        assert!(
            !tmp.path().join(".forge/deploy").exists(),
            "a refused apply writes no delivery state"
        );
    }

    #[test]
    fn apply_proceeds_with_satisfied_gate_evidence() {
        let tmp = TempDir::new().unwrap();
        write_gated_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = apply_request(&manifest, &config);
        let script = envelope_script(tmp.path(), "ok.sh", ENVELOPE_OK, 0);
        let report = apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: script.display().to_string(),
            },
        )
        .unwrap();
        assert!(report.healthy);
    }

    #[test]
    fn a_dry_run_apply_is_not_refused_without_gate_evidence() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: true,
            dry_run: true,
        };
        let script = envelope_script(tmp.path(), "ok.sh", ENVELOPE_OK, 0);
        let report = apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: script.display().to_string(),
            },
        )
        .unwrap();
        assert_eq!(report.stages.len(), 2);
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

    /// A deploy configuration built directly, with one target of
    /// the named kind. `DeployConfig` is public, so the plan has
    /// to be safe on its own.
    fn config_with_kind(kind: &str) -> DeployConfig {
        DeployConfig {
            adapter: kind.to_string(),
            default_target: "vps".to_string(),
            targets: vec![DeployTargetSpec {
                name: "vps".to_string(),
                kind: kind.to_string(),
                host: None,
                user: None,
                path: None,
                service: None,
                note: None,
            }],
            artifact: None,
            health: None,
        }
    }

    /// A manifest naming one deploy target kind.
    fn manifest_with_kind(kind: &str) -> Manifest {
        Manifest::parse(
            Path::new("forge.yaml"),
            format!(
                "schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  default: vps\n  targets:\n    - name: vps\n      kind: {kind}\n"
            )
            .as_bytes(),
        )
        .expect("manifest")
    }

    #[test]
    fn ssh_target_is_refused_by_the_registry_in_one_wording() {
        // The plan must refuse an unwired kind on its own, because
        // `DeployConfig` is public and `prepare_deploy` is a
        // public function.
        let manifest = manifest_with_kind("ssh");
        let config = config_with_kind("ssh");
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: false,
            dry_run: true,
        };
        let err = prepare_deploy(Path::new("."), &manifest, &config, &req).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
        assert!(
            err.to_string().contains(
                "deployment target kind `ssh` is known but not yet wired; v0.1.0 supports `local`, `docker-compose`, `jenkins`, and `mac-runtime`"
            ),
            "the refusal names the shared wording: {err}"
        );
    }

    #[test]
    fn observe_refuses_an_unwired_kind_in_the_same_words_as_the_plan() {
        let manifest = manifest_with_kind("ssh");
        let config = config_with_kind("ssh");
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: true,
            dry_run: true,
        };
        let err = observe_deploy(
            Path::new("."),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: "true".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
        assert!(
            err.to_string().contains(
                "deployment target kind `ssh` is known but not yet wired; v0.1.0 supports `local`, `docker-compose`, `jenkins`, and `mac-runtime`"
            ),
            "the refusal names the shared wording: {err}"
        );
    }

    #[test]
    fn every_registered_kind_is_refused_consistently_by_the_configuration_and_the_plan() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        for entry in DEPLOY_TARGET_KINDS {
            let manifest = manifest_with_kind(entry.kind);
            let configured = deploy_config_from_manifest(&manifest);
            let config = config_with_kind(entry.kind);
            let req = DeployRequest {
                project_id: manifest.project.id.clone(),
                target: config.default_target.clone(),
                confirm: false,
                dry_run: true,
            };
            let planned = prepare_deploy(tmp.path(), &manifest, &config, &req);
            if entry.wired {
                assert!(
                    configured.is_ok(),
                    "the wired kind `{}` must be accepted by the configuration validator: {:?}",
                    entry.kind,
                    configured.err()
                );
                assert!(
                    planned.is_ok(),
                    "the wired kind `{}` must be accepted by the plan: {:?}",
                    entry.kind,
                    planned.err()
                );
                continue;
            }
            let err = configured.expect_err("an unwired kind must be refused by the configuration");
            assert_eq!(err.code(), "deploy-invalid");
            let plan_err = planned.expect_err("an unwired kind must be refused by the plan");
            assert_eq!(plan_err.code(), "deploy-invalid");
            assert_eq!(
                err.to_string(),
                plan_err.to_string(),
                "the configuration and the plan must refuse the kind in the same words"
            );
            assert!(
                err.to_string().contains(&format!(
                    "deployment target kind `{}` is known but not yet wired",
                    entry.kind
                )),
                "the refusal names the shared wording: {err}"
            );
        }
        // An unknown kind is refused by name at both entry points.
        let manifest = manifest_with_kind("carrier-pigeon");
        let err = deploy_config_from_manifest(&manifest)
            .expect_err("an unknown kind must be refused by the configuration");
        assert_eq!(err.code(), "deploy-invalid");
        assert!(
            err.to_string().contains(
                "deployment target kind `carrier-pigeon` is not supported; expected one of `local`, `docker-compose`, `jenkins`, and `mac-runtime`"
            ),
            "the refusal names the supported kinds: {err}"
        );
        let config = config_with_kind("carrier-pigeon");
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: false,
            dry_run: true,
        };
        let plan_err = prepare_deploy(tmp.path(), &manifest, &config, &req)
            .expect_err("an unknown kind must be refused by the plan");
        assert_eq!(
            err.to_string(),
            plan_err.to_string(),
            "the configuration and the plan must refuse the kind in the same words"
        );
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
        // The executor family is `forge-deploy-executor` at the
        // required major `0`.
        let good = AdapterResponse::parse(ENVELOPE_OK).expect("conformant envelope");
        assert_eq!(good.apply_status, "delivered");
        assert_eq!(
            good.source.as_ref().map(|s| s.family.as_str()),
            Some("fixture-executor")
        );
        assert_eq!(
            good.source.as_ref().map(|s| s.version()),
            Some("0.1.0".to_string())
        );
        assert_eq!(good.source_revision.as_deref(), Some("cafe1234"));
        // The bare pre-namespacing `0.1.0` value carries no
        // family and cannot be read as a contract.
        let stale = ENVELOPE_OK.replace(
            "\"contract\":\"forge-deploy-executor/0.1.0\"",
            "\"contract\":\"0.1.0\"",
        );
        let err = AdapterResponse::parse(&stale).unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
        let msg = err.to_string();
        assert!(
            msg.contains("`0.1.0`")
                && msg.contains("does not name a contract family")
                && msg.contains("family/major.minor.patch"),
            "the refusal must name the offending value and the expected shape: {msg}"
        );
        let err = AdapterResponse::parse("not json at all").unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
    }

    #[test]
    fn executor_envelope_accepts_a_newer_minor_of_the_same_family() {
        // The shared rule: the family and the major decide. A
        // newer minor is an addition the old reader can ignore.
        let newer = ENVELOPE_OK.replace(
            "\"contract\":\"forge-deploy-executor/0.1.0\"",
            "\"contract\":\"forge-deploy-executor/0.2.0\"",
        );
        let response = AdapterResponse::parse(&newer).expect("a newer minor is conformant");
        assert_eq!(response.apply_status, "delivered");
    }

    #[test]
    fn executor_envelope_refuses_a_different_major_or_family() {
        let major = ENVELOPE_OK.replace(
            "\"contract\":\"forge-deploy-executor/0.1.0\"",
            "\"contract\":\"forge-deploy-executor/1.0.0\"",
        );
        let err = AdapterResponse::parse(&major).unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
        let msg = err.to_string();
        assert!(
            msg.contains("fixture-executor declares the contract `forge-deploy-executor/1.0.0`")
                && msg.contains("the required `forge-deploy-executor/0.1.0`")
                && msg.contains("the major versions differ (1 and 0)"),
            "the refusal must name the extension, both contracts and the reason: {msg}"
        );
        let family = ENVELOPE_OK.replace(
            "\"contract\":\"forge-deploy-executor/0.1.0\"",
            "\"contract\":\"forge-deployer/0.1.0\"",
        );
        let err = AdapterResponse::parse(&family).unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
        let msg = err.to_string();
        assert!(
            msg.contains("the families differ (`forge-deployer` and `forge-deploy-executor`)"),
            "the refusal must name both families: {msg}"
        );
    }

    #[test]
    fn a_malformed_self_identification_is_refused_and_an_absent_one_is_allowed() {
        // A malformed claim is worse than no claim at all.
        let malformed = ENVELOPE_OK.replace(
            "\"source\":\"fixture-executor/0.1.0\"",
            "\"source\":\"0.1.0\"",
        );
        let err = AdapterResponse::parse(&malformed).unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
        let msg = err.to_string();
        assert!(
            msg.contains("self-identification `0.1.0`")
                && msg.contains("does not name a contract family"),
            "the refusal must name the offending value: {msg}"
        );
        // An absent claim is not a claim: the adapter is accepted
        // and the evidence names the configured binary.
        let absent = ENVELOPE_OK.replace(
            ",\"source\":\"fixture-executor/0.1.0\",\"source_revision\":\"cafe1234\"",
            "",
        );
        let response = AdapterResponse::parse(&absent).expect("an absent claim is allowed");
        assert!(response.source.is_none());
        assert_eq!(
            response.attribution("forge-deployer"),
            "executor=forge-deployer@unknown"
        );
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
        write_gated_project(tmp.path());
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
        write_gated_project(tmp.path());
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
        write_gated_project(tmp.path());
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
        write_gated_project(tmp.path());
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
        write_gated_project(tmp.path());
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
        write_gated_project(tmp.path());
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
