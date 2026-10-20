//! The fleet liveness surface's own suite.
//!
//! Split out of `online.rs` so the surface itself stays under the
//! repository's 1000-line source cap.

#[cfg(test)]
pub(super) mod tests {
    use crate::fleet::online::{
        build_report, cat_caddyfile_command, classify, docker_ps_command, match_container,
        parse_caddyfile_hosts, route_for_project, LivenessCandidate, LivenessSummary, ProbeFacts,
        Verdict,
    };
    use crate::publish::inventory::InventoryClassification;
    use crate::runtime::{HostModel, SHARED_HOST_REASON};

    fn facts(
        container: Option<&str>,
        route: Option<&str>,
        status: Option<u16>,
        body: Option<&str>,
    ) -> ProbeFacts {
        ProbeFacts {
            container: container.map(str::to_owned),
            route: route.map(str::to_owned),
            http_status: status,
            body: body.map(str::to_owned),
            probe_error: None,
        }
    }

    #[test]
    fn classify_marks_no_route_when_route_absent() {
        let (verdict, status, detail) =
            classify(facts(Some("forge-alethefy-abc (Up)"), None, None, None));
        assert_eq!(verdict, Verdict::NoRoute);
        assert_eq!(status, None);
        assert!(detail.is_none());
    }

    #[test]
    fn classify_marks_not_deployed_when_container_absent() {
        let (verdict, _status, detail) =
            classify(facts(None, Some("alethefy.tooosall.uk"), None, None));
        assert_eq!(verdict, Verdict::NotDeployed);
        assert_eq!(
            detail.as_deref(),
            Some("no compose container running on target")
        );
    }

    #[test]
    fn classify_marks_down_on_gateway_error() {
        for status in [502u16, 503, 504] {
            let (verdict, got, detail) = classify(facts(
                Some("forge-alethefy-abc (Up)"),
                Some("alethefy.tooosall.uk"),
                Some(status),
                Some(""),
            ));
            assert_eq!(verdict, Verdict::Down, "status {status}");
            assert_eq!(got, Some(status));
            assert!(detail.unwrap_or_default().contains("gateway error"));
        }
    }

    #[test]
    fn classify_marks_down_on_connection_failure() {
        let (verdict, got, detail) = classify(facts(
            Some("forge-alethefy-abc (Up)"),
            Some("alethefy.tooosall.uk"),
            Some(0),
            Some(""),
        ));
        assert_eq!(verdict, Verdict::Down);
        assert_eq!(got, Some(0));
        assert!(detail.unwrap_or_default().contains("connection failed"));
    }

    #[test]
    fn classify_marks_down_on_router_fallback_body() {
        let body = "Unknown application hostname: alethefy.tooosall.uk".to_string();
        let (verdict, status, detail) = classify(facts(
            Some("forge-alethefy-abc (Up)"),
            Some("alethefy.tooosall.uk"),
            Some(404),
            Some(&body),
        ));
        assert_eq!(verdict, Verdict::Down);
        assert_eq!(status, Some(404));
        assert!(detail.unwrap_or_default().contains("router fallback"));
    }

    #[test]
    fn classify_marks_online_for_application_404() {
        let body = "<html><body>Not Found</body></html>".to_string();
        let (verdict, status, detail) = classify(facts(
            Some("forge-alethefy-abc (Up)"),
            Some("alethefy.tooosall.uk"),
            Some(404),
            Some(&body),
        ));
        assert_eq!(verdict, Verdict::Online);
        assert_eq!(status, Some(404));
        assert!(detail.is_none());
    }

    #[test]
    fn classify_marks_unavailable_when_probe_error_present() {
        let (verdict, _status, detail) = classify(ProbeFacts {
            container: Some("forge-alethefy-abc".to_string()),
            route: Some("alethefy.tooosall.uk".to_string()),
            http_status: None,
            body: None,
            probe_error: Some("curl: (6) Could not resolve host".to_string()),
        });
        assert_eq!(verdict, Verdict::Unavailable);
        assert!(detail.unwrap_or_default().contains("Could not resolve"));
    }

    #[test]
    fn classify_redacts_secrets_in_probe_error_detail() {
        let (verdict, _status, detail) = classify(ProbeFacts {
            container: None,
            route: Some("alethefy.tooosall.uk".to_string()),
            http_status: None,
            body: None,
            probe_error: Some("Authorization: Bearer token=abc123secretXYZ".to_string()),
        });
        assert_eq!(verdict, Verdict::Unavailable);
        let detail = detail.unwrap_or_default();
        assert!(!detail.contains("abc123secretXYZ"));
        assert!(detail.contains("[REDACTED]"));
    }

