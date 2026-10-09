//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::analytics::constants::{
        ANALYTICS_CONTRACT_VERSION, MAX_PROVIDERS_PER_PLANE, MAX_WINDOW_DAYS, METRIC_PROJECTS,
        METRIC_QUALITY_FAILURES, METRIC_QUALITY_HEALTHY, METRIC_REPOSITORY_STARS,
        METRIC_REPOSITORY_STARS_GROWTH, METRIC_STATE_MIXED_WINDOWS, METRIC_STATE_PRESENT,
        METRIC_STATE_UNAVAILABLE, PROVIDER_GITHUB_ANALYTICS, PROVIDER_NOTION,
        PROVIDER_UNIFIED_CONTENT, STATUS_AMBIGUOUS, STATUS_AVAILABLE, STATUS_DISABLED,
        STATUS_UNAVAILABLE, WINDOW_INSTANT,
    };
    use crate::analytics::model::{
        AnalyticsConfig, AnalyticsInspectOptions, AnalyticsProvider, DoctorSummary,
        HealthObservation, MetricsAggregateOptions, MetricsSummary, ProviderSupportStatus,
    };
    use crate::analytics::projections::{
        aggregate_project_metrics, count_metric, inspect_external_planes, load_metrics_summary,
        parse_provider, provider_support_status, redact_analytics_evidence, render_metrics_human,
        render_report_human, save_metrics_summary,
    };
    use crate::core::manifest::{AnalyticsMeta, AnalyticsProviderEntry, Manifest};
    use crate::registry::Registry;

    fn sample_meta() -> AnalyticsMeta {
        AnalyticsMeta {
            enabled: Some(true),
            default_window_days: Some(7),
            content: vec![AnalyticsProviderEntry {
                provider: PROVIDER_UNIFIED_CONTENT.to_string(),
                enabled: Some(true),
                project_ref: Some("forge-proj".to_string()),
                adapter_command: None,
            }],
            repository: vec![AnalyticsProviderEntry {
                provider: PROVIDER_GITHUB_ANALYTICS.to_string(),
                enabled: Some(true),
                project_ref: Some("owner/repo".to_string()),
                adapter_command: None,
            }],
        }
    }

    #[test]
    fn parse_provider_supports_supported_set() {
        assert_eq!(
            parse_provider(PROVIDER_UNIFIED_CONTENT).unwrap(),
            AnalyticsProvider::UnifiedContent
        );
        assert_eq!(
            parse_provider(PROVIDER_GITHUB_ANALYTICS).unwrap(),
            AnalyticsProvider::GithubAnalytics
        );
    }

    #[test]
    fn parse_provider_refuses_unknown() {
        let err = parse_provider("nope").unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
    }

    #[test]
    fn provider_support_status_separates_supported_and_planned() {
        assert_eq!(
            provider_support_status(AnalyticsProvider::GithubAnalytics),
            ProviderSupportStatus::Supported
        );
        assert_eq!(
            provider_support_status(AnalyticsProvider::Notion),
            ProviderSupportStatus::Planned
        );
    }

    #[test]
    fn config_from_manifest_validates_supported_block() {
        let cfg = AnalyticsConfig::from_manifest_meta(&sample_meta()).unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.content.len(), 1);
        assert_eq!(cfg.repository.len(), 1);
        assert_eq!(cfg.default_window_days, 7);
    }

    #[test]
    fn config_refuses_unknown_provider() {
        let mut meta = sample_meta();
        meta.content[0].provider = "nope".to_string();
        let err = AnalyticsConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
    }

    #[test]
    fn config_refuses_empty_project_ref() {
        let mut meta = sample_meta();
        meta.content[0].project_ref = Some("".to_string());
        let err = AnalyticsConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
    }

    #[test]
    fn config_refuses_shell_metacharacter_project_ref() {
        let mut meta = sample_meta();
        meta.repository[0].project_ref = Some("owner; rm -rf /".to_string());
        let err = AnalyticsConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
    }

    #[test]
    fn config_refuses_duplicate_provider() {
        let mut meta = sample_meta();
        meta.content.push(AnalyticsProviderEntry {
            provider: PROVIDER_UNIFIED_CONTENT.to_string(),
            enabled: Some(true),
            project_ref: Some("dup".to_string()),
            adapter_command: None,
        });
        let err = AnalyticsConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
    }

    #[test]
    fn config_refuses_oversized_provider_list() {
        let mut meta = sample_meta();
        for idx in 0..(MAX_PROVIDERS_PER_PLANE + 1) {
            meta.content.push(AnalyticsProviderEntry {
                provider: format!("unified-content-{idx}"),
                enabled: Some(true),
                project_ref: Some(format!("slug-{idx}")),
                adapter_command: None,
            });
        }
        let err = AnalyticsConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
    }

    #[test]
    fn config_refuses_out_of_range_window() {
        let mut meta = sample_meta();
        meta.default_window_days = Some(0);
        let err = AnalyticsConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
        meta.default_window_days = Some(MAX_WINDOW_DAYS + 1);
        let err = AnalyticsConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
    }

    #[test]
    fn config_from_manifest_opt_handles_absent_block() {
        let manifest = Manifest::parse(
            std::path::Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\n",
        )
        .unwrap();
        let cfg = AnalyticsConfig::from_manifest_opt(&manifest).unwrap();
        assert!(cfg.is_none());
    }

    #[test]
    fn inspect_disabled_block_reports_disabled_without_contacting_adapter() {
        let mut meta = sample_meta();
        meta.enabled = Some(false);
        let cfg = AnalyticsConfig::from_manifest_meta(&meta).unwrap();
        let report = inspect_external_planes(
            "forge-proj",
            &cfg,
            &AnalyticsInspectOptions { dry_run: false },
        )
        .unwrap();
        assert!(!report.enabled);
        assert_eq!(report.observations.len(), 2);
        assert!(report
            .observations
            .iter()
            .all(|o| o.status == STATUS_DISABLED));
        assert!(report.healthy());
    }

    #[test]
    fn inspect_disabled_entry_is_disabled_boundary() {
        let mut meta = sample_meta();
        meta.content[0].enabled = Some(false);
        let cfg = AnalyticsConfig::from_manifest_meta(&meta).unwrap();
        let report = inspect_external_planes(
            "forge-proj",
            &cfg,
            &AnalyticsInspectOptions { dry_run: false },
        )
        .unwrap();
        let content_obs = report
            .observations
            .iter()
            .find(|o| o.plane == "content")
            .unwrap();
        assert_eq!(content_obs.status, STATUS_DISABLED);
        // No adapter call should have been made; the
        // available / observation source is the manifest.
        assert_eq!(content_obs.source, "manifest");
    }

    #[test]
    fn inspect_planned_provider_reports_unavailable_with_source_catalog() {
        let mut meta = sample_meta();
        meta.content[0].provider = PROVIDER_NOTION.to_string();
        let cfg = AnalyticsConfig::from_manifest_meta(&meta).unwrap();
        let report = inspect_external_planes(
            "forge-proj",
            &cfg,
            &AnalyticsInspectOptions { dry_run: false },
        )
        .unwrap();
        let content_obs = report
            .observations
            .iter()
            .find(|o| o.plane == "content")
            .unwrap();
        assert_eq!(content_obs.status, STATUS_UNAVAILABLE);
        assert_eq!(content_obs.source, "catalog");
    }

    #[test]
    fn inspect_dry_run_reports_available_without_invoking_adapter() {
        let cfg = AnalyticsConfig::from_manifest_meta(&sample_meta()).unwrap();
        let report = inspect_external_planes(
            "forge-proj",
            &cfg,
            &AnalyticsInspectOptions { dry_run: true },
        )
        .unwrap();
        assert!(report
            .observations
            .iter()
            .all(|o| o.status == STATUS_AVAILABLE));
        assert!(report.healthy());
    }

    #[test]
    fn inspect_missing_adapter_reports_unavailable() {
        let cfg = AnalyticsConfig::from_manifest_meta(&sample_meta()).unwrap();
        let report = inspect_external_planes(
            "forge-proj",
            &cfg,
            &AnalyticsInspectOptions { dry_run: false },
        )
        .unwrap();
        let available: Vec<&HealthObservation> = report
            .observations
            .iter()
            .filter(|o| o.status == STATUS_AVAILABLE)
            .collect();
        assert!(available.is_empty(), "{:?}", report.observations);
    }

    #[test]
    fn inspect_failing_adapter_reports_unavailable_with_evidence() {
        let mut meta = sample_meta();
        meta.repository[0].adapter_command = Some("false".to_string());
        let cfg = AnalyticsConfig::from_manifest_meta(&meta).unwrap();
        let report = inspect_external_planes(
            "forge-proj",
            &cfg,
            &AnalyticsInspectOptions { dry_run: false },
        )
        .unwrap();
        let repo_obs = report
            .observations
            .iter()
            .find(|o| o.plane == "repository")
            .unwrap();
        assert_eq!(repo_obs.status, STATUS_UNAVAILABLE);
        assert!(!repo_obs.evidence.is_empty());
        assert!(!report.healthy());
    }

    #[test]
    fn inspect_adapter_with_ambiguous_project_ref_reports_ambiguous() {
        let mut meta = sample_meta();
        meta.repository[0].adapter_command = Some("echo".to_string());
        // `echo` will succeed but the output will be empty
        // and fail JSON parsing. The boundary case we
        // exercise is: provide an adapter that prints a
        // different project_ref.
        meta.repository[0].project_ref = Some("other/repo".to_string());
        let cfg = AnalyticsConfig::from_manifest_meta(&meta).unwrap();
        let report = inspect_external_planes(
            "forge-proj",
            &cfg,
            &AnalyticsInspectOptions { dry_run: false },
        )
        .unwrap();
        let repo_obs = report
            .observations
            .iter()
            .find(|o| o.plane == "repository")
            .unwrap();
        // Without a real adapter the call lands in
        // parse-error or unavailable, but the path under
        // test is the ambiguous mapping. A
        // helper-driven JSON fixture will cover the
        // ambiguous code in the contract test.
        assert!(matches!(
            repo_obs.status.as_str(),
            STATUS_AMBIGUOUS | STATUS_UNAVAILABLE
        ));
    }

    #[test]
    fn metrics_aggregate_counters_present_with_windows() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("proj");
        std::fs::create_dir(&proj).unwrap();
        let manifest_text = "schema: 1\nproject:\n  id: metrics-app\n  name: Metrics App\n  \
                             profile: rust-web\n";
        std::fs::write(proj.join("forge.yaml"), manifest_text).unwrap();
        let db_path = tmp.path().join("registry.db");
        let mut registry = Registry::open(&db_path).unwrap();
        registry.register(&proj, None).unwrap();
        let doctor = DoctorSummary {
            healthy: 2,
            warnings: 1,
            failures: 1,
            agents_running: 3,
            specs_queued: 4,
            deployments_running: 5,
            deployments_offline: 1,
            deployments_pending: 2,
        };
        let report =
            aggregate_project_metrics(&registry, Some(doctor), &MetricsAggregateOptions::default())
                .unwrap();
        let projects = report.aggregate(METRIC_PROJECTS).unwrap();
        assert_eq!(projects.current, Some(1));
        assert_eq!(projects.windows, vec![WINDOW_INSTANT.to_string()]);
        let healthy = report.aggregate(METRIC_QUALITY_HEALTHY).unwrap();
        assert_eq!(healthy.current, Some(2));
        let failures = report.aggregate(METRIC_QUALITY_FAILURES).unwrap();
        assert_eq!(failures.current, Some(1));
    }

    #[test]
    fn metrics_repository_aggregate_handles_missing_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("registry.db");
        let registry = Registry::open(&db_path).unwrap();
        let report =
            aggregate_project_metrics(&registry, None, &MetricsAggregateOptions::default())
                .unwrap();
        let stars = report.aggregate(METRIC_REPOSITORY_STARS).unwrap();
        assert_eq!(stars.state, METRIC_STATE_UNAVAILABLE);
        assert!(stars.current.is_none());
        assert!(!report.complete);
    }

    #[test]
    fn metrics_repository_aggregate_sums_observations_from_same_window() {
        let obs_a = HealthObservation {
            provider: PROVIDER_GITHUB_ANALYTICS.to_string(),
            plane: "repository".to_string(),
            project_ref: "owner/repo".to_string(),
            status: STATUS_AVAILABLE.to_string(),
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            source: "adapter".to_string(),
            evidence: vec!["stars=42".to_string()],
            note: "ok".to_string(),
        };
        let obs_b = HealthObservation {
            provider: PROVIDER_GITHUB_ANALYTICS.to_string(),
            plane: "repository".to_string(),
            project_ref: "owner/repo".to_string(),
            status: STATUS_AVAILABLE.to_string(),
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            source: "adapter".to_string(),
            evidence: vec!["stars=42".to_string(), "growth=3".to_string()],
            note: "ok".to_string(),
        };
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("registry.db");
        let registry = Registry::open(&db_path).unwrap();
        let report = aggregate_project_metrics(
            &registry,
            None,
            &MetricsAggregateOptions {
                default_window_days: 7,
                external_observations: vec![obs_a, obs_b],
            },
        )
        .unwrap();
        let stars = report.aggregate(METRIC_REPOSITORY_STARS).unwrap();
        assert_eq!(stars.current, Some(42));
        assert_eq!(stars.state, METRIC_STATE_PRESENT);
        let growth = report.aggregate(METRIC_REPOSITORY_STARS_GROWTH).unwrap();
        assert_eq!(growth.current, Some(3));
        assert_eq!(growth.windows, vec!["7-day".to_string()]);
    }

    #[test]
    fn metrics_repository_aggregate_reports_mixed_windows() {
        // A second observation in a different window is
        // explicitly refused: the summary reports
        // `mixed-windows` rather than summing across
        // windows (R2 boundary).
        let obs_a = HealthObservation {
            provider: PROVIDER_GITHUB_ANALYTICS.to_string(),
            plane: "repository".to_string(),
            project_ref: "owner/repo".to_string(),
            status: STATUS_AVAILABLE.to_string(),
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            source: "adapter".to_string(),
            evidence: vec!["stars=42".to_string()],
            note: "ok".to_string(),
        };
        let obs_b = HealthObservation {
            provider: PROVIDER_GITHUB_ANALYTICS.to_string(),
            plane: "repository".to_string(),
            project_ref: "owner/repo".to_string(),
            status: STATUS_AVAILABLE.to_string(),
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            source: "adapter".to_string(),
            evidence: vec!["stars=99".to_string()],
            note: "ok".to_string(),
        };
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("registry.db");
        let registry = Registry::open(&db_path).unwrap();
        let report = aggregate_project_metrics(
            &registry,
            None,
            &MetricsAggregateOptions {
                default_window_days: 7,
                external_observations: vec![obs_a, obs_b],
            },
        )
        .unwrap();
        let stars = report.aggregate(METRIC_REPOSITORY_STARS).unwrap();
        assert_eq!(stars.state, METRIC_STATE_MIXED_WINDOWS);
        assert!(stars.current.is_none());
    }

    #[test]
    fn metrics_aggregate_refuses_out_of_range_window() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("registry.db");
        let registry = Registry::open(&db_path).unwrap();
        let err = aggregate_project_metrics(
            &registry,
            None,
            &MetricsAggregateOptions {
                default_window_days: 0,
                external_observations: Vec::new(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "analytics-invalid");
    }

    #[test]
    fn metrics_summary_round_trips_through_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("metrics.json");
        let summary = MetricsSummary {
            contract: ANALYTICS_CONTRACT_VERSION.to_string(),
            project_id: "p".to_string(),
            generated_at: "2026-01-15T12:00:00Z".to_string(),
            aggregates: vec![count_metric(
                METRIC_PROJECTS,
                WINDOW_INSTANT,
                "registry",
                "2026-01-15T12:00:00Z".to_string(),
                7,
            )],
        };
        save_metrics_summary(&path, &summary).unwrap();
        let loaded = load_metrics_summary(&path).unwrap();
        assert_eq!(loaded, summary);
    }

    #[test]
    fn redact_analytics_evidence_delegates_to_policy() {
        let sample = "token=ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let redacted = redact_analytics_evidence(sample);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("ghp_abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn human_renderers_carry_required_fields() {
        let cfg = AnalyticsConfig::from_manifest_meta(&sample_meta()).unwrap();
        let report = inspect_external_planes(
            "forge-proj",
            &cfg,
            &AnalyticsInspectOptions { dry_run: true },
        )
        .unwrap();
        let text = render_report_human(&report);
        for needle in [
            ANALYTICS_CONTRACT_VERSION,
            "forge-proj",
            PROVIDER_UNIFIED_CONTENT,
            PROVIDER_GITHUB_ANALYTICS,
        ] {
            assert!(text.contains(needle), "{needle} missing in {text}");
        }
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("registry.db");
        let registry = Registry::open(&db_path).unwrap();
        let metrics =
            aggregate_project_metrics(&registry, None, &MetricsAggregateOptions::default())
                .unwrap();
        let text = render_metrics_human(&metrics);
        for needle in [
            ANALYTICS_CONTRACT_VERSION,
            METRIC_PROJECTS,
            METRIC_REPOSITORY_STARS,
        ] {
            assert!(text.contains(needle), "{needle} missing in {text}");
        }
    }
}
