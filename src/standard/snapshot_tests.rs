//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use crate::standard::layout::{
        CI_PATH, COMPOSE_PATH, DETERMINISTIC_TIMESTAMP, PROFILE_PATH, QUALITY_PATH, RECEIPT_PATH,
        STANDARD_DIR, VERIFY_PATH,
    };
    use crate::standard::model::{
        DiffChange, FileState, PackAssetOrigin, PackDescriptor, PackEvidence, PackSupportState,
        SnapshotState,
    };
    use crate::standard::snapshot::{
        all_packs, check_snapshot, diff_snapshot, inspect_pack, pack_asset_digest, parse_pack_spec,
        receipt_text, render_snapshot, resolve_asset_origin, select_for_generation, staged_files,
        upgrade_snapshot,
    };

    fn baseline(spec: &str) -> PackDescriptor {
        inspect_pack(spec).unwrap()
    }

    fn write(dir: &Path, rel: &str, bytes: &[u8]) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(bytes).unwrap();
    }

    fn materialize(id: &str, profile: &str, spec: &str, dir: &Path) {
        let pack = inspect_pack(spec).unwrap();
        let snapshot = render_snapshot(id, profile, &pack, DETERMINISTIC_TIMESTAMP, true).unwrap();
        for (path, content) in &snapshot.files {
            write(dir, path, content.as_bytes());
        }
        write(
            dir,
            RECEIPT_PATH,
            receipt_text(&snapshot.receipt).unwrap().as_bytes(),
        );
    }

    #[test]
    fn catalog_is_versioned_and_lifecycle_closed() {
        let packs = all_packs();
        let states: Vec<PackSupportState> = packs.iter().map(|p| p.support_state).collect();
        assert!(states.contains(&PackSupportState::Supported));
        assert!(states.contains(&PackSupportState::Deprecated));
        assert!(states.contains(&PackSupportState::Proposed));
        for pack in &packs {
            assert_eq!(pack.asset_digest.len(), 64, "{}", pack.version);
            assert!(!pack.compatible_profiles.is_empty());
            assert_eq!(pack.asset_digest, pack_asset_digest(&pack.files));
        }
        // Only supported+verified versions are selectable.
        for pack in &packs {
            let expected = pack.support_state == PackSupportState::Supported
                && pack.evidence == PackEvidence::Verified;
            assert_eq!(pack.is_selectable(), expected, "{}", pack.version);
        }
    }

    #[test]
    fn selection_requires_explicit_supported_compatible_pack() {
        // Supported + compatible is accepted verbatim.
        assert_eq!(
            select_for_generation("rust-web", "baseline-service@1.1.0").unwrap(),
            "baseline-service@1.1.0"
        );
        // Non-selectable versions refuse even when compatible.
        for spec in ["baseline-service@0.9.0", "baseline-service@2.0.0"] {
            let err = select_for_generation("rust-web", spec).expect_err("must refuse");
            assert_eq!(err.code(), "standard-invalid");
        }
        // Client profiles have no implicit pack: an explicit selection for
        // an incompatible profile refuses rather than guessing.
        let err = select_for_generation("react-web", "baseline-service@1.1.0")
            .expect_err("incompatible profile must refuse");
        assert_eq!(err.code(), "standard-invalid");
        // Unknown selectors refuse by name.
        let err = select_for_generation("rust-web", "nope@1.0.0").expect_err("must refuse");
        assert_eq!(err.code(), "standard-invalid");
    }

    #[test]
    fn pack_spec_requires_both_halves() {
        assert_eq!(
            parse_pack_spec("baseline-service@1.1.0").unwrap(),
            ("baseline-service".to_string(), "1.1.0".to_string())
        );
        for bad in ["baseline-service", "@1.0.0", "baseline-service@", ""] {
            let err = parse_pack_spec(bad).expect_err("must refuse");
            assert_eq!(err.code(), "standard-invalid");
        }
    }

    #[test]
    fn unknown_pack_or_version_is_typed() {
        for spec in ["nope@1.0.0", "baseline-service@9.9.9"] {
            let err = inspect_pack(spec).expect_err("must refuse");
            assert_eq!(err.code(), "standard-invalid");
            assert!(err.to_string().contains(spec) || err.to_string().contains("unknown"));
        }
    }

    #[test]
    fn rendering_is_deterministic_and_substitutes_inputs() {
        let pack = baseline("baseline-service@1.1.0");
        let a = render_snapshot("demo", "rust-web", &pack, DETERMINISTIC_TIMESTAMP, false).unwrap();
        let b = render_snapshot("demo", "rust-web", &pack, DETERMINISTIC_TIMESTAMP, false).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.asset_digest, pack.asset_digest);
        let profile_yaml = a
            .files
            .iter()
            .find(|(p, _)| p == PROFILE_PATH)
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(profile_yaml.contains("pack: baseline-service"));
        assert!(profile_yaml.contains("profile: rust-web"));
        assert!(profile_yaml.contains("project: demo"));
        let verify = a
            .files
            .iter()
            .find(|(p, _)| p == VERIFY_PATH)
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(verify.contains("cargo test"), "{verify}");
        // Different inputs change bytes; different ids change the receipt.
        let other =
            render_snapshot("other", "rust-web", &pack, DETERMINISTIC_TIMESTAMP, false).unwrap();
        assert_ne!(a.files, other.files);
    }

    #[test]
    fn incompatible_profile_and_non_selectable_pack_refuse() {
        let pack = baseline("baseline-service@1.1.0");
        let err = render_snapshot("demo", "flutter-app", &pack, DETERMINISTIC_TIMESTAMP, false)
            .expect_err("incompatible profile must refuse");
        assert_eq!(err.code(), "standard-invalid");
        assert!(err.to_string().contains("does not support profile"));

        for spec in ["baseline-service@0.9.0", "baseline-service@2.0.0"] {
            let pinned = baseline(spec);
            let err = render_snapshot("demo", "rust-web", &pinned, DETERMINISTIC_TIMESTAMP, false)
                .expect_err("non-selectable pack must refuse for new generation");
            assert_eq!(err.code(), "standard-invalid");
            // The pinned path still resolves it for check/upgrade.
            assert!(
                render_snapshot("demo", "rust-web", &pinned, DETERMINISTIC_TIMESTAMP, true).is_ok()
            );
        }
    }

    #[test]
    fn external_source_falls_back_locally_and_never_fetches() {
        let pack = baseline("baseline-service@1.1.0");
        let (origin, notes) = resolve_asset_origin(&pack, None);
        assert_eq!(origin, PackAssetOrigin::LocalFallback);
        assert!(notes[0].contains("never fetched"), "{notes:?}");
        let missing = PathBuf::from("/nonexistent/standard-templates");
        let (origin, notes) = resolve_asset_origin(&pack, Some(&missing));
        assert_eq!(origin, PackAssetOrigin::LocalFallback);
        assert!(notes[0].contains("unavailable"), "{notes:?}");
    }

    #[test]
    fn check_reports_absent_rendered_and_modified() {
        let tmp = tempfile::TempDir::new().unwrap();
        let absent = check_snapshot(tmp.path()).unwrap();
        assert_eq!(absent.state, SnapshotState::Absent);
        assert!(absent.issues[0].contains("no Forge-owned"));

        materialize("demo", "rust-web", "baseline-service@1.1.0", tmp.path());
        let rendered = check_snapshot(tmp.path()).unwrap();
        assert_eq!(rendered.state, SnapshotState::Rendered);
        assert!(rendered.files.iter().all(|f| f.state == FileState::Present));
        assert_eq!(rendered.asset_digest_matches, Some(true));

        let verify = fs::read_to_string(tmp.path().join(VERIFY_PATH)).unwrap();
        write(
            tmp.path(),
            VERIFY_PATH,
            format!("{verify}\n# user edit\n").as_bytes(),
        );
        let modified = check_snapshot(tmp.path()).unwrap();
        assert_eq!(modified.state, SnapshotState::Modified);
        assert!(modified
            .files
            .iter()
            .any(|f| f.path == VERIFY_PATH && f.state == FileState::Modified));
    }

    #[test]
    fn diff_and_upgrade_preserve_unrelated_files_and_refresh_receipt() {
        let tmp = tempfile::TempDir::new().unwrap();
        materialize("demo", "rust-web", "baseline-service@1.0.0", tmp.path());
        write(tmp.path(), "user-notes.txt", b"mine");
        let plan = diff_snapshot(tmp.path(), "baseline-service@1.1.0").unwrap();
        assert!(plan.conflicts.is_empty(), "{plan:?}");
        assert!(plan
            .entries
            .iter()
            .any(|e| e.path == VERIFY_PATH && e.change == DiffChange::Updated));

        // Confirmation is mandatory.
        let err = upgrade_snapshot(
            tmp.path(),
            "baseline-service@1.1.0",
            false,
            false,
            DETERMINISTIC_TIMESTAMP,
        )
        .expect_err("upgrade without --confirm must refuse");
        assert_eq!(err.code(), "standard-invalid");
        // The unedited upgrade writes only owned files.
        let report = upgrade_snapshot(
            tmp.path(),
            "baseline-service@1.1.0",
            true,
            false,
            DETERMINISTIC_TIMESTAMP,
        )
        .unwrap();
        assert!(report.written.contains(&VERIFY_PATH.to_string()));
        assert_eq!(
            fs::read_to_string(tmp.path().join("user-notes.txt")).unwrap(),
            "mine"
        );
        let after = check_snapshot(tmp.path()).unwrap();
        assert_eq!(after.state, SnapshotState::Rendered);
        assert_eq!(after.version.as_deref(), Some("1.1.0"));
    }

    #[test]
    fn modified_owned_file_is_a_conflict_and_writes_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        materialize("demo", "rust-web", "baseline-service@1.0.0", tmp.path());
        let verify = fs::read_to_string(tmp.path().join(VERIFY_PATH)).unwrap();
        let edited = format!("{verify}\n# local edit\n");
        write(tmp.path(), VERIFY_PATH, edited.as_bytes());

        let plan = diff_snapshot(tmp.path(), "baseline-service@1.1.0").unwrap();
        assert!(
            plan.conflicts.contains(&VERIFY_PATH.to_string()),
            "{plan:?}"
        );

        let err = upgrade_snapshot(
            tmp.path(),
            "baseline-service@1.1.0",
            true,
            false,
            DETERMINISTIC_TIMESTAMP,
        )
        .expect_err("modified owned file must conflict");
        assert_eq!(err.code(), "standard-invalid");
        assert_eq!(
            fs::read_to_string(tmp.path().join(VERIFY_PATH)).unwrap(),
            edited
        );

        // A provided resolution (--force) replaces the modified file.
        let report = upgrade_snapshot(
            tmp.path(),
            "baseline-service@1.1.0",
            true,
            true,
            DETERMINISTIC_TIMESTAMP,
        )
        .unwrap();
        assert!(report.forced.contains(&VERIFY_PATH.to_string()));
        assert_ne!(
            fs::read_to_string(tmp.path().join(VERIFY_PATH)).unwrap(),
            edited
        );
    }

    #[test]
    fn foreign_file_collision_is_preserved_and_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        materialize("demo", "rust-web", "baseline-service@1.0.0", tmp.path());
        write(tmp.path(), PROFILE_PATH, b"mine: true\n");
        let err = upgrade_snapshot(
            tmp.path(),
            "baseline-service@1.1.0",
            true,
            false,
            DETERMINISTIC_TIMESTAMP,
        )
        .expect_err("foreign collision must refuse");
        assert_eq!(err.code(), "standard-invalid");
        assert_eq!(
            fs::read_to_string(tmp.path().join(PROFILE_PATH)).unwrap(),
            "mine: true\n"
        );
    }

    #[test]
    fn upgrade_without_receipt_refuses() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = diff_snapshot(tmp.path(), "baseline-service@1.1.0").unwrap_err();
        assert_eq!(err.code(), "standard-invalid");
        let err = upgrade_snapshot(
            tmp.path(),
            "baseline-service@1.1.0",
            true,
            false,
            DETERMINISTIC_TIMESTAMP,
        )
        .unwrap_err();
        assert_eq!(err.code(), "standard-invalid");
    }

    #[test]
    fn staged_files_are_owned_and_receipted() {
        let files = staged_files(
            "demo",
            "rust-web",
            "baseline-service@1.1.0",
            DETERMINISTIC_TIMESTAMP,
        )
        .unwrap();
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&PROFILE_PATH));
        assert!(paths.contains(&VERIFY_PATH));
        assert!(paths.contains(&CI_PATH));
        assert!(paths.contains(&QUALITY_PATH));
        assert!(paths.contains(&COMPOSE_PATH));
        assert!(paths.contains(&RECEIPT_PATH));
        // Every owned path lives under the snapshot directory.
        assert!(
            paths.iter().all(|p| p.starts_with(STANDARD_DIR)),
            "{paths:?}"
        );
        // A refused selection stages nothing, not a guessed fallback.
        let err = staged_files("demo", "react-web", "nope@1.0.0", DETERMINISTIC_TIMESTAMP)
            .expect_err("unknown pack refuses before staging");
        assert_eq!(err.code(), "standard-invalid");
    }
}
