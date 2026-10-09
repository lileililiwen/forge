//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    use crate::core::manifest::Manifest;
    use crate::feature::contract::TESTED_VERSION;
    use crate::feature::engine::{
        add_feature, descriptor, edit_manifest_features, expected_receipt, feature_catalog,
        inspect_feature, lookup, receipt_path, remove_feature, resolve_plan,
        resolve_plan_with_catalog, upgrade_feature,
    };
    use crate::feature::model::FeatureDescriptor;
    use crate::registry::Registry;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    fn synthetic(
        id: &str,
        depends: &[&str],
        conflicts: &[&str],
        profiles: &[&str],
    ) -> FeatureDescriptor {
        FeatureDescriptor {
            id: id.to_string(),
            version: TESTED_VERSION.to_string(),
            compatible_profiles: profiles.iter().map(|s| s.to_string()).collect(),
            depends: depends.iter().map(|s| s.to_string()).collect(),
            conflicts: conflicts.iter().map(|s| s.to_string()).collect(),
            install_strategy: "generator".to_string(),
            upgrade_strategy: "manifest-repin".to_string(),
            validation: Vec::new(),
            documentation: String::new(),
            tests: String::new(),
        }
    }

    fn write_project(dir: &Path, text: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("forge.yaml"), text).unwrap();
    }

    fn open_registry(dir: &TempDir) -> Registry {
        Registry::open(&dir.path().join("registry.db")).unwrap()
    }

    fn rust_manifest(id: &str, features: &str) -> String {
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n{features}"
        )
    }

    #[test]
    fn catalog_lists_brief_features_with_tested_mappings() {
        let catalog = feature_catalog();
        let ids: Vec<&str> = catalog.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "admin",
                "analytics",
                "audit",
                "auth",
                "background-jobs",
                "billing",
                "content",
                "email",
                "health-check",
                "i18n",
                "notifications",
                "postgres",
                "privacy",
                "rate-limit",
                "redis",
                "search",
                "storage",
                "telemetry",
            ]
        );
        for f in &catalog {
            assert_eq!(f.version, TESTED_VERSION, "{}", f.id);
            assert!(!f.compatible_profiles.is_empty(), "{}", f.id);
            assert!(!f.install_strategy.trim().is_empty(), "{}", f.id);
            assert_eq!(f.upgrade_strategy, "manifest-repin", "{}", f.id);
            assert!(!f.documentation.trim().is_empty(), "{}", f.id);
            assert!(!f.tests.trim().is_empty(), "{}", f.id);
        }
        let admin = lookup(&catalog, "admin").unwrap();
        assert_eq!(admin.depends, vec!["auth".to_string()]);
        assert!(admin.compatible_profiles.contains(&"rust-web".to_string()));
        // flutter-app is client-only: server features stay unsupported there.
        let postgres = lookup(&catalog, "postgres").unwrap();
        assert!(!postgres
            .compatible_profiles
            .contains(&"flutter-app".to_string()));
        assert!(inspect_feature("billing").is_ok());
        assert_eq!(
            inspect_feature("nosuch").expect_err("unknown").code(),
            "unknown-feature"
        );
    }

    #[test]
    fn resolve_admin_pulls_auth_first_with_exact_versions() {
        let plan = resolve_plan("rust-web", &strings(&["admin"])).unwrap();
        let steps: Vec<(&str, &str)> = plan
            .steps
            .iter()
            .map(|s| (s.feature.as_str(), s.version.as_str()))
            .collect();
        assert_eq!(steps, vec![("auth", "0.1.0"), ("admin", "0.1.0")]);
        assert!(plan.validators.contains(&"AUTH-001".to_string()));
    }

    #[test]
    fn resolve_unknown_feature_fails_before_edits() {
        let err = resolve_plan("rust-web", &strings(&["nosuch"])).expect_err("unknown");
        assert_eq!(err.code(), "unknown-feature");
    }

    #[test]
    fn resolve_unsupported_profile_reports_without_inventing() {
        let err = resolve_plan("flutter-app", &strings(&["postgres"])).expect_err("unsupported");
        assert_eq!(err.code(), "incompatible-feature");
        let text = err.to_string();
        assert!(text.contains("no implementation"), "{text}");
        assert!(text.contains("no files were changed"), "{text}");
    }

    #[test]
    fn resolve_conflict_identifies_blocking_edges() {
        let catalog = vec![
            synthetic("alpha", &[], &["beta"], &["rust-web"]),
            synthetic("beta", &[], &[], &["rust-web"]),
        ];
        let err = resolve_plan_with_catalog("rust-web", &strings(&["alpha", "beta"]), &catalog)
            .expect_err("conflict");
        assert_eq!(err.code(), "incompatible-feature");
        let text = err.to_string();
        assert!(text.contains("alpha"), "{text}");
        assert!(text.contains("beta"), "{text}");
    }

    #[test]
    fn resolve_cycle_identifies_blocking_edges() {
        let catalog = vec![
            synthetic("alpha", &["beta"], &[], &["rust-web"]),
            synthetic("beta", &["alpha"], &[], &["rust-web"]),
        ];
        let err = resolve_plan_with_catalog("rust-web", &strings(&["alpha"]), &catalog)
            .expect_err("cycle");
        assert_eq!(err.code(), "incompatible-feature");
        let text = err.to_string();
        assert!(text.contains("cycle"), "{text}");
        assert!(text.contains("alpha"), "{text}");
        assert!(text.contains("beta"), "{text}");
    }

    #[test]
    fn add_then_upgrade_roundtrip_keeps_source_manifest_registry_agreeing() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(&proj, &rust_manifest("round-trip", ""));
        let mut reg = open_registry(&tmp);

        let added = add_feature(&mut reg, proj.to_str().unwrap(), "admin", None).unwrap();
        assert!(added.changed);
        assert_eq!(
            added.features.get("auth").map(String::as_str),
            Some("0.1.0")
        );
        assert_eq!(
            added.features.get("admin").map(String::as_str),
            Some("0.1.0")
        );
        assert!(added.plan.validators.contains(&"AUTH-001".to_string()));
        assert!(proj.join(".forge/features/auth.receipt").is_file());
        assert!(proj.join(".forge/features/admin.receipt").is_file());
        // Registry agrees with the manifest.
        let record = reg.inspect("round-trip").unwrap();
        assert_eq!(record.features, added.features);

        // Simulate an older installed version, then upgrade.
        let mut updates = BTreeMap::new();
        updates.insert("auth".to_string(), Some("0.0.9".to_string()));
        let (manifest, path) = Manifest::load_from_dir(&proj, None).unwrap();
        let before = fs::read(&path).unwrap();
        let next = edit_manifest_features(&before, &path, &updates).unwrap();
        fs::write(&path, &next).unwrap();
        // Receipt still expects the tested version, so refresh it to the
        // older spelling to model a clean older install.
        let auth = lookup(&feature_catalog(), "auth").unwrap().clone();
        fs::write(
            receipt_path(&proj, "auth"),
            expected_receipt(&auth, "0.0.9"),
        )
        .unwrap();
        let _ = manifest;

        let upgraded = upgrade_feature(&mut reg, proj.to_str().unwrap(), "auth", None).unwrap();
        assert!(upgraded.changed);
        assert_eq!(
            upgraded.features.get("auth").map(String::as_str),
            Some("0.1.0")
        );
        let record = reg.inspect("round-trip").unwrap();
        assert_eq!(record.features, upgraded.features);
    }

    #[test]
    fn add_exact_version_is_noop_without_duplicate_registration() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(
            &proj,
            &rust_manifest("noop-add", "features:\n  auth: \"0.1.0\"\n"),
        );
        let mut reg = open_registry(&tmp);
        reg.register(&proj, None).unwrap();
        let journal_before = reg.journal_entries().unwrap().len();

        let outcome = add_feature(&mut reg, proj.to_str().unwrap(), "auth", None).unwrap();
        assert!(!outcome.changed);
        assert!(
            outcome.note.contains("already installed"),
            "{}",
            outcome.note
        );
        assert!(outcome.files_changed.is_empty());
        assert_eq!(reg.journal_entries().unwrap().len(), journal_before);
    }

    #[test]
    fn unknown_version_fails_before_edits() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        let text = rust_manifest("old-ver", "");
        write_project(&proj, &text);
        let mut reg = open_registry(&tmp);

        let err = add_feature(&mut reg, proj.to_str().unwrap(), "auth", Some("9.9.9"))
            .expect_err("unknown version");
        assert_eq!(err.code(), "incompatible-feature");
        assert_eq!(fs::read_to_string(proj.join("forge.yaml")).unwrap(), text);
        assert!(!proj.join(".forge").exists());
    }

    #[test]
    fn remove_blocked_by_reverse_dependency_preserves_files() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(
            &proj,
            &rust_manifest(
                "dep-block",
                "features:\n  auth: \"0.1.0\"\n  admin: \"0.1.0\"\n",
            ),
        );
        let mut reg = open_registry(&tmp);
        reg.register(&proj, None).unwrap();
        // Clean receipts model an installed pair.
        for id in ["auth", "admin"] {
            let descriptor = lookup(&feature_catalog(), id).unwrap().clone();
            let path = receipt_path(&proj, id);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, expected_receipt(&descriptor, "0.1.0")).unwrap();
        }
        let before_manifest = fs::read_to_string(proj.join("forge.yaml")).unwrap();

        let err =
            remove_feature(&mut reg, proj.to_str().unwrap(), "auth").expect_err("reverse dep");
        assert_eq!(err.code(), "incompatible-feature");
        let text = err.to_string();
        assert!(text.contains("admin"), "{text}");
        assert_eq!(
            fs::read_to_string(proj.join("forge.yaml")).unwrap(),
            before_manifest
        );
        assert!(proj.join(".forge/features/auth.receipt").is_file());
    }

    #[test]
    fn remove_blocked_by_user_edited_receipt_preserves_files() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(
            &proj,
            &rust_manifest("owned-block", "features:\n  auth: \"0.1.0\"\n"),
        );
        let mut reg = open_registry(&tmp);
        reg.register(&proj, None).unwrap();
        let path = receipt_path(&proj, "auth");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "user notes about auth setup\n").unwrap();
        let before_manifest = fs::read_to_string(proj.join("forge.yaml")).unwrap();

        let err = remove_feature(&mut reg, proj.to_str().unwrap(), "auth").expect_err("ownership");
        assert_eq!(err.code(), "feature-ownership-conflict");
        assert_eq!(
            fs::read_to_string(proj.join("forge.yaml")).unwrap(),
            before_manifest
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "user notes about auth setup\n"
        );
    }

    #[test]
    fn manifest_sections_survive_feature_edits() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        let text = "schema: 1\nproject:\n  id: keep-sections\n  name: Keep\n  profile: rust-web\n  maturity: L2\nruntime:\n  language: rust\n  version: stable\ndeployment:\n  type: docker\n  target: home-server-01\ndocs:\n  source_language: en\n";
        write_project(&proj, text);
        let mut reg = open_registry(&tmp);

        add_feature(&mut reg, proj.to_str().unwrap(), "telemetry", None).unwrap();
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            &fs::read(proj.join("forge.yaml")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest.features.get("telemetry").map(String::as_str),
            Some("0.1.0")
        );
        assert_eq!(
            manifest.project.maturity,
            Some(crate::core::manifest::Maturity::L2)
        );
        assert_eq!(
            manifest.deployment.as_ref().and_then(|d| d.target.clone()),
            Some("home-server-01".to_string())
        );
        assert_eq!(
            manifest
                .docs
                .as_ref()
                .and_then(|d| d.source_language.clone()),
            Some("en".to_string())
        );

        remove_feature(&mut reg, proj.to_str().unwrap(), "telemetry").unwrap();
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            &fs::read(proj.join("forge.yaml")).unwrap(),
        )
        .unwrap();
        assert!(manifest.features.is_empty());
        assert_eq!(
            manifest.deployment.as_ref().and_then(|d| d.target.clone()),
            Some("home-server-01".to_string())
        );
    }
}
