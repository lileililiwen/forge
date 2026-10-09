//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    use chrono::{DateTime, Utc};

    use crate::fleet::limits::{
        DEFAULT_MAX_AGE_SECONDS, FLEET_CONTRACT_VERSION, MAX_FIELD_CHARS, MAX_MAX_AGE_SECONDS,
        MAX_REGISTRY_ENTRIES, WORKSPACE_REGISTRY_ENV,
    };
    use crate::fleet::model::{FleetEntry, FleetFreshness, FleetState};
    use crate::fleet::registry::{
        classify_freshness, entry_fields, entry_summary_line, health_json, inspect_entry,
        observe_at, render_entry_human, render_list_block_human, render_report_human,
        render_status_human, resolve_registry_path, validate_max_age,
    };

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-24T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn ids(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    /// Write a registry document into a fixture tree that mirrors the
    /// real Workspace Governance shape.
    fn workspace(parent: &Path) -> PathBuf {
        let root = parent.join("ws");
        let registry_dir = root.join("workspace-governance");
        fs::create_dir_all(registry_dir.join("alpha")).unwrap();
        fs::create_dir_all(registry_dir.join("beta")).unwrap();
        fs::create_dir_all(registry_dir.join("gamma")).unwrap();
        fs::write(registry_dir.join("alpha/forge.yaml"), "schema: 1\n").unwrap();
        fs::write(registry_dir.join("alpha/README.md"), "# alpha\n").unwrap();
        fs::write(registry_dir.join("beta/forge.yaml"), "schema: 1\n").unwrap();
        registry_dir
    }

    fn write_registry(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("projects.json");
        fs::write(&path, body).unwrap();
        path
    }

    fn clean_body() -> String {
        r#"{
  "schema_version": 1,
  "workspace_root": null,
  "discovery": {
    "mode": "explicit-plus-immediate-directories",
    "unregistered_policy": "report",
    "exclude": ["build", "dist", "node_modules", "target", "vendor"]
  },
  "generated_by": "workspace-governance/scripts/whatever.py",
  "projects": [
    {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active", "adoption": "adopted"},
    {"id": "beta", "path": "beta", "profile": "dotnet-library", "lifecycle": "planning", "adoption": null},
    {"id": "gamma", "path": "gamma", "profile": "typescript-monorepo", "lifecycle": "active"}
  ]
}"#
        .to_string()
    }

    #[test]
    fn parses_the_real_wg_shape_and_tolerates_unknown_fields() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        let report = observe_at(
            Some(&registry),
            DEFAULT_MAX_AGE_SECONDS,
            &ids(&["beta"]),
            now(),
        )
        .unwrap();
        assert_eq!(report.contract, FLEET_CONTRACT_VERSION);
        assert_eq!(report.freshness, FleetFreshness::Fresh);
        assert_eq!(report.entries.len(), 3);
        assert!(report.malformed.is_empty());
        let alpha = &report.entries[0];
        assert_eq!(alpha.id, "alpha");
        assert_eq!(alpha.path, "alpha");
        assert_eq!(alpha.profile, "rust-product");
        assert_eq!(alpha.lifecycle, "active");
        assert_eq!(alpha.adoption.as_deref(), Some("adopted"));
        assert!(alpha.forge_yaml_present);
        assert_eq!(alpha.state, FleetState::Unmanaged);
        let beta = &report.entries[1];
        assert!(beta.forge_yaml_present);
        assert!(beta.adoption.is_none(), "null adoption projects as none");
        assert_eq!(beta.state, FleetState::Managed, "joined by id only");
        assert!(beta.locally_registered);
        let gamma = &report.entries[2];
        assert!(!gamma.forge_yaml_present);
        assert!(gamma.adoption.is_none());
        assert!(report.source.as_ref().unwrap().ends_with("projects.json"));
        assert!(report.age_seconds.is_some());
    }

    #[test]
    fn profiles_are_surfaced_verbatim_never_coerced() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        let profiles: Vec<&str> = report.entries.iter().map(|e| e.profile.as_str()).collect();
        assert_eq!(
            profiles,
            vec!["rust-product", "dotnet-library", "typescript-monorepo"]
        );
    }

    #[test]
    fn unknown_schema_version_refuses_the_report() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            &clean_body().replace("\"schema_version\": 1", "\"schema_version\": 2"),
        );
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("schema_version"), "{err}");
    }

    #[test]
    fn missing_schema_version_and_malformed_json_refuse() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, r#"{"projects": []}"#);
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        let registry = write_registry(&dir, "{not json");
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("malformed JSON"), "{err}");
    }

    #[test]
    fn duplicate_ids_refuse_the_report_naming_the_id() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "workspace_root": null, "projects": [
  {"id": "dup", "path": "alpha", "profile": "product", "lifecycle": "active"},
  {"id": "dup", "path": "beta", "profile": "product", "lifecycle": "active"}
]}"#,
        );
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("`dup`"), "{err}");
    }

    #[test]
    fn traversing_entry_is_malformed_and_the_rest_still_reports() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "runner", "path": "../outside-root", "profile": "product", "lifecycle": "active"},
  {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].id, "alpha");
        assert_eq!(report.malformed.len(), 1);
        assert_eq!(report.malformed[0].name, "runner");
        assert!(
            report.malformed[0].reason.contains("outside"),
            "{:?}",
            report.malformed[0]
        );
    }

    #[test]
    fn internal_parent_segments_stay_confined() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        fs::create_dir_all(dir.join("packages/inner")).unwrap();
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "inner", "path": "packages/../packages/inner", "profile": "product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert!(report.malformed.is_empty(), "{:?}", report.malformed);
        assert_eq!(report.entries.len(), 1);
    }

    #[test]
    fn symlinked_escape_is_malformed() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let outside = tmp.path().join("outside-target");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("forge.yaml"), "schema: 1\n").unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("sneaky")).unwrap();
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "sneaky", "path": "sneaky", "profile": "product", "lifecycle": "active"},
  {"id": "alpha", "path": "alpha", "profile": "product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.malformed[0].name, "sneaky");
        assert!(
            report.malformed[0].reason.contains("symlink"),
            "{:?}",
            report.malformed[0]
        );
    }

    #[test]
    fn absolute_entry_path_inside_root_is_accepted() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let absolute = format!(r#""path": "{}""#, dir.join("alpha").display());
        let body = clean_body().replace(r#""path": "alpha""#, &absolute);
        let registry = write_registry(&dir, &body);
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.entries.len(), 3);
        assert!(report
            .entries
            .iter()
            .any(|e| e.id == "alpha" && e.forge_yaml_present));
        assert!(report.malformed.is_empty());
    }

    #[test]
    fn blank_and_oversized_entry_fields_are_malformed_isolated() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "no-path", "path": "", "profile": "product", "lifecycle": "active"},
  {"id": "no-profile", "path": "alpha", "lifecycle": "active"},
  {"id": "Bad ID", "path": "alpha", "profile": "product", "lifecycle": "active"},
  {"id": "no-lifecycle", "path": "alpha", "profile": "product"},
  {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.entries.len(), 1, "only the clean entry survives");
        assert_eq!(report.malformed.len(), 4);
        let names: Vec<&str> = report.malformed.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["no-path", "no-profile", "Bad ID", "no-lifecycle"]
        );
    }

    #[test]
    fn freshness_classifies_from_the_document_mtime() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        let (age, freshness) = classify_freshness(&registry, 60, Utc::now()).unwrap();
        assert_eq!(freshness, FleetFreshness::Fresh, "age {age:?}");
        // Move the mtime 10 days into the past through the filesystem.
        let old = Utc::now() - chrono::Duration::days(10);
        let file_time = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(old.timestamp().max(0) as u64);
        let file = fs::OpenOptions::new().write(true).open(&registry).unwrap();
        file.set_times(fs::FileTimes::new().set_modified(file_time))
            .unwrap();
        drop(file);
        let (age, freshness) = classify_freshness(&registry, 60, Utc::now()).unwrap();
        assert_eq!(freshness, FleetFreshness::Stale);
        assert!(age.unwrap() > 60);
    }

    #[test]
    fn unconfigured_when_no_registry_path_is_given() {
        let report = observe_at(None, DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.freshness, FleetFreshness::Unconfigured);
        assert!(report.source.is_none());
        assert!(report.entries.is_empty());
        assert!(report.malformed.is_empty());
        let text = render_status_human(&report);
        assert!(text.contains("unconfigured"), "{text}");
        assert!(
            text.contains("local workflows continue unaffected")
                || text.contains("no fleet source"),
            "{text}"
        );
    }

    #[test]
    fn oversized_document_refuses() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let fat = format!(
            r#"{{"schema_version": 1, "projects": [{{"id": "alpha", "path": "alpha", "profile": "{}", "lifecycle": "active"}}]}}"#,
            "p".repeat(2_000_000)
        );
        let registry = write_registry(&dir, &fat);
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("bytes"), "{err}");
    }

    #[test]
    fn entry_count_over_the_bound_refuses() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let entries: Vec<String> = (0..=MAX_REGISTRY_ENTRIES)
            .map(|i| {
                format!(r#"{{"id": "p{i}", "path": "alpha", "profile": "x", "lifecycle": "y"}}"#)
            })
            .collect();
        let registry = write_registry(
            &dir,
            &format!(
                r#"{{"schema_version": 1, "projects": [{}]}}"#,
                entries.join(",")
            ),
        );
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("entries"), "{err}");
    }

    #[test]
    fn credential_shaped_fields_are_redacted_and_bounded() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            &format!(
                r#"{{"schema_version": 1, "projects": [
  {{ "id": "leak", "path": "alpha", "profile": "ghp_abcdefghijklmnopqrstuvwxyz0123456789", "lifecycle": "{}" }}
]}}"#,
                "l".repeat(2_000)
            ),
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        let entry = report.entries.iter().find(|e| e.id == "leak").unwrap();
        assert!(!entry.profile.contains("ghp_abcdef"), "{}", entry.profile);
        assert!(entry.profile.contains("[REDACTED]"), "{}", entry.profile);
        assert!(entry.lifecycle.chars().count() <= MAX_FIELD_CHARS);
        assert!(entry.lifecycle.ends_with("..."));
    }

    #[test]
    fn max_age_window_is_bounded() {
        assert_eq!(
            validate_max_age(0).unwrap_err().code(),
            "fleet-registry-invalid"
        );
        assert_eq!(
            validate_max_age(MAX_MAX_AGE_SECONDS + 1)
                .unwrap_err()
                .code(),
            "fleet-registry-invalid"
        );
        assert!(validate_max_age(DEFAULT_MAX_AGE_SECONDS).is_ok());
    }

    #[test]
    fn reports_are_deterministic_except_for_observed_at() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        // Pin the document mtime to a fixed instant 10s before the
        // observation clock below; a wall-clock mtime would make the age
        // assertions depend on the run time (and fail once the real clock
        // passes the fixed observations), so the filesystem — not the
        // clock — carries the determinism.
        let mtime = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(
                (now() - chrono::Duration::seconds(10)).timestamp().max(0) as u64,
            );
        let file = fs::OpenOptions::new().write(true).open(&registry).unwrap();
        file.set_times(fs::FileTimes::new().set_modified(mtime))
            .unwrap();
        drop(file);
        let earlier = now();
        let later = earlier + chrono::Duration::seconds(60);
        let a = observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), earlier).unwrap();
        let b = observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), later).unwrap();
        assert_eq!(a.entries, b.entries);
        assert_eq!(a.malformed, b.malformed);
        assert_eq!(a.source, b.source);
        assert_ne!(a.observed_at, b.observed_at);
        assert_eq!(a.age_seconds, Some(10));
        assert_eq!(b.age_seconds, Some(70));
    }

    #[test]
    fn inspect_entry_covers_found_malformed_and_missing() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "runner", "path": "../outside-root", "profile": "product", "lifecycle": "active"},
  {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        let entry = inspect_entry(&report, "alpha").unwrap();
        assert_eq!(entry.id, "alpha");
        let err = inspect_entry(&report, "runner").unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("malformed"), "{err}");
        let err = inspect_entry(&report, "ghost").unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("ghost"), "{err}");
    }

    #[test]
    fn entry_fields_are_the_shared_normalized_projection() {
        let entry = FleetEntry {
            id: "alpha".to_string(),
            path: "alpha".to_string(),
            profile: "rust-product".to_string(),
            lifecycle: "active".to_string(),
            adoption: Some("adopted".to_string()),
            forge_yaml_present: true,
            locally_registered: false,
            state: FleetState::Unmanaged,
        };
        let fields = entry_fields(&entry);
        let map: BTreeSet<String> = fields
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        for needle in [
            "path=alpha",
            "profile=rust-product",
            "lifecycle=active",
            "adoption=adopted",
            "forge_yaml=present",
            "state=unmanaged",
        ] {
            assert!(map.contains(needle), "{needle} missing in {map:?}");
        }
        let line = entry_summary_line(&entry);
        assert!(line.starts_with("fleet entry `alpha`: "), "{line}");
        assert!(line.contains("forge_yaml=present"), "{line}");
    }

    #[test]
    fn human_renderers_carry_the_required_fields() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        let report = observe_at(
            Some(&registry),
            DEFAULT_MAX_AGE_SECONDS,
            &ids(&["alpha"]),
            now(),
        )
        .unwrap();
        let list = render_report_human(&report);
        for needle in [
            FLEET_CONTRACT_VERSION,
            "freshness=fresh",
            "observed_at=",
            "alpha",
            "rust-product",
            "forge_yaml=present",
            "state=managed",
            "read-only",
        ] {
            assert!(list.contains(needle), "{needle} missing in {list}");
        }
        let status = render_status_human(&report);
        for needle in [FLEET_CONTRACT_VERSION, "health: ok", "max_age="] {
            assert!(status.contains(needle), "{needle} missing in {status}");
        }
        let entry = inspect_entry(&report, "alpha").unwrap();
        let inspect = render_entry_human(&report, entry);
        for needle in [
            FLEET_CONTRACT_VERSION,
            "id=alpha",
            "locally_registered=true",
            "state=managed",
        ] {
            assert!(inspect.contains(needle), "{needle} missing in {inspect}");
        }
        let block = render_list_block_human(&report);
        assert!(block.contains("fleet registry:"), "{block}");
        assert!(block.contains("fleet entry `alpha`"), "{block}");
    }

    #[test]
    fn malformed_entries_render_with_reasons_and_never_silently_drop() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "runner", "path": "../../etc", "profile": "product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.declared_count(), 1);
        assert!(report.entries.is_empty());
        let text = render_report_human(&report);
        assert!(text.contains("malformed"), "{text}");
        assert!(text.contains("runner"), "{text}");
        let health = health_json(&report);
        assert_eq!(health["malformed"], serde_json::json!(1));
        assert_eq!(health["declared_count"], serde_json::json!(1));
    }

    #[test]
    fn resolve_registry_path_prefers_the_flag_then_the_env() {
        // The env var is process-global; test the two orders without
        // racing other tests by restoring it afterwards.
        std::env::remove_var(WORKSPACE_REGISTRY_ENV);
        assert_eq!(resolve_registry_path(None), None);
        let explicit = PathBuf::from("/flag/projects.json");
        assert_eq!(
            resolve_registry_path(Some(&explicit)),
            Some(explicit.clone())
        );
        std::env::set_var(WORKSPACE_REGISTRY_ENV, "/env/projects.json");
        assert_eq!(
            resolve_registry_path(None),
            Some(PathBuf::from("/env/projects.json"))
        );
        assert_eq!(resolve_registry_path(Some(&explicit)), Some(explicit));
        std::env::set_var(WORKSPACE_REGISTRY_ENV, "   ");
        assert_eq!(resolve_registry_path(None), None);
        std::env::remove_var(WORKSPACE_REGISTRY_ENV);
    }
}