    #[test]
    fn parse_caddyfile_returns_hosts_and_skips_nav() {
        let content = "{\n\tauto_https off\n}\n\nhttp://apps.tooosall.uk {\n\troot * /srv\n\tfile_server\n}\n\nhttp://alethefy.tooosall.uk {\n\treverse_proxy host.docker.internal:17700\n}\n\nhttp://crossalheart.tooosall.uk {\n\treverse_proxy host.docker.internal:17710\n}\n\n:80 {\n\trespond \"Unknown application hostname: {http.request.host}\" 404\n}\n";
        let hosts = parse_caddyfile_hosts(content, "tooosall.uk", "apps").unwrap();
        assert_eq!(
            hosts,
            vec![
                "alethefy.tooosall.uk".to_string(),
                "crossalheart.tooosall.uk".to_string()
            ]
        );
    }

    #[test]
    fn parse_caddyfile_handles_empty_file() {
        let hosts = parse_caddyfile_hosts("", "tooosall.uk", "apps").unwrap();
        assert!(hosts.is_empty());
    }

    #[test]
    fn parse_caddyfile_excludes_only_fallback_when_present() {
        let content = ":80 {\n\trespond \"Unknown application hostname\" 404\n}\n";
        let hosts = parse_caddyfile_hosts(content, "tooosall.uk", "apps").unwrap();
        assert!(hosts.is_empty());
    }

    #[test]
    fn route_for_project_matches_exact_host() {
        let hosts = vec![
            "alethefy.tooosall.uk".to_string(),
            "forge.tooosall.uk".to_string(),
        ];
        assert_eq!(
            route_for_project(&hosts, "alethefy", "tooosall.uk").as_deref(),
            Some("alethefy.tooosall.uk")
        );
        assert_eq!(
            route_for_project(&hosts, "forge", "tooosall.uk").as_deref(),
            Some("forge.tooosall.uk")
        );
        assert!(route_for_project(&hosts, "alethefy-staging", "tooosall.uk").is_none());
    }

    #[test]
    fn match_container_finds_project_prefix() {
        let output =
            "forge-alethefy-0123456789ab\tUp 5m\nforge-crossalheart-abcdef012345\tUp 10m\n";
        assert_eq!(
            match_container(output, "alethefy").as_deref(),
            Some("forge-alethefy-0123456789ab (Up 5m)")
        );
        assert!(match_container(output, "absent").is_none());
    }

    #[test]
    fn match_container_bounds_long_strings() {
        let long_status = "x".repeat(500);
        let output = format!("forge-alethefy-0123456789ab\t{long_status}\n");
        let found = match_container(&output, "alethefy").expect("container should match");
        assert!(
            found.len() <= 200,
            "container string should be bounded: {}",
            found.len()
        );
        assert!(found.ends_with('…'));
    }

    #[test]
    fn match_container_redacts_secrets() {
        let output = "forge-alethefy-0123456789ab\tUp password=abc123secretXYZ\n";
        let found = match_container(output, "alethefy").expect("container should match");
        assert!(!found.contains("abc123secretXYZ"));
        assert!(found.contains("[REDACTED]"));
    }

    #[test]
    fn cat_caddyfile_command_matches_publish_shape() {
        let argv = cat_caddyfile_command("mac", "/srv/platform/Caddyfile");
        assert_eq!(argv[0], "ssh");
        assert_eq!(argv[1], "mac");
        assert_eq!(argv[2], "cat");
        assert_eq!(argv[3], "/srv/platform/Caddyfile");
    }

    #[test]
    fn cat_caddyfile_command_invokes_stub_directly_for_path_target() {
        // A test stub named in `FORGE_PUBLISH_SSH_TARGET` is a local
        // executable path; the probe must exec it directly instead
        // of wrapping it in a real `ssh <path> ...` argv.
        let argv = cat_caddyfile_command("/tmp/fake/ssh", "/srv/platform/Caddyfile");
        assert_eq!(argv[0], "/tmp/fake/ssh");
        assert_eq!(argv[1], "cat");
        assert_eq!(argv[2], "/srv/platform/Caddyfile");
    }

    #[test]
    fn docker_ps_command_matches_publish_shape() {
        let argv = docker_ps_command("mac");
        assert_eq!(argv[0], "ssh");
        assert_eq!(argv[1], "mac");
        // Mirror the publish adapter's PATH + BUILDKIT shape so the
        // same Docker Desktop helper directory reaches `docker ps`.
        assert!(argv
            .iter()
            .any(|arg| arg.to_string_lossy().starts_with("PATH=")));
        assert!(argv.contains(&std::ffi::OsString::from(
            crate::publish::remote_compose::DOCKER_BIN
        )));
        assert!(argv.contains(&std::ffi::OsString::from("--format")));
    }

