//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::core::manifest::Manifest;
    use crate::core::ForgeError;
    use crate::fleet::FleetReport;
    use crate::portal::activity::render_dashboard_human;
    use crate::portal::constants::{
        MAX_ENTRIES_PER_VIEW, MAX_ENTRY_ID_LEN, MAX_PORTAL_FLEET_ENTRIES, PORTAL_CONTRACT_VERSION,
        SECTION_FEATURES, SECTION_PROJECTS, SECTION_SERVERS, SUPPORTED_SECTIONS,
    };
    use crate::portal::model::{
        FleetProjection, PortalConfig, PortalDashboard, PortalEntry, PortalScope, PortalSection,
        PortalSectionView, PortalStatus,
    };
    use crate::portal::render::render_section_human;
    use crate::portal::sections::{
        build_agents_section, build_analytics_section, build_components_section,
        build_deployments_section, build_documentation_section, build_features_section,
        build_policies_section, build_projects_section, build_repositories_section,
        build_section_view, build_servers_section, build_settings_section, build_specs_section,
        controls_for, fleet_block_entries, parse_section, validate_entries,
    };
    use crate::registry::{ProjectRecord, Registry};
    use std::collections::BTreeMap;
    use std::path::Path;

    fn sample_meta() -> Manifest {
        let raw = br#"
schema: 1
project:
  id: portal-sample
  name: Portal Sample
  profile: rust-web
  maturity: L2
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
portal:
  enabled: true
  title: Custom Dashboard
  default_scope: fleet
"#;
        Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap()
    }

    fn empty_meta() -> Manifest {
        let raw = br#"
schema: 1
project:
  id: portal-empty
  name: Empty
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
"#;
        Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap()
    }

    #[test]
    fn parse_section_accepts_every_supported_id() {
        for id in SUPPORTED_SECTIONS {
            let parsed = parse_section(id).expect(id);
            assert_eq!(parsed.id(), *id);
        }
    }

    #[test]
    fn parse_section_refuses_unknown_id() {
        let err = parse_section("unknown").unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn every_section_names_a_valid_spa_pointer() {
        // The (section, spa_route, web_coverage) triple is the
        // convergence contract: only the five dashboard routes (or
        // empty for CLI-only) and only the three verdicts.
        let expected: &[(&str, &str, &str)] = &[
            ("projects", "/projects", "covered"),
            ("features", "/workbench", "covered"),
            ("components", "/projects", "covered"),
            ("policies", "/workbench", "partial"),
            ("specs", "/projects", "covered"),
            ("agents", "/projects", "covered"),
            ("deployments", "/delivery", "covered"),
            ("repositories", "/management", "partial"),
            ("documentation", "", "cli-only"),
            ("analytics", "/projects", "partial"),
            ("servers", "", "cli-only"),
            ("settings", "/projects", "partial"),
        ];
        assert_eq!(expected.len(), SUPPORTED_SECTIONS.len());
        for (id, route, coverage) in expected {
            let section = parse_section(id).expect(id);
            assert_eq!(section.spa_route(), *route, "{id} route");
            assert_eq!(section.web_coverage(), *coverage, "{id} coverage");
            assert!(
                [
                    "/projects",
                    "/workbench",
                    "/management",
                    "/portfolio",
                    "/delivery",
                    ""
                ]
                .contains(&section.spa_route()),
                "{id} names a route no dashboard view serves",
            );
            assert!(
                ["covered", "partial", "cli-only"].contains(&section.web_coverage()),
                "{id} names an unknown verdict",
            );
            assert!(
                (*coverage == "cli-only") == section.spa_route().is_empty(),
                "{id} must pair cli-only with an empty route and vice versa",
            );
            assert!(
                !section.spa_note().trim().is_empty(),
                "{id} must carry an operator note",
            );
        }
    }

    #[test]
    fn parse_section_is_idempotent_on_whitespace() {
        let parsed = parse_section("  projects  ").unwrap();
        assert_eq!(parsed, PortalSection::Projects);
    }

    #[test]
    fn portal_config_default_is_consistent() {
        let cfg = PortalConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.title, "Forge Control Plane");
        assert_eq!(cfg.default_scope, PortalScope::Project);
    }

    #[test]
    fn portal_config_from_manifest_uses_defaults_when_block_missing() {
        let cfg = PortalConfig::from_manifest(&empty_meta()).unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.default_scope, PortalScope::Project);
        assert_eq!(cfg.title, "Forge Control Plane");
    }

    #[test]
    fn portal_config_from_manifest_uses_typed_fields() {
        let cfg = PortalConfig::from_manifest(&sample_meta()).unwrap();
        assert_eq!(cfg.title, "Custom Dashboard");
        assert_eq!(cfg.default_scope, PortalScope::Fleet);
    }

    #[test]
    fn portal_config_from_manifest_refuses_empty_title() {
        let raw = br#"
schema: 1
project:
  id: portal-bad
  name: Bad
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
portal:
  enabled: true
  title: ""
  default_scope: project
"#;
        let manifest = Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap();
        let err = PortalConfig::from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn portal_config_from_manifest_refuses_oversized_title() {
        let oversized = "a".repeat(129);
        let raw = format!(
            r#"
schema: 1
project:
  id: portal-bad
  name: Bad
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {{}}
portal:
  enabled: true
  title: "{oversized}"
  default_scope: project
"#
        );
        let manifest = Manifest::parse(std::path::Path::new("forge.yaml"), raw.as_bytes()).unwrap();
        let err = PortalConfig::from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn portal_config_from_manifest_refuses_unknown_scope() {
        let raw = br#"
schema: 1
project:
  id: portal-bad
  name: Bad
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
portal:
  enabled: true
  title: "Forge"
  default_scope: organization
"#;
        let manifest = Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap();
        let err = PortalConfig::from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn portal_config_from_manifest_refuses_title_with_disabled_block() {
        let raw = br#"
schema: 1
project:
  id: portal-bad
  name: Bad
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
portal:
  enabled: false
  title: "Custom"
"#;
        let manifest = Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap();
        let err = PortalConfig::from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn validate_entries_refuses_oversized_list() {
        let entries: Vec<PortalEntry> = (0..(MAX_ENTRIES_PER_VIEW + 1))
            .map(|i| PortalEntry {
                id: format!("e{i}"),
                label: "x".to_string(),
                status: PortalStatus::Ok,
                source: "registry".to_string(),
                observed_at: "2026-01-15T12:00:00Z".to_string(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            })
            .collect();
        let err = validate_entries(&entries).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn validate_entries_refuses_empty_id() {
        let entry = PortalEntry {
            id: String::new(),
            label: "x".to_string(),
            status: PortalStatus::Ok,
            source: "registry".to_string(),
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            evidence: Vec::new(),
            attributes: BTreeMap::new(),
        };
        let err = validate_entries(&[entry]).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn validate_entries_refuses_oversized_id() {
        let entry = PortalEntry {
            id: "x".repeat(MAX_ENTRY_ID_LEN + 1),
            label: "x".to_string(),
            status: PortalStatus::Ok,
            source: "registry".to_string(),
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            evidence: Vec::new(),
            attributes: BTreeMap::new(),
        };
        let err = validate_entries(&[entry]).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn section_rollup_returns_worst_status() {
        let entries = vec![
            PortalEntry {
                id: "a".to_string(),
                label: "a".to_string(),
                status: PortalStatus::Ok,
                source: "registry".to_string(),
                observed_at: "2026-01-15T12:00:00Z".to_string(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            },
            PortalEntry {
                id: "b".to_string(),
                label: "b".to_string(),
                status: PortalStatus::Fail,
                source: "registry".to_string(),
                observed_at: "2026-01-15T12:00:00Z".to_string(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            },
        ];
        assert_eq!(
            PortalSectionView::rollup_status(&entries),
            PortalStatus::Fail
        );
    }

    #[test]
    fn dashboard_rollup_returns_worst_section() {
        let section_a = PortalSectionView {
            contract: PORTAL_CONTRACT_VERSION.to_string(),
            generated_at: "2026-01-15T12:00:00Z".to_string(),
            project_id: None,
            scope: PortalScope::Fleet,
            section: PortalSection::Projects,
            section_id: SECTION_PROJECTS.to_string(),
            status: PortalStatus::Ok,
            source: "registry".to_string(),
            entries: Vec::new(),
            controls_available: Vec::new(),
            spa_route: PortalSection::Projects.spa_route().to_string(),
            web_coverage: PortalSection::Projects.web_coverage().to_string(),
        };
        let section_b = PortalSectionView {
            status: PortalStatus::Unavailable,
            ..section_a.clone()
        };
        let dashboard = PortalDashboard {
            contract: PORTAL_CONTRACT_VERSION.to_string(),
            generated_at: "2026-01-15T12:00:00Z".to_string(),
            title: "Forge".to_string(),
            scope: PortalScope::Fleet,
            project_id: None,
            section_count: 2,
            sections: vec![section_a, section_b],
            operations: Vec::new(),
        };
        assert_eq!(dashboard.rollup(), PortalStatus::Unavailable);
    }

    #[test]
    fn dashboard_drops_project_id_for_fleet_scope() {
        let records: Vec<ProjectRecord> = Vec::new();
        let dashboard = build_dashboard_from_records(
            &records,
            PortalScope::Fleet,
            None,
            "2026-01-15T12:00:00Z",
        )
        .unwrap();
        assert!(dashboard.project_id.is_none());
        assert_eq!(dashboard.scope, PortalScope::Fleet);
        assert_eq!(dashboard.sections.len(), SUPPORTED_SECTIONS.len());
    }

    fn build_dashboard_from_records(
        records: &[ProjectRecord],
        scope: PortalScope,
        project_id: Option<&str>,
        now: &str,
    ) -> Result<PortalDashboard, ForgeError> {
        build_dashboard_from_records_with_fleet(records, scope, project_id, now, None)
    }

    fn build_dashboard_from_records_with_fleet(
        records: &[ProjectRecord],
        scope: PortalScope,
        project_id: Option<&str>,
        now: &str,
        fleet: Option<FleetProjection<'_>>,
    ) -> Result<PortalDashboard, ForgeError> {
        let mut sections = Vec::new();
        for section in [
            PortalSection::Projects,
            PortalSection::Features,
            PortalSection::Components,
            PortalSection::Policies,
            PortalSection::Specs,
            PortalSection::Agents,
            PortalSection::Deployments,
            PortalSection::Repositories,
            PortalSection::Documentation,
            PortalSection::Analytics,
            PortalSection::Servers,
            PortalSection::Settings,
        ] {
            let (entries, source) = match section {
                PortalSection::Projects => {
                    let mut entries = build_projects_section(records);
                    if scope == PortalScope::Fleet {
                        entries.extend(fleet_block_entries(fleet.clone(), now));
                    }
                    (entries, "registry")
                }
                PortalSection::Features => (build_features_section(records), "registry"),
                PortalSection::Components => (build_components_section(records), "registry"),
                PortalSection::Policies => (build_policies_section(records), "registry"),
                PortalSection::Specs => (build_specs_section(records), "registry"),
                PortalSection::Agents => (build_agents_section(records), "registry"),
                PortalSection::Deployments => (build_deployments_section(records), "registry"),
                PortalSection::Repositories => (build_repositories_section(records), "registry"),
                PortalSection::Documentation => (build_documentation_section(records), "registry"),
                PortalSection::Analytics => (build_analytics_section(records), "registry"),
                PortalSection::Servers => (build_servers_section(records), "registry"),
                PortalSection::Settings => (build_settings_section(records), "registry"),
            };
            validate_entries(&entries)?;
            let status = PortalSectionView::rollup_status(&entries);
            sections.push(PortalSectionView {
                contract: PORTAL_CONTRACT_VERSION.to_string(),
                generated_at: now.to_string(),
                project_id: project_id.map(str::to_string),
                scope,
                section,
                section_id: section.id().to_string(),
                status,
                source: source.to_string(),
                entries,
                controls_available: controls_for(section),
                spa_route: section.spa_route().to_string(),
                web_coverage: section.web_coverage().to_string(),
            });
        }
        Ok(PortalDashboard {
            contract: PORTAL_CONTRACT_VERSION.to_string(),
            generated_at: now.to_string(),
            title: "Forge Control Plane".to_string(),
            scope,
            project_id: project_id.map(str::to_string),
            section_count: sections.len(),
            sections,
            operations: Vec::new(),
        })
    }

    #[test]
    fn dashboard_uses_unavailable_for_missing_projects() {
        let record = ProjectRecord {
            id: "p1".to_string(),
            name: "P1".to_string(),
            path: "/nonexistent".to_string(),
            git_remote: None,
            mirror_remotes: Vec::new(),
            stack: None,
            profile: "rust-web".to_string(),
            maturity: Some("L1".to_string()),
            target_maturity: Some("L2".to_string()),
            schema_version: 1,
            platform_version: "0.1.0".to_string(),
            features: BTreeMap::new(),
            deployment_target: None,
            runtime: None,
            last_commit: None,
            quality_status: None,
            agent_status: None,
            docs_status: None,
            kit_id: None,
            kit_version: None,
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            available: false,
        };
        let dashboard = build_dashboard_from_records(
            &[record],
            PortalScope::Project,
            Some("p1"),
            "2026-01-15T12:00:00Z",
        )
        .unwrap();
        let projects_section = dashboard
            .sections
            .iter()
            .find(|s| s.section == PortalSection::Projects)
            .unwrap();
        assert_eq!(projects_section.status, PortalStatus::Unavailable);
    }

    #[test]
    fn human_renderer_carries_required_fields() {
        let dashboard =
            build_dashboard_from_records(&[], PortalScope::Fleet, None, "2026-01-15T12:00:00Z")
                .unwrap();
        let text = render_dashboard_human(&dashboard);
        for needle in [
            PORTAL_CONTRACT_VERSION,
            "Forge Control Plane",
            "fleet",
            "rollup",
            "sections",
            "recent operations",
            "interactive surface",
            "forge web",
            "read-only pointer",
        ] {
            assert!(text.contains(needle), "{needle} missing in {text}");
        }
    }

    #[test]
    fn section_human_renderer_carries_required_fields() {
        let record = ProjectRecord {
            id: "p1".to_string(),
            name: "P1".to_string(),
            path: "/nonexistent".to_string(),
            git_remote: None,
            mirror_remotes: Vec::new(),
            stack: None,
            profile: "rust-web".to_string(),
            maturity: Some("L1".to_string()),
            target_maturity: Some("L2".to_string()),
            schema_version: 1,
            platform_version: "0.1.0".to_string(),
            features: BTreeMap::from([("auth".to_string(), "0.1.0".to_string())]),
            deployment_target: None,
            runtime: None,
            last_commit: None,
            quality_status: Some("warn".to_string()),
            agent_status: None,
            docs_status: None,
            kit_id: None,
            kit_version: None,
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            available: true,
        };
        let mut features = build_features_section(std::slice::from_ref(&record));
        assert_eq!(features.len(), 1);
        let entry = features.remove(0);
        assert_eq!(entry.id, "p1:auth");
        assert_eq!(entry.status, PortalStatus::Ok);
        let view = PortalSectionView {
            contract: PORTAL_CONTRACT_VERSION.to_string(),
            generated_at: "2026-01-15T12:00:00Z".to_string(),
            project_id: Some("p1".to_string()),
            scope: PortalScope::Project,
            section: PortalSection::Features,
            section_id: SECTION_FEATURES.to_string(),
            status: PortalStatus::Ok,
            source: "feature".to_string(),
            entries: vec![entry],
            controls_available: controls_for(PortalSection::Features),
            spa_route: PortalSection::Features.spa_route().to_string(),
            web_coverage: PortalSection::Features.web_coverage().to_string(),
        };
        let text = render_section_human(&view);
        for needle in [
            PORTAL_CONTRACT_VERSION,
            "Features",
            SECTION_FEATURES,
            "p1:auth",
            "version=0.1.0",
            "controls_available",
            "spa: /workbench",
            "web: covered",
        ] {
            assert!(text.contains(needle), "{needle} missing in {text}");
        }
    }

    #[test]
    fn section_view_returns_requested_section_only() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("registry.db");
        let registry = Registry::open(&db_path).unwrap();
        // No registered projects: the dashboard is built against
        // an empty registry; the section view is the same view
        // for the requested section.
        let view = build_section_view(&registry, None, PortalSection::Servers).unwrap();
        assert_eq!(view.section, PortalSection::Servers);
        assert_eq!(view.section_id, SECTION_SERVERS);
        assert!(view.entries.is_empty());
    }

    #[test]
    fn dashboard_records_unknown_observations_prominently() {
        let record = ProjectRecord {
            id: "p-unknown".to_string(),
            name: "Unknown".to_string(),
            path: "/nonexistent".to_string(),
            git_remote: None,
            mirror_remotes: Vec::new(),
            stack: None,
            profile: "rust-web".to_string(),
            maturity: Some("L1".to_string()),
            target_maturity: Some("L2".to_string()),
            schema_version: 1,
            platform_version: "0.1.0".to_string(),
            features: BTreeMap::new(),
            deployment_target: None,
            runtime: None,
            last_commit: None,
            quality_status: None,
            agent_status: None,
            docs_status: None,
            kit_id: None,
            kit_version: None,
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            available: true,
        };
        let dashboard = build_dashboard_from_records(
            &[record],
            PortalScope::Project,
            Some("p-unknown"),
            "2026-01-15T12:00:00Z",
        )
        .unwrap();
        // A project with no runtime/quality/agents/... is
        // reported as `unknown`, not `ok`. The dashboard rollup
        // surfaces the worst section so the operator never sees
        // a false-positive `ok`.
        let rollup = dashboard.rollup();
        assert!(
            matches!(
                rollup,
                PortalStatus::Unknown | PortalStatus::Unavailable | PortalStatus::Warn
            ),
            "rollup was {rollup:?}"
        );
    }

    #[test]
    fn portal_section_label_matches_id() {
        for section in [
            PortalSection::Projects,
            PortalSection::Features,
            PortalSection::Components,
            PortalSection::Policies,
            PortalSection::Specs,
            PortalSection::Agents,
            PortalSection::Deployments,
            PortalSection::Repositories,
            PortalSection::Documentation,
            PortalSection::Analytics,
            PortalSection::Servers,
            PortalSection::Settings,
        ] {
            assert!(!section.id().is_empty());
            assert!(!section.label().is_empty());
            assert!(SUPPORTED_SECTIONS.contains(&section.id()));
        }
    }

    fn fleet_entry(id: &str, managed: bool) -> crate::fleet::FleetEntry {
        crate::fleet::FleetEntry {
            id: id.to_string(),
            path: id.to_string(),
            profile: "rust-product".to_string(),
            lifecycle: "active".to_string(),
            adoption: Some("adopted".to_string()),
            forge_yaml_present: managed,
            locally_registered: managed,
            state: if managed {
                crate::fleet::FleetState::Managed
            } else {
                crate::fleet::FleetState::Unmanaged
            },
        }
    }

    fn fleet_report(freshness: crate::fleet::FleetFreshness) -> FleetReport {
        FleetReport {
            contract: crate::fleet::FLEET_CONTRACT_VERSION.to_string(),
            source: Some("/ws/projects.json".to_string()),
            observed_at: "2026-09-24T12:00:00Z".to_string(),
            freshness,
            max_age_seconds: 86_400,
            age_seconds: Some(10),
            entries: vec![fleet_entry("alpha", true), fleet_entry("beta", false)],
            malformed: Vec::new(),
        }
    }

    fn projects_view_status_and_entries(
        scope: PortalScope,
        fleet: Option<FleetProjection<'_>>,
    ) -> (Vec<PortalEntry>, PortalStatus) {
        let mut entries = build_projects_section(&[]);
        if scope == PortalScope::Fleet {
            entries.extend(fleet_block_entries(fleet, "2026-09-24T12:00:00Z"));
        }
        let status = PortalSectionView::rollup_status(&entries);
        (entries, status)
    }

    #[test]
    fn fleet_block_is_absent_when_unconfigured_or_project_scope() {
        // Unconfigured: no fleet block at all (workflows byte-identical).
        let (entries, status) = projects_view_status_and_entries(PortalScope::Fleet, None);
        assert!(entries.is_empty());
        assert_eq!(status, PortalStatus::Ok);
        // Project scope never renders the fleet block even when configured.
        let report = fleet_report(crate::fleet::FleetFreshness::Fresh);
        let (entries, _) = projects_view_status_and_entries(
            PortalScope::Project,
            Some(FleetProjection::Configured(&report)),
        );
        assert!(
            entries.is_empty(),
            "project scope must not carry fleet entries"
        );
    }

    #[test]
    fn fleet_block_carries_normalized_entries_in_fleet_scope() {
        let report = fleet_report(crate::fleet::FleetFreshness::Fresh);
        let (entries, status) = projects_view_status_and_entries(
            PortalScope::Fleet,
            Some(FleetProjection::Configured(&report)),
        );
        assert_eq!(entries.len(), 3, "meta + 2 fleet entries");
        assert_eq!(entries[0].id, "fleet:source");
        assert_eq!(entries[0].status, PortalStatus::Ok);
        assert_eq!(entries[1].id, "fleet:alpha");
        assert_eq!(entries[1].status, PortalStatus::Ok);
        assert_eq!(entries[2].id, "fleet:beta");
        assert_eq!(
            entries[2].attributes.get("state").map(String::as_str),
            Some("unmanaged")
        );
        assert_eq!(
            entries[1].attributes.get("forge_yaml").map(String::as_str),
            Some("present")
        );
        assert_eq!(
            entries[2].attributes.get("forge_yaml").map(String::as_str),
            Some("missing")
        );
        // The section roll-up keeps a fresh clean block at ok.
        assert_eq!(status, PortalStatus::Ok);
    }

    #[test]
    fn stale_fleet_block_never_rolls_up_as_ok() {
        let report = fleet_report(crate::fleet::FleetFreshness::Stale);
        let (entries, status) = projects_view_status_and_entries(
            PortalScope::Fleet,
            Some(FleetProjection::Configured(&report)),
        );
        assert_eq!(status, PortalStatus::Warn, "stale must be at most warn");
        assert!(entries.iter().all(|e| e.status == PortalStatus::Warn));
        let meta = &entries[0];
        assert!(meta
            .evidence
            .iter()
            .any(|line| line.contains("freshness=stale")));
    }

    #[test]
    fn malformed_fleet_entries_keep_the_meta_out_of_ok_and_name_reasons() {
        let mut report = fleet_report(crate::fleet::FleetFreshness::Fresh);
        report.malformed.push(crate::fleet::FleetMalformedEntry {
            name: "runner".to_string(),
            reason: "traverses outside the registry workspace root".to_string(),
        });
        let (entries, status) = projects_view_status_and_entries(
            PortalScope::Fleet,
            Some(FleetProjection::Configured(&report)),
        );
        assert_eq!(status, PortalStatus::Warn);
        assert!(entries[0]
            .evidence
            .iter()
            .any(|line| line.contains("malformed") && line.contains("runner")));
        assert_eq!(
            entries.len(),
            3,
            "malformed entries are named in the meta, not listed"
        );
    }

    #[test]
    fn failed_fleet_source_is_unavailable_not_masked() {
        let (entries, status) = projects_view_status_and_entries(
            PortalScope::Fleet,
            Some(FleetProjection::Failed {
                code: "fleet-registry-invalid",
                reason: "unknown registry schema_version 7".to_string(),
            }),
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, PortalStatus::Unavailable);
        assert_eq!(status, PortalStatus::Unavailable);
        assert!(entries[0].evidence[0].contains("fleet-registry-invalid"));
    }

    #[test]
    fn oversized_fleet_blocks_truncate_with_an_explicit_note() {
        let mut report = fleet_report(crate::fleet::FleetFreshness::Fresh);
        report.entries = (0..(MAX_PORTAL_FLEET_ENTRIES + 10))
            .map(|i| fleet_entry(&format!("p{i}"), false))
            .collect();
        let (entries, _) = projects_view_status_and_entries(
            PortalScope::Fleet,
            Some(FleetProjection::Configured(&report)),
        );
        assert_eq!(entries.len(), MAX_PORTAL_FLEET_ENTRIES + 1);
        assert_eq!(
            entries[0]
                .attributes
                .get("portal_rendered")
                .map(String::as_str),
            Some("128 of 138")
        );
        validate_entries(&entries).expect("the rendered block stays inside the view bound");
    }
}
