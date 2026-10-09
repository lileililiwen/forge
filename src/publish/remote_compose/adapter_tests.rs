//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::publish::remote_compose::model::{Overrides, RemoteComposeAdapter, RemoteConfig};
    use crate::publish::remote_compose::stages::{
        compose_profiles, compose_project_identity, profile_args,
    };
    use crate::publish::{
        CommandResult, PublishAction, PublishAdapter, PublishRequest, RecordingTransport,
        PUBLISH_DEPLOY_TIMEOUT, PUBLISH_SYNC_TIMEOUT, STATUS_DONE,
    };
    use std::path::Path;

    fn config() -> RemoteConfig {
        RemoteConfig::from_overrides(&Overrides::default())
    }

    fn adapter() -> RemoteComposeAdapter {
        RemoteComposeAdapter::new(config())
    }

    fn request(project_id: &str, dir: &Path, action: PublishAction) -> PublishRequest {
        PublishRequest {
            project_id: project_id.to_string(),
            project_dir: dir.to_path_buf(),
            action,
            dry_run: true,
        }
    }

    fn local_project() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("docker-compose.yml"), "services: {}\n").unwrap();
        dir
    }

    #[test]
    fn config_defaults_preserve_the_mac_layout() {
        let cfg = config();
        assert_eq!(cfg.ssh_target, "mac");
        assert_eq!(cfg.remote_root, "/Users/allen/jenkins/projects");
        assert_eq!(cfg.runtime_root, "/Users/allen/jenkins/runtime");
        assert_eq!(cfg.platform_root, "/Users/allen/production/platform");
        assert_eq!(cfg.secrets_root, "/Users/allen/production/secrets");
        assert_eq!(
            cfg.shared_infra_root,
            "/Users/allen/production/shared-infra"
        );
        assert_eq!(cfg.domain, "tooosall.uk");
        assert_eq!(cfg.nav_host, "apps");
        cfg.validate().unwrap();
    }

    #[test]
    fn cloud_roots_are_env_only_and_validate() {
        let cfg = RemoteConfig::from_overrides(&Overrides {
            ssh_target: Some("cloud".to_string()),
            remote_root: Some("/srv/projects".to_string()),
            runtime_root: Some("/srv/runtime".to_string()),
            platform_root: Some("/srv/platform".to_string()),
            secrets_root: Some("/srv/secrets".to_string()),
            shared_infra_root: Some("/srv/shared-infra".to_string()),
            domain: Some("tooosall.uk".to_string()),
            nav_host: Some("apps".to_string()),
        });
        cfg.validate().unwrap();
        assert_eq!(cfg.remote_project_dir("alethefy"), "/srv/projects/alethefy");
        assert_eq!(cfg.remote_runtime_dir("alethefy"), "/srv/runtime/alethefy");
        assert_eq!(cfg.registry_path(), "/srv/runtime/port-registry.json");
    }

    #[test]
    fn relative_or_traversing_roots_are_refused() {
        let cfg = RemoteConfig {
            remote_root: "relative/projects".to_string(),
            ..config()
        };
        assert_eq!(cfg.validate().unwrap_err().code(), "publish-invalid");
        let cfg = RemoteConfig {
            secrets_root: "/srv/../etc".to_string(),
            ..config()
        };
        assert_eq!(cfg.validate().unwrap_err().code(), "publish-invalid");
    }

    #[test]
    fn adapter_id_and_label_are_stable() {
        let adapter = adapter();
        assert_eq!(adapter.id(), "remote-compose");
        assert_eq!(adapter.label(), "Remote compose (generic Docker host)");
        assert_eq!(
            adapter.subdomain("demo"),
            Some("demo.tooosall.uk".to_string())
        );
    }

    #[test]
    fn plan_never_references_a_target_script() {
        let dir = local_project();
        let adapter = adapter();
        let transport = RecordingTransport::new();
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, true).unwrap();
        for stage in PublishAction::All.stages() {
            let plan = adapter.plan(&req, stage.clone()).unwrap();
            let rendered = plan.render();
            for forbidden in [
                "project-ports.sh",
                "project-action.sh",
                "shared-postgres.sh",
                "bash",
            ] {
                assert!(
                    !rendered.contains(forbidden),
                    "stage {} rendered `{forbidden}`: {rendered}",
                    stage.label()
                );
            }
            assert!(rendered.contains("ssh ") || rendered.contains("rsync "));
        }
    }

    #[test]
    fn sync_plan_is_mkdir_plus_rsync() {
        let dir = local_project();
        let plan = adapter()
            .plan(
                &request("alethefy", dir.path(), PublishAction::Sync),
                PublishAction::Sync,
            )
            .unwrap();
        let rendered = plan.render();
        assert!(rendered.contains("ssh mac /bin/mkdir -p /Users/allen/jenkins/projects/alethefy"));
        assert!(rendered.contains("rsync -az --human-readable --exclude .git/"));
        assert!(rendered.contains("--exclude obj/"));
        assert!(rendered.contains("--exclude appendonlydir/"));
        assert!(rendered.contains("*.rdb"));
        assert!(rendered.contains("mac:/Users/allen/jenkins/projects/alethefy/"));
    }

    #[test]
    fn db_plan_starts_shared_postgres_without_a_script() {
        let dir = local_project();
        let plan = adapter()
            .plan(
                &request("alethefy", dir.path(), PublishAction::Db),
                PublishAction::Db,
            )
            .unwrap();
        let rendered = plan.render();
        assert!(rendered.contains("/usr/local/bin/docker compose -f /Users/allen/production/shared-infra/compose.yml -p shared-postgres up -d production-postgres"));
        assert!(rendered.contains("network inspect production-db-network"));
        assert!(rendered.contains("PATH=/Applications/Docker.app/Contents/Resources/bin"));
        assert!(rendered.contains("BUILDKIT_PROGRESS=plain"));
    }

    #[test]
    fn declared_profiles_are_enabled_on_deploy() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    image: demo:latest\n    profiles: [base, app]\n  db:\n    image: postgres:16\n    profiles:\n      - base\n",
        )
        .unwrap();
        assert_eq!(
            compose_profiles(dir.path(), "docker-compose.yml"),
            vec!["app".to_string(), "base".to_string()]
        );
        let adapter = adapter();
        let transport = RecordingTransport::new();
        let req = request("profiled", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, true).unwrap();
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("profile(s) app, base")));
        let rendered = adapter.plan(&req, PublishAction::Deploy).unwrap().render();
        assert!(
            rendered.contains("--profile app --profile base"),
            "deploy was: {rendered}"
        );
    }

    #[test]
    fn missing_or_profileless_compose_means_no_profiles() {
        let dir = local_project();
        assert!(compose_profiles(dir.path(), "docker-compose.yml").is_empty());
        assert!(compose_profiles(dir.path(), "compose.yaml").is_empty());
        assert!(profile_args(&[]).is_empty());
        assert_eq!(
            profile_args(&["app".to_string()]),
            vec!["--profile".to_string(), "app".to_string()]
        );
    }

    #[test]
    fn deploy_without_prepare_refuses_as_target_unavailable() {
        let dir = local_project();
        let err = adapter()
            .plan(
                &request("alethefy", dir.path(), PublishAction::Deploy),
                PublishAction::Deploy,
            )
            .unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
    }

    #[test]
    fn deploy_build_commands_carry_the_long_timeout() {
        use crate::publish::{PUBLISH_DEPLOY_TIMEOUT, PUBLISH_SYNC_TIMEOUT};
        let dir = local_project();
        let adapter = adapter();
        let transport = RecordingTransport::new();
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, true).unwrap();
        let deploy = adapter.plan(&req, PublishAction::Deploy).unwrap();
        let up = deploy
            .commands
            .iter()
            .find(|cmd| cmd.args.iter().any(|arg| arg == "up"))
            .expect("deploy up command");
        assert_eq!(up.timeout, Some(PUBLISH_DEPLOY_TIMEOUT));
        // The timeout is transport metadata only: it never renders
        // into the dry-run plan an operator reviews.
        assert!(!deploy.render().contains("1800"));
        // Sync-stage rsync carries the sync ceiling so multi-GB
        // trees under parallel-fleet contention do not fail closed
        // at the 60s transport default (`fleet-live-rollout`); the
        // mkdir probe keeps the default.
        let sync = adapter.plan(&req, PublishAction::Sync).unwrap();
        let rsync = sync
            .commands
            .iter()
            .find(|cmd| cmd.program == "rsync")
            .expect("sync rsync command");
        assert_eq!(rsync.timeout, Some(PUBLISH_SYNC_TIMEOUT));
        assert!(!sync.render().contains("600"));
    }

    #[test]
    fn dry_run_materialize_renders_a_preview_plan() {
        let dir = local_project();
        let adapter = adapter();
        let transport = RecordingTransport::new();
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, true).unwrap();
        assert_eq!(transport.command_count(), 0);
        let prepare = adapter.plan(&req, PublishAction::Prepare).unwrap();
        let rendered = prepare.render();
        assert!(rendered.contains("scp"));
        assert!(rendered.contains("/Users/allen/jenkins/runtime/port-registry.json.forge-new"));
        assert!(
            rendered.contains("/Users/allen/jenkins/runtime/alethefy/ports.compose.yml.forge-new")
        );
        assert!(rendered.contains("/bin/mv -f"));
        assert!(
            adapter
                .notes()
                .iter()
                .any(|note| note
                    .contains("dry-run: target registry and Compose config were not read"))
        );
    }

    #[test]
    fn apply_materialize_reads_the_target_and_ships_documents() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 0,
            stdout:
                r#"{"projects":{"local:other":{"base":15000,"services":{"web:80/tcp":15000}}}}"#
                    .to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"services":{"alethefy":{"image":"alethefy:latest","ports":[{"published":"8000","target":8000,"protocol":"tcp"}]}}}"#
                .to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: "other|127.0.0.1:15000->80/tcp\n".to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, false).unwrap();
        let artifacts = adapter.staged_artifacts();
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.remote.ends_with("/port-registry.json")));
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.remote.ends_with("/ports.compose.yml")));
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.remote.ends_with("/Caddyfile")));
        for artifact in &artifacts {
            assert!(
                artifact.local.is_file(),
                "{} missing",
                artifact.local.display()
            );
        }
        let deploy = adapter.plan(&req, PublishAction::Deploy).unwrap();
        let rendered = deploy.render();
        assert!(rendered.contains("-p forge-alethefy"));
        assert!(rendered.contains("/Users/allen/jenkins/runtime/alethefy/ports.compose.yml"));
        assert!(rendered.contains("--project-directory /Users/allen/jenkins/projects/alethefy"));
        assert!(rendered.contains("up -d --build --remove-orphans"));
        assert!(rendered.contains("/usr/local/bin/docker compose -f /Users/allen/production/platform/compose.yml up -d --force-recreate"));
        assert!(rendered.contains("--env-file /Users/allen/production/secrets/alethefy/.env"));
        assert!(
            rendered.contains("--env-file /Users/allen/production/secrets/alethefy/.shared-db.env")
        );
        assert!(rendered.contains("-f /Users/allen/jenkins/runtime/alethefy/shared-db.compose.yml"));
        // The shared single-container router reload must be marked
        // exclusive so parallel fleet workers serialise it.
        let reload = deploy
            .commands
            .iter()
            .find(|command| command.label == "router reload")
            .expect("deploy plan carries the platform router reload");
        assert!(reload.exclusive);
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.remote.ends_with("/shared-db.compose.yml")));
    }

    #[test]
    fn shared_db_declaration_adds_the_overlay_and_scale_args() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        let compose = r#"{"services":{"alethefy":{"image":"alethefy:latest","ports":[{"published":"8000","target":8000,"protocol":"tcp"}]}}}"#;
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: compose.to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, false).unwrap();
        assert!(adapter
            .staged_artifacts()
            .iter()
            .any(|artifact| artifact.remote.ends_with("/shared-db.compose.yml")));
        let rendered = adapter.plan(&req, PublishAction::Deploy).unwrap().render();
        assert!(rendered.contains("-f /Users/allen/jenkins/runtime/alethefy/shared-db.compose.yml"));
    }

    #[test]
    fn standalone_database_compose_deploys_without_an_overlay() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        // Crossify-class compose: its own postgres, no application
        // service consuming the shared database.
        let compose = r#"{"services":{"postgres":{"image":"postgres:16-alpine","ports":[{"published":"5437","target":5432,"protocol":"tcp"}]},"minio":{"image":"minio/minio:RELEASE.2024-10-13T13-34-11Z"}}}"#;
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: compose.to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        let req = request("crossify", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, false).unwrap();
        assert!(!adapter
            .staged_artifacts()
            .iter()
            .any(|artifact| artifact.remote.ends_with("/shared-db.compose.yml")));
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("standalone database")));
        let rendered = adapter.plan(&req, PublishAction::Deploy).unwrap().render();
        assert!(!rendered.contains("shared-db.compose.yml"));
        assert!(!rendered.contains("--scale"));
    }

    #[test]
    fn missing_target_registry_bootstraps_a_new_registry() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: "cat: no such file".to_string(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"services":{"alethefy":{"image":"alethefy:latest","ports":[{"published":"8000","target":8000,"protocol":"tcp"}]}}}"#
                .to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        adapter
            .materialize(
                &request("alethefy", dir.path(), PublishAction::All),
                &transport,
                false,
            )
            .unwrap();
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("bootstrapping a new registry")));
        assert!(adapter
            .staged_artifacts()
            .iter()
            .any(|artifact| artifact.remote.ends_with("/port-registry.json")));
    }

    #[test]
    fn unreadable_target_registry_is_a_typed_target_failure() {
        let dir = local_project();
        let adapter = adapter();
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: "cat: permission denied".to_string(),
        }));
        let err = adapter
            .materialize(
                &request("alethefy", dir.path(), PublishAction::All),
                &transport,
                false,
            )
            .unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
        assert!(err.to_string().contains("port-registry.json"));
    }

    #[test]
    fn absent_target_compose_falls_back_to_the_local_checkout() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"projects":{}}"#.to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: "open /remote/dir/docker-compose.yml: no such file or directory".to_string(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        adapter
            .materialize(
                &request("alethefy", dir.path(), PublishAction::All),
                &transport,
                false,
            )
            .unwrap();
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("used the local checkout for port and overlay rendering")));
        assert!(adapter
            .staged_artifacts()
            .iter()
            .any(|artifact| artifact.remote.ends_with("/ports.compose.yml")));
    }

    #[test]
    fn captured_target_output_is_redacted_before_it_is_rendered() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"projects":{}}"#.to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"services":{"alethefy":{"image":"alethefy:latest","environment":{"APP_PASSWORD":"password=letmein"},"ports":[{"published":"8000","target":8000,"protocol":"tcp"}]}}}"#
                .to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        adapter
            .materialize(
                &request("alethefy", dir.path(), PublishAction::All),
                &transport,
                false,
            )
            .unwrap();
        for note in adapter.notes() {
            assert!(!note.contains("letmein"));
        }
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("allocating from the registry only")));
    }

    #[test]
    fn classification_reports_typed_recovery_hints() {
        let dir = local_project();
        let adapter = adapter();
        let req = request("alethefy", dir.path(), PublishAction::Sync);
        let plan = adapter.plan(&req, PublishAction::Sync).unwrap();
        let ok = CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert_eq!(adapter.classify(&plan, &ok).status, STATUS_DONE);
        let unreachable = CommandResult {
            status: 255,
            stdout: String::new(),
            stderr: "ssh: Could not resolve hostname mac".to_string(),
        };
        let classification = adapter.classify(&plan, &unreachable);
        assert_eq!(classification.status, "failed");
        assert!(classification
            .recovery
            .iter()
            .any(|hint| hint.contains("mac")));
    }

    #[test]
    fn compose_project_identity_is_the_forge_namespace() {
        assert_eq!(compose_project_identity("alethefy"), "forge-alethefy");
        assert_ne!(compose_project_identity("alethefy"), "jenkins-alethefy");
    }

    #[test]
    fn provider_container_identity_still_uses_the_revision_suffix() {
        assert_eq!(
            crate::publish::providers::compose_project_name(
                "alethefy",
                "0123456789abcdef0123456789abcdef01234567",
            ),
            "forge-alethefy-0123456789ab"
        );
    }
}
