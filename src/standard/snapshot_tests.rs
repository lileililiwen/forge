//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use crate::standard::layout::{
        ACCESSIBILITY_CHECK_PATH, CI_PATH, CLIENT_PACK, COMPOSE_PATH, DETERMINISTIC_TIMESTAMP,
        GITHUB_CI_PATH, OPENSPEC_NAME_CHECK_PATH, PRE_COMMIT_PATH, PRIVACY_CHECK_PATH,
        PROFILE_PATH, QUALITY_PATH, RECEIPT_PATH, SHARED_ACCESSIBILITY_CHECK,
        SHARED_OPENSPEC_NAME_CHECK, SHARED_PRIVACY_CHECK, SHARED_VERIFY_SCRIPT, STANDARD_DIR,
        VERIFY_LOCAL_PATH, VERIFY_PATH,
    };
    use crate::standard::model::{
        DiffChange, FileState, PackAssetOrigin, PackDescriptor, PackEvidence, PackSupportState,
        SnapshotState,
    };
    use crate::standard::snapshot::{
        all_packs, check_snapshot, default_for_generation, diff_snapshot, inspect_pack,
        install_snapshot, pack_asset_digest, parse_pack_spec, receipt_text, render_snapshot,
        resolve_asset_origin, select_for_generation, staged_files, upgrade_snapshot,
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
        // Client profiles select their own Gate-only pack. Cross-family
        // explicit selections remain incompatible.
        assert_eq!(
            select_for_generation("react-web", "baseline-client@1.0.0").unwrap(),
            "baseline-client@1.0.0"
        );
        assert_eq!(
            select_for_generation("react-web", "baseline-client@1.1.0").unwrap(),
            "baseline-client@1.1.0"
        );
        let err = select_for_generation("react-web", "baseline-service@1.1.0")
            .expect_err("incompatible profile must refuse");
        assert_eq!(err.code(), "standard-invalid");
        assert_eq!(
            default_for_generation("rust-web").as_deref(),
            Some("baseline-service@1.4.0")
        );
        assert_eq!(
            default_for_generation("react-web").as_deref(),
            Some("baseline-client@1.1.0")
        );
        assert_eq!(
            default_for_generation("flutter-app").as_deref(),
            Some("baseline-client@1.1.0")
        );
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

    #[test]
    fn shared_asset_version_supersets_the_previous_one() {
        let previous = baseline("baseline-service@1.1.0");
        let shared = baseline("baseline-service@1.2.0");
        assert!(shared.is_selectable(), "1.2.0 must be selectable");
        let previous_paths: Vec<&str> = previous.files.iter().map(|a| a.path.as_str()).collect();
        let shared_paths: Vec<&str> = shared.files.iter().map(|a| a.path.as_str()).collect();
        for path in &previous_paths {
            assert!(
                shared_paths.contains(path),
                "missing inherited asset {path}"
            );
        }
        for path in [
            VERIFY_LOCAL_PATH,
            OPENSPEC_NAME_CHECK_PATH,
            ACCESSIBILITY_CHECK_PATH,
            PRIVACY_CHECK_PATH,
        ] {
            assert!(shared_paths.contains(&path), "missing shared asset {path}");
        }
        // The previous version keeps its own bytes: adding assets is additive.
        assert_ne!(previous.asset_digest, shared.asset_digest);
    }

    #[test]
    fn gate_pack_assets_share_one_repository_wide_command() {
        let pack = baseline("baseline-service@1.3.0");
        let rendered =
            render_snapshot("demo", "rust-web", &pack, DETERMINISTIC_TIMESTAMP, false).unwrap();
        let content = |path: &str| -> String {
            rendered
                .files
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| panic!("{path} not rendered"))
        };
        let ci = content(CI_PATH);
        let github_ci = content(GITHUB_CI_PATH);
        let hook = content(PRE_COMMIT_PATH);
        assert!(ci.contains("forge gate ."), "{ci}");
        assert!(ci.contains("sh .standard/scripts/verify.sh"), "{ci}");
        assert!(github_ci.contains("forge gate ."), "{github_ci}");
        assert!(
            github_ci.contains("sh .standard/scripts/verify.sh"),
            "{github_ci}"
        );
        assert!(ci.contains("d7c5f96c16fab8c50a9b40f80e55783b25a729af"));
        assert!(ci.contains("contents: read"), "{ci}");
        assert!(hook.contains("entry: forge gate ."), "{hook}");
        assert!(hook.contains("pass_filenames: false"), "{hook}");
        assert!(hook.contains("always_run: true"), "{hook}");
    }

    #[test]
    fn client_pack_is_gate_only_and_covers_both_client_profiles() {
        let pack = baseline("baseline-client@1.0.0");
        assert!(pack.supports("react-web"));
        assert!(pack.supports("flutter-app"));
        let rendered =
            render_snapshot("demo", "react-web", &pack, DETERMINISTIC_TIMESTAMP, false).unwrap();
        let content = |path: &str| -> String {
            rendered
                .files
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| panic!("{path} not rendered"))
        };
        let verifier = content(VERIFY_PATH);
        let ci = content(GITHUB_CI_PATH);
        let hook = content(PRE_COMMIT_PATH);
        assert!(verifier.contains("exec forge gate ."), "{verifier}");
        assert!(ci.contains("sh .standard/scripts/verify.sh"), "{ci}");
        assert!(!ci.contains("verify-local.sh"), "{ci}");
        assert!(hook.contains("entry: forge gate ."), "{hook}");
        assert!(hook.contains("pass_filenames: false"), "{hook}");
        assert_eq!(pack.id, CLIENT_PACK);
    }

    #[test]
    fn rendered_shared_assets_are_the_pinned_canonical_bytes() {
        let pack = baseline("baseline-service@1.2.0");
        let rendered =
            render_snapshot("demo", "rust-web", &pack, DETERMINISTIC_TIMESTAMP, false).unwrap();
        let content = |path: &str| -> String {
            rendered
                .files
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| panic!("{path} not rendered"))
        };
        // Byte equality is the whole point: a project that installs the pack
        // runs the same logic the hand-copied files ran.
        assert_eq!(content(VERIFY_LOCAL_PATH), SHARED_VERIFY_SCRIPT);
        assert_eq!(
            content(OPENSPEC_NAME_CHECK_PATH),
            SHARED_OPENSPEC_NAME_CHECK
        );
        assert_eq!(
            content(ACCESSIBILITY_CHECK_PATH),
            SHARED_ACCESSIBILITY_CHECK
        );
        assert_eq!(content(PRIVACY_CHECK_PATH), SHARED_PRIVACY_CHECK);
        // …and that logic is the real rehearsal script, not a stub.
        let rehearsal = content(VERIFY_LOCAL_PATH);
        for expected in ["cargo clippy", "dotnet build", "pytest", "npm test"] {
            assert!(rehearsal.contains(expected), "missing {expected}");
        }
        // The snapshot entry point delegates to the root script from the project
        // root: installed anywhere else the script would check the wrong tree.
        let wrapper = content(VERIFY_PATH);
        assert!(wrapper.contains("exec sh ./verify-local.sh"), "{wrapper}");
        assert!(wrapper.contains("/../.."), "{wrapper}");
    }

    #[test]
    fn install_refuses_unconfirmed_existing_snapshot_and_conflicts() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = install_snapshot(
            tmp.path(),
            "baseline-service@1.2.0",
            Some("rust-web"),
            Some("demo"),
            false,
            DETERMINISTIC_TIMESTAMP,
        )
        .expect_err("confirmation is required");
        assert_eq!(err.code(), "standard-invalid");
        assert!(!tmp.path().join(RECEIPT_PATH).exists());

        // A divergent owned path refuses the whole install and names it.
        write(tmp.path(), PRIVACY_CHECK_PATH, b"# local privacy checker\n");
        let err = install_snapshot(
            tmp.path(),
            "baseline-service@1.2.0",
            Some("rust-web"),
            Some("demo"),
            true,
            DETERMINISTIC_TIMESTAMP,
        )
        .expect_err("a divergent owned path must refuse");
        assert_eq!(err.code(), "standard-invalid");
        assert!(err.to_string().contains(PRIVACY_CHECK_PATH), "{err}");
        assert_eq!(
            fs::read_to_string(tmp.path().join(PRIVACY_CHECK_PATH)).unwrap(),
            "# local privacy checker\n"
        );
        assert!(!tmp.path().join(RECEIPT_PATH).exists());
        assert!(!tmp.path().join(VERIFY_LOCAL_PATH).exists());

        // With the conflict resolved, the install succeeds, then refuses to
        // install over its own snapshot.
        fs::remove_file(tmp.path().join(PRIVACY_CHECK_PATH)).unwrap();
        install_snapshot(
            tmp.path(),
            "baseline-service@1.2.0",
            Some("rust-web"),
            Some("demo"),
            true,
            DETERMINISTIC_TIMESTAMP,
        )
        .unwrap();
        let err = install_snapshot(
            tmp.path(),
            "baseline-service@1.2.0",
            Some("rust-web"),
            Some("demo"),
            true,
            DETERMINISTIC_TIMESTAMP,
        )
        .expect_err("an existing snapshot must refuse");
        assert_eq!(err.code(), "standard-invalid");
        assert!(err.to_string().contains("diff"), "{err}");
    }

    #[test]
    fn install_adopts_identical_files_and_writes_only_owned_paths() {
        let tmp = tempfile::TempDir::new().unwrap();
        // The fleet's real starting state: the canonical script already there.
        write(
            tmp.path(),
            VERIFY_LOCAL_PATH,
            SHARED_VERIFY_SCRIPT.as_bytes(),
        );
        write(tmp.path(), "README.md", b"# untouched\n");
        write(tmp.path(), "src/main.rs", b"fn main() {}\n");
        let before_script = fs::read(tmp.path().join(VERIFY_LOCAL_PATH)).unwrap();
        let report = install_snapshot(
            tmp.path(),
            "baseline-service@1.2.0",
            Some("rust-web"),
            Some("demo"),
            true,
            DETERMINISTIC_TIMESTAMP,
        )
        .unwrap();
        assert_eq!(report.pack, "baseline-service");
        assert_eq!(report.version, "1.2.0");
        assert_eq!(report.project, "demo");
        assert_eq!(report.profile, "rust-web");
        assert!(
            report.adopted.contains(&VERIFY_LOCAL_PATH.to_string()),
            "{:?}",
            report.adopted
        );
        assert!(!report.installed.contains(&VERIFY_LOCAL_PATH.to_string()));
        assert_eq!(
            fs::read(tmp.path().join(VERIFY_LOCAL_PATH)).unwrap(),
            before_script,
            "an adopted file must not be rewritten"
        );
        // Unrelated files are untouched, and the receipt exists.
        assert_eq!(
            fs::read_to_string(tmp.path().join("README.md")).unwrap(),
            "# untouched\n"
        );
        assert!(tmp.path().join(RECEIPT_PATH).exists());
        // The installed snapshot verifies, and diffs clean against itself.
        let checked = check_snapshot(tmp.path()).unwrap();
        assert_eq!(checked.state, SnapshotState::Rendered);
        assert_eq!(checked.files.len(), 9);
        assert!(checked.files.iter().all(|f| f.state == FileState::Present));
        let diff = diff_snapshot(tmp.path(), "baseline-service@1.2.0").unwrap();
        assert!(diff.conflicts.is_empty(), "{:?}", diff.conflicts);
        assert!(diff
            .entries
            .iter()
            .all(|e| e.change == DiffChange::Unchanged));
    }

    #[test]
    fn install_identity_comes_from_the_manifest_and_conflicts_refuse() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = install_snapshot(
            tmp.path(),
            "baseline-service@1.2.0",
            None,
            Some("demo"),
            true,
            DETERMINISTIC_TIMESTAMP,
        )
        .expect_err("a profile is required without a manifest");
        assert_eq!(err.code(), "standard-invalid");
        assert!(err.to_string().contains("--profile"), "{err}");
        assert!(!tmp.path().join(RECEIPT_PATH).exists());

        // A manifest supplies both, and an explicit contradiction refuses.
        write(
            tmp.path(),
            "forge.yaml",
            b"schema: 1\nproject:\n  id: from-manifest\n  name: From Manifest\n  profile: rust-web\n",
        );
        let err = install_snapshot(
            tmp.path(),
            "baseline-service@1.2.0",
            Some("rust-web"),
            Some("other-id"),
            true,
            DETERMINISTIC_TIMESTAMP,
        )
        .expect_err("a conflicting id must refuse");
        assert_eq!(err.code(), "standard-invalid");
        assert!(!tmp.path().join(RECEIPT_PATH).exists());

        let report = install_snapshot(
            tmp.path(),
            "baseline-service@1.2.0",
            Some("rust-web"),
            None,
            true,
            DETERMINISTIC_TIMESTAMP,
        )
        .unwrap();
        assert_eq!(report.project, "from-manifest");
        assert_eq!(report.profile, "rust-web");
    }
}
