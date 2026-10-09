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
    use crate::feature::{expected_receipt, inspect_feature, TESTED_VERSION};
    use crate::registry::Registry;
    use crate::upgrade::contract::UPGRADE_CONTRACT_VERSION;
    use crate::upgrade::engine::{apply_upgrade, plan_for_dir, run_fleet};

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

    fn age_feature(proj: &Path, id: &str, old: &str) {
        let text = fs::read_to_string(proj.join("forge.yaml")).unwrap();
        let aged = text
            .replace(&format!("{id}: 0.1.0"), &format!("{id}: {old}"))
            .replace(&format!("{id}: \"0.1.0\""), &format!("{id}: {old}"));
        assert_ne!(aged, text, "aging must change the manifest");
        fs::write(proj.join("forge.yaml"), &aged).unwrap();
        let receipt = proj.join(format!(".forge/features/{id}.receipt"));
        if receipt.is_file() {
            let receipt_text = fs::read_to_string(&receipt).unwrap();
            fs::write(
                &receipt,
                receipt_text.replace("version: 0.1.0", &format!("version: {old}")),
            )
            .unwrap();
        }
    }

    #[test]
    fn plan_describes_versions_assets_validation_and_recovery() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(
            &proj,
            &rust_manifest("plan-app", "features:\n  auth: \"0.0.9\"\n"),
        );
        let plan = plan_for_dir(&proj, None).unwrap();
        assert_eq!(plan.project_id, "plan-app");
        assert_eq!(plan.contract, UPGRADE_CONTRACT_VERSION);
        assert_eq!(plan.steps.len(), 1);
        let step = &plan.steps[0];
        assert_eq!(step.feature, "auth");
        assert_eq!(step.old_version.as_deref(), Some("0.0.9"));
        assert_eq!(step.new_version, TESTED_VERSION);
        assert_eq!(step.action, "upgrade");
        assert!(
            step.kinds.contains(&"package".to_string()),
            "{:?}",
            step.kinds
        );
        assert!(step.assets.contains(&"forge.yaml".to_string()));
        assert!(!step.validators.is_empty());
        assert!(step.reversible);
        assert!(step.recovery.contains("reversible"), "{}", step.recovery);
    }

    #[test]
    fn postgres_plan_marks_schema_irreversible_with_declared_strategy() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(
            &proj,
            &rust_manifest("schema-app", "features:\n  postgres: \"0.0.9\"\n"),
        );
        let plan = plan_for_dir(&proj, None).unwrap();
        let step = plan.steps.iter().find(|s| s.feature == "postgres").unwrap();
        assert!(
            step.kinds.contains(&"schema".to_string()),
            "{:?}",
            step.kinds
        );
        assert!(!step.reversible);
        assert_eq!(
            step.migration_strategy,
            "manifest-repin+manual-schema-review"
        );
        assert!(
            step.recovery.contains("does NOT reverse"),
            "{}",
            step.recovery
        );
    }

    #[test]
    fn drifted_receipt_blocks_with_handoff_and_preserves_files() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(
            &proj,
            &rust_manifest("drift-app", "features:\n  auth: \"0.0.9\"\n"),
        );
        let mut reg = open_registry(&tmp);
        reg.register(&proj, None).unwrap();
        // Model a clean older install: write the receipt matching the
        // installed version, then let the operator edit it.
        let receipt = proj.join(".forge/features/auth.receipt");
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        let descriptor = inspect_feature("auth").unwrap();
        fs::write(&receipt, expected_receipt(&descriptor, "0.0.9")).unwrap();
        fs::write(&receipt, "operator custom checkout notes\n").unwrap();
        let before = fs::read_to_string(proj.join("forge.yaml")).unwrap();

        let err = apply_upgrade(&mut reg, proj.to_str().unwrap(), None).expect_err("drift blocks");
        assert_eq!(err.code(), "feature-ownership-conflict");
        let text = err.to_string();
        assert!(text.contains("semantic-conflict handoff"), "{text}");
        assert!(text.contains("forge spec generate"), "{text}");
        assert_eq!(fs::read_to_string(proj.join("forge.yaml")).unwrap(), before);
    }

    #[test]
    fn satisfied_upgrade_rewrites_no_files() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(
            &proj,
            &rust_manifest("sat-app", "features:\n  auth: \"0.1.0\"\n"),
        );
        let mut reg = open_registry(&tmp);
        reg.register(&proj, None).unwrap();
        let journal_before = reg.journal_entries().unwrap().len();

        let outcome = apply_upgrade(&mut reg, proj.to_str().unwrap(), None).unwrap();
        assert!(!outcome.changed);
        assert!(outcome.files_changed.is_empty());
        assert!(
            outcome.note.contains("already satisfied"),
            "{}",
            outcome.note
        );
        assert_eq!(reg.journal_entries().unwrap().len(), journal_before);
    }

    #[test]
    fn missing_requested_feature_installs_automatically() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(&proj, &rust_manifest("missing-app", ""));
        let mut reg = open_registry(&tmp);

        let outcome = apply_upgrade(&mut reg, proj.to_str().unwrap(), Some("privacy")).unwrap();
        assert!(outcome.changed);
        assert_eq!(
            outcome.features.get("privacy").map(String::as_str),
            Some(TESTED_VERSION)
        );
        let record = reg.inspect("missing-app").unwrap();
        assert_eq!(
            record.features.get("privacy").map(String::as_str),
            Some(TESTED_VERSION)
        );
    }

    #[test]
    fn unknown_requested_feature_fails_before_edits() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        let text = rust_manifest("unknown-app", "");
        write_project(&proj, &text);
        let mut reg = open_registry(&tmp);

        let err = apply_upgrade(&mut reg, proj.to_str().unwrap(), Some("nosuch"))
            .expect_err("unknown feature");
        assert_eq!(err.code(), "unknown-feature");
        assert_eq!(fs::read_to_string(proj.join("forge.yaml")).unwrap(), text);
    }

    #[test]
    fn fleet_completes_two_projects_with_per_project_journals() {
        let tmp = TempDir::new().unwrap();
        let first = tmp.path().join("first");
        let second = tmp.path().join("second");
        write_project(
            &first,
            &rust_manifest("fleet-one", "features:\n  auth: \"0.0.9\"\n"),
        );
        write_project(
            &second,
            &rust_manifest("fleet-two", "features:\n  telemetry: \"0.0.9\"\n"),
        );
        let mut reg = open_registry(&tmp);
        reg.register(&first, None).unwrap();
        reg.register(&second, None).unwrap();
        // Model clean older installs for both.
        for (proj, id) in [(&first, "auth"), (&second, "telemetry")] {
            let descriptor = inspect_feature(id).unwrap();
            let path = proj.join(format!(".forge/features/{id}.receipt"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, expected_receipt(&descriptor, "0.0.9")).unwrap();
        }

        let report = run_fleet(&mut reg, None, false).unwrap();
        assert_eq!(
            report.selection,
            vec!["fleet-one".to_string(), "fleet-two".to_string()]
        );
        assert_eq!(report.succeeded, 2);
        assert_eq!(report.failed, 0);
        assert_eq!(report.blocked, 0);
        assert!(report.healthy());
        for entry in &report.entries {
            assert_eq!(entry.status, "success");
            assert!(entry.changed);
            assert!(!entry.validation.is_empty());
            assert!(!entry.recovery.is_empty());
        }
        assert_eq!(
            reg.inspect("fleet-one")
                .unwrap()
                .features
                .get("auth")
                .map(String::as_str),
            Some(TESTED_VERSION)
        );
        assert_eq!(
            reg.inspect("fleet-two")
                .unwrap()
                .features
                .get("telemetry")
                .map(String::as_str),
            Some(TESTED_VERSION)
        );
        let kinds: Vec<String> = reg
            .journal_entries()
            .unwrap()
            .into_iter()
            .filter(|e| e.kind == "upgrade")
            .map(|e| e.state)
            .collect();
        assert!(kinds.iter().all(|s| s == "done"), "{kinds:?}");
    }

    #[test]
    fn fleet_isolates_failure_without_wholesale_success() {
        let tmp = TempDir::new().unwrap();
        let good = tmp.path().join("good");
        let bad = tmp.path().join("bad");
        write_project(
            &good,
            &rust_manifest("fleet-good", "features:\n  auth: \"0.0.9\"\n"),
        );
        // `bad` carries two outdated features; the second has user-owned
        // edits, so the first applies and the second blocks: partial state.
        write_project(
            &bad,
            &rust_manifest(
                "fleet-bad",
                "features:\n  auth: \"0.0.9\"\n  telemetry: \"0.0.9\"\n",
            ),
        );
        let mut reg = open_registry(&tmp);
        reg.register(&good, None).unwrap();
        reg.register(&bad, None).unwrap();
        for (proj, id) in [(&good, "auth"), (&bad, "auth"), (&bad, "telemetry")] {
            let descriptor = inspect_feature(id).unwrap();
            let path = proj.join(format!(".forge/features/{id}.receipt"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, expected_receipt(&descriptor, "0.0.9")).unwrap();
        }
        fs::write(
            bad.join(".forge/features/telemetry.receipt"),
            "custom notes\n",
        )
        .unwrap();

        let report = run_fleet(&mut reg, None, false).unwrap();
        assert!(!report.healthy());
        let good_entry = report
            .entries
            .iter()
            .find(|e| e.project_id == "fleet-good")
            .unwrap();
        assert_eq!(good_entry.status, "success");
        let bad_entry = report
            .entries
            .iter()
            .find(|e| e.project_id == "fleet-bad")
            .unwrap();
        // Nothing applied for `bad` yet (auth sorts before telemetry but the
        // precondition sweep blocks before any mutation), so this is a
        // blocked entry with preserved files and a recovery note.
        assert_eq!(bad_entry.status, "blocked");
        assert!(!bad_entry.changed);
        assert!(!bad_entry.recovery.is_empty());
        // The healthy project still completed despite its sibling blocking.
        assert_eq!(
            reg.inspect("fleet-good")
                .unwrap()
                .features
                .get("auth")
                .map(String::as_str),
            Some(TESTED_VERSION)
        );
    }

    #[test]
    fn fleet_retry_skips_completed_and_replans_on_changed_preconditions() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        write_project(
            &proj,
            &rust_manifest("retry-app", "features:\n  auth: \"0.0.9\"\n"),
        );
        let mut reg = open_registry(&tmp);
        reg.register(&proj, None).unwrap();
        let descriptor = inspect_feature("auth").unwrap();
        let receipt = proj.join(".forge/features/auth.receipt");
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        fs::write(&receipt, expected_receipt(&descriptor, "0.0.9")).unwrap();

        let first = run_fleet(&mut reg, None, false).unwrap();
        assert_eq!(first.succeeded, 1);
        let manifest_after_first = fs::read(proj.join("forge.yaml")).unwrap();

        // Retry: already satisfied, so skipped without rewriting files.
        let second = run_fleet(&mut reg, None, false).unwrap();
        assert_eq!(second.skipped, 1);
        assert_eq!(second.succeeded, 0);
        assert!(second.healthy());
        assert_eq!(
            fs::read(proj.join("forge.yaml")).unwrap(),
            manifest_after_first
        );

        // Changed preconditions (operator edits the receipt) cause fresh
        // re-planning: age auth back and drift the receipt, then retry must
        // block instead of blindly reporting success.
        age_feature(&proj, "auth", "0.0.8");
        fs::write(&receipt, "operator edits after upgrade\n").unwrap();
        let third = run_fleet(&mut reg, None, false).unwrap();
        assert_eq!(third.blocked, 1);
        assert!(!third.healthy());
    }

    #[test]
    fn manifest_sections_survive_upgrade_edits() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        let text = "schema: 1\nproject:\n  id: keep-up\n  name: Keep\n  profile: rust-web\n  maturity: L2\nruntime:\n  language: rust\n  version: stable\ndeployment:\n  type: docker\n  target: home-server-01\ndocs:\n  source_language: en\nfeatures:\n  auth: \"0.0.9\"\n";
        write_project(&proj, text);
        let mut reg = open_registry(&tmp);
        reg.register(&proj, None).unwrap();
        let descriptor = inspect_feature("auth").unwrap();
        let receipt = proj.join(".forge/features/auth.receipt");
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        fs::write(&receipt, expected_receipt(&descriptor, "0.0.9")).unwrap();

        apply_upgrade(&mut reg, proj.to_str().unwrap(), None).unwrap();
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            &fs::read(proj.join("forge.yaml")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest.features.get("auth").map(String::as_str),
            Some(TESTED_VERSION)
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
        let _ = BTreeMap::<String, String>::new();
    }
}