    #[test]
    fn docker_ps_command_invokes_stub_directly_for_path_target() {
        let argv = docker_ps_command("/tmp/fake/ssh");
        assert_eq!(argv[0], "/tmp/fake/ssh");
        assert!(argv
            .iter()
            .any(|arg| arg.to_string_lossy().starts_with("PATH=")));
        assert!(argv.contains(&std::ffi::OsString::from(
            crate::publish::remote_compose::DOCKER_BIN
        )));
    }

    #[test]
    fn summary_all_probed_online_requires_online_count() {
        let mut summary = LivenessSummary {
            online: 3,
            ..Default::default()
        };
        assert!(summary.all_probed_online());
        summary.no_route = 1;
        assert!(!summary.all_probed_online());
        summary.no_route = 0;
        summary.skipped = 5;
        // Skipped entries do not change the exit code.
        assert!(summary.all_probed_online());
    }

    #[test]
    fn summary_zero_online_is_not_all_probed_online() {
        // An empty roster is a typed refusal, not an "all online" run.
        let summary = LivenessSummary::default();
        assert!(!summary.all_probed_online());
    }

    #[test]
    fn build_report_counts_every_state() {
        let own = HostModel::OwnProcess;
        let entries = vec![
            LivenessCandidate {
                id: "alpha".to_string(),
                classification: InventoryClassification::ComposeReady,
                host_model: Some(own),
                facts: facts(
                    Some("forge-alpha-1 (Up)"),
                    Some("alpha.tooosall.uk"),
                    Some(200),
                    Some("ok"),
                ),
            },
            LivenessCandidate {
                id: "beta".to_string(),
                classification: InventoryClassification::ComposeReady,
                host_model: Some(own),
                facts: facts(
                    Some("forge-beta-1 (Up)"),
                    Some("beta.tooosall.uk"),
                    Some(502),
                    Some(""),
                ),
            },
            LivenessCandidate {
                id: "gamma".to_string(),
                classification: InventoryClassification::ComposeReady,
                host_model: Some(own),
                facts: facts(None, Some("gamma.tooosall.uk"), None, None),
            },
            LivenessCandidate {
                id: "delta".to_string(),
                classification: InventoryClassification::ComposeReady,
                host_model: Some(own),
                facts: facts(Some("forge-delta-1 (Up)"), None, None, None),
            },
            LivenessCandidate {
                id: "epsilon".to_string(),
                classification: InventoryClassification::ComposeMissing,
                host_model: Some(own),
                facts: facts(None, None, None, None),
            },
            LivenessCandidate {
                id: "zeta".to_string(),
                classification: InventoryClassification::ComposeReady,
                host_model: Some(own),
                facts: ProbeFacts {
                    container: Some("forge-zeta-1".to_string()),
                    route: Some("zeta.tooosall.uk".to_string()),
                    http_status: None,
                    body: None,
                    probe_error: Some("curl: (6) Could not resolve host".to_string()),
                },
            },
        ];
        let report = build_report(
            entries,
            "inventory:/tmp/inv.json".to_string(),
            "mac".to_string(),
            "tooosall.uk".to_string(),
            "2026-09-28T00:00:00Z".to_string(),
        );
        assert_eq!(report.summary.online, 1);
        assert_eq!(report.summary.down, 1);
        assert_eq!(report.summary.not_deployed, 1);
        assert_eq!(report.summary.no_route, 1);
        assert_eq!(report.summary.unavailable, 1);
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.entries.len(), 6);
    }

    #[test]
    fn a_shared_host_module_is_skipped_not_probed() {
        // A compose-ready module the fleet never started must not
        // read as a service that failed to start.
        let entries = vec![LivenessCandidate {
            id: "platform-modules".to_string(),
            classification: InventoryClassification::ComposeReady,
            host_model: Some(HostModel::SharedHost),
            facts: facts(None, Some("platform-modules.tooosall.uk"), None, None),
        }];
        let report = build_report(
            entries,
            "inventory:/tmp/inv.json".to_string(),
            "mac".to_string(),
            "tooosall.uk".to_string(),
            "2026-09-28T00:00:00Z".to_string(),
        );
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.summary.no_route, 0);
        assert_eq!(report.summary.not_deployed, 0);
        assert_eq!(report.summary.online, 0);
        let entry = &report.entries[0];
        assert_eq!(entry.verdict, Verdict::Skipped);
        assert_eq!(entry.host_model, Some(HostModel::SharedHost));
        assert_eq!(entry.detail.as_deref(), Some(SHARED_HOST_REASON));
    }
}
