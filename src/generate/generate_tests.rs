//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::core::validate_project_id;
    use crate::generate::engine::{
        check_files_inside, generate, render_files, toolchain_present, verify_native,
    };
    use crate::generate::model::CreationRequest;
    use crate::generate::request::{normalize_explicit, parse_interactive};
    use crate::profile::inspect_profile;
    use crate::registry::Registry;
    use std::collections::HashSet;
    use std::fs;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn dest(tmp: &TempDir, name: &str) -> PathBuf {
        tmp.path().join(name)
    }

    fn request_for(profile: &str, dest: &Path) -> CreationRequest {
        let id = format!("{profile}-demo");
        normalize_explicit(Some(profile), Some(id.as_str()), None, &[], dest, None).unwrap()
    }

    fn open_registry(dir: &TempDir) -> Registry {
        Registry::open(&dir.path().join("registry.db")).unwrap()
    }

    /// Whether the text names a date in the shape `YYYY-MM-DD`.
    ///
    /// The generated changelog carries no clock: the scaffold's assembly is
    /// deterministic, so the same request renders the same bytes.
    fn names_a_date(text: &str) -> bool {
        let groups: Vec<&str> = text
            .split(|c: char| !c.is_ascii_digit())
            .filter(|group| !group.is_empty())
            .collect();
        groups
            .windows(3)
            .any(|w| w[0].len() == 4 && w[1].len() == 2 && w[2].len() == 2)
    }

    #[test]
    fn workspace_metadata_staged_by_default_and_omitted_on_opt_out() {
        let tmp = TempDir::new().unwrap();
        for profile in [
            "aspnet-web",
            "flutter-app",
            "nextjs-web",
            "python-service",
            "react-web",
            "rust-web",
        ] {
            let id = format!("meta-{profile}");
            let req = normalize_explicit(
                Some(profile),
                Some(&id),
                None,
                &[],
                &dest(&tmp, &format!("meta-{profile}")),
                None,
            )
            .unwrap();
            let files = render_files(&req).unwrap();
            let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
            assert!(
                paths.contains(&crate::generate::workspace::METADATA_PATH),
                "{profile}: {paths:?}"
            );
            assert!(
                paths.contains(&crate::generate::workspace::RECEIPT_PATH),
                "{profile}: {paths:?}"
            );
            let mut opted_out = req.clone();
            opted_out.workspace_metadata = false;
            let legacy = render_files(&opted_out).unwrap();
            let expected: Vec<(String, String)> = files
                .iter()
                .filter(|(p, _)| {
                    p != crate::generate::workspace::METADATA_PATH
                        && p != crate::generate::workspace::RECEIPT_PATH
                })
                .cloned()
                .collect();
            assert_eq!(
                legacy, expected,
                "{profile}: opt-out must equal the prior output"
            );
            // Receipt records exactly the staged declaration bytes.
            let declaration = files
                .iter()
                .find(|(p, _)| p == crate::generate::workspace::METADATA_PATH)
                .map(|(_, c)| c.clone())
                .unwrap();
            let receipt = files
                .iter()
                .find(|(p, _)| p == crate::generate::workspace::RECEIPT_PATH)
                .map(|(_, c)| c.clone())
                .unwrap();
            assert!(
                receipt.contains(&crate::generate::workspace::sha256_hex(
                    declaration.as_bytes()
                )),
                "{profile}"
            );
        }
    }

    #[test]
    fn explicit_and_interactive_normalize_equivalently() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "equiv-app");
        let explicit = normalize_explicit(
            Some("rust-web"),
            Some("equiv-app"),
            Some("Equiv App"),
            &["auth".to_string()],
            &target,
            None,
        )
        .unwrap();
        let input = b"rust-web\nequiv-app\nEquiv App\nauth\n";
        let mut reader = Cursor::new(input);
        let mut writer: Vec<u8> = Vec::new();
        let interactive = parse_interactive(
            &mut reader,
            &mut writer,
            &target,
            None,
            None,
            None,
            &[],
            None,
        )
        .unwrap();
        assert_eq!(explicit, interactive);
        let a = render_files(&explicit).unwrap();
        let b = render_files(&interactive).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn repeatable_bytes_across_directories() {
        let tmp = TempDir::new().unwrap();
        let a = normalize_explicit(
            Some("rust-web"),
            Some("same-id"),
            Some("Same"),
            &[],
            &dest(&tmp, "dir-a"),
            None,
        )
        .unwrap();
        let b = normalize_explicit(
            Some("rust-web"),
            Some("same-id"),
            Some("Same"),
            &[],
            &dest(&tmp, "dir-b"),
            None,
        )
        .unwrap();
        assert_eq!(render_files(&a).unwrap(), render_files(&b).unwrap());
        // The changelog is part of that byte-equality, and it is the file the
        // release lane reads by default.
        let changelog_of = |files: Vec<(String, String)>| {
            files
                .iter()
                .find(|(path, _)| path == "CHANGELOG.md")
                .map(|(_, content)| content.clone())
                .expect("the scaffold writes a changelog")
        };
        assert_eq!(
            changelog_of(render_files(&a).unwrap()),
            changelog_of(render_files(&b).unwrap()),
            "the same request must render byte-identical changelogs"
        );
    }

    #[test]
    fn all_six_profiles_render_portable_manifests() {
        let tmp = TempDir::new().unwrap();
        for profile in [
            "aspnet-web",
            "rust-web",
            "nextjs-web",
            "react-web",
            "flutter-app",
            "python-service",
        ] {
            let id = format!(
                "{}-t1",
                profile
                    .replace("-web", "")
                    .replace("-app", "")
                    .replace("-service", "")
            );
            let req = normalize_explicit(
                Some(profile),
                Some(&id),
                Some(&id),
                &[],
                &dest(&tmp, &format!("scaffold-{profile}")),
                None,
            )
            .unwrap();
            let files = render_files(&req).unwrap();
            assert!(files.iter().any(|(p, _)| p == "forge.yaml"), "{profile}");
            assert!(files.iter().any(|(p, _)| p == "README.md"), "{profile}");
            assert!(files.iter().any(|(p, _)| p == "CHANGELOG.md"), "{profile}");
            let manifest_text = files
                .iter()
                .find(|(p, _)| p == "forge.yaml")
                .map(|(_, c)| c.clone())
                .unwrap();
            let manifest = crate::core::manifest::Manifest::parse(
                Path::new("forge.yaml"),
                manifest_text.as_bytes(),
            )
            .expect("generated manifest must validate");
            assert_eq!(manifest.project.profile, profile);
            crate::profile::resolve_profile(&manifest.project.profile, &[]).unwrap();
            for (rel, contents) in &files {
                assert!(!rel.contains(".."), "{profile}:{rel}");
                if rel == "forge.yaml" || rel == ".gitignore" {
                    continue;
                }
                assert!(
                    !contents.contains("use forge::")
                        && !contents.contains("extern crate forge")
                        && !contents.contains("FORGE_REGISTRY"),
                    "{profile}:{rel} must not depend on a Forge runtime"
                );
            }
            let readme = files
                .iter()
                .find(|(p, _)| p == "README.md")
                .map(|(_, c)| c.clone())
                .unwrap();
            let descriptor = inspect_profile(profile).unwrap();
            assert!(readme.contains(&descriptor.build_command), "{profile}");
            assert!(readme.contains(&descriptor.test_command), "{profile}");
        }
    }

    #[test]
    fn generated_manifests_carry_a_complete_release_and_deployment_block() {
        let tmp = TempDir::new().unwrap();
        for profile in [
            "aspnet-web",
            "rust-web",
            "nextjs-web",
            "react-web",
            "flutter-app",
            "python-service",
        ] {
            let req = request_for(profile, &dest(&tmp, &format!("blocks-{profile}")));
            let files = render_files(&req).unwrap();
            let manifest_text = files
                .iter()
                .find(|(p, _)| p == "forge.yaml")
                .map(|(_, c)| c.clone())
                .unwrap();
            let manifest = crate::core::manifest::Manifest::parse(
                Path::new("forge.yaml"),
                manifest_text.as_bytes(),
            )
            .expect("generated manifest must validate");

            let release = crate::release::prepare::release_config_from_manifest(&manifest)
                .expect("the release configuration must resolve");
            assert_eq!(release.versioning, "semver", "{profile}");
            assert_eq!(
                release
                    .checks
                    .iter()
                    .map(|check| check.kind.as_str())
                    .collect::<Vec<_>>(),
                vec!["doctor", "test"],
                "{profile}"
            );

            let deploy = crate::deploy::prepare::deploy_config_from_manifest(&manifest)
                .expect("the deployment configuration must resolve");
            assert_eq!(deploy.default_target, "local", "{profile}");
            // The artifact is derived from the files the tree contains, so it
            // is declared exactly when the tree ships a container file, and
            // it is always a file the generated tree contains.
            let tree_has_a_container_file = files.iter().any(|(path, _)| {
                path == crate::core::manifest_blocks::DOCKERFILE
                    || crate::core::manifest_blocks::COMPOSE_FILES.contains(&path.as_str())
            });
            assert_eq!(
                deploy.artifact.is_some(),
                tree_has_a_container_file,
                "{profile}"
            );
            if let Some(artifact) = deploy.artifact.as_deref() {
                assert!(
                    files.iter().any(|(path, _)| path == artifact),
                    "{profile}: the declared artifact `{artifact}` is not in the tree"
                );
            }
        }
    }

    #[test]
    fn every_profile_carries_a_changelog_that_claims_no_release() {
        let tmp = TempDir::new().unwrap();
        for profile in [
            "aspnet-web",
            "rust-web",
            "nextjs-web",
            "react-web",
            "flutter-app",
            "python-service",
        ] {
            let id = format!("changelog-{profile}");
            let req = normalize_explicit(
                Some(profile),
                Some(&id),
                Some(&id),
                &[],
                &dest(&tmp, &id),
                None,
            )
            .unwrap();
            let files = render_files(&req).unwrap();
            let changelog = files
                .iter()
                .find(|(p, _)| p == "CHANGELOG.md")
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| panic!("{profile}: the scaffold must write a changelog"));
            // One `## [Unreleased]` section and no other version heading:
            // nothing has been released.
            let headings: Vec<&str> = changelog
                .lines()
                .filter(|line| line.starts_with("## ["))
                .collect();
            assert_eq!(headings, vec!["## [Unreleased]"], "{profile}: {changelog}");
            assert!(
                !headings.iter().any(|line| line
                    .trim_start_matches("## [")
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit())),
                "{profile}: {changelog}"
            );
            // No date: the assembly is deterministic and the clock is not
            // part of the file.
            assert!(!names_a_date(&changelog), "{profile}: {changelog}");
            // The file names the project, the scaffold and the profile.
            assert!(changelog.contains(&id), "{profile}: {changelog}");
            assert!(
                changelog.contains(&format!("All notable changes to {id} are documented here.")),
                "{profile}: {changelog}"
            );
            assert!(
                changelog.contains(&format!(
                    "Initial Forge scaffold for the `{profile}` profile."
                )),
                "{profile}: {changelog}"
            );
        }
    }

    #[test]
    fn the_scaffolded_changelog_satisfies_the_release_lane_default_path() {
        let tmp = TempDir::new().unwrap();
        let req = request_for("rust-web", &dest(&tmp, "release-path"));
        let files = render_files(&req).unwrap();
        // The release lane reads this path when the manifest names none.
        assert_eq!(
            crate::release::contract::DEFAULT_CHANGELOG,
            "CHANGELOG.md",
            "the release lane's default changelog path"
        );
        assert!(
            files
                .iter()
                .any(|(path, _)| path == crate::release::contract::DEFAULT_CHANGELOG),
            "the scaffold must write the changelog the release lane reads"
        );
        // And the release lane really loads that file.
        let written = TempDir::new().unwrap();
        for (rel, contents) in &files {
            let path = written.path().join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
        let captured = crate::release::prepare::load_changelog(
            written.path(),
            crate::release::contract::DEFAULT_CHANGELOG,
        )
        .expect("the release lane finds the scaffolded changelog");
        assert_eq!(captured.path, "CHANGELOG.md");
        assert!(
            captured.excerpt.contains("## [Unreleased]"),
            "{}",
            captured.excerpt
        );
    }

    #[test]
    fn request_for_helper_builds_valid_ids() {
        let tmp = TempDir::new().unwrap();
        for profile in ["rust-web", "flutter-app", "python-service"] {
            let req = request_for(profile, &dest(&tmp, profile));
            validate_project_id(&req.id).unwrap();
        }
    }

    #[test]
    fn nonempty_destination_fails_before_overwrite() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "taken");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep.txt"), "do not touch").unwrap();
        let req =
            normalize_explicit(Some("rust-web"), Some("taken"), None, &[], &target, None).unwrap();
        let mut reg = open_registry(&tmp);
        let err = generate(&mut reg, &req).expect_err("nonempty must fail");
        assert_eq!(err.code(), "generation-conflict");
        assert_eq!(
            fs::read_to_string(target.join("keep.txt")).unwrap(),
            "do not touch"
        );
        assert!(!target.join("forge.yaml").exists());
        assert!(reg.inspect("taken").is_err());
    }

    #[test]
    fn template_escape_is_rejected_without_writes() {
        let evil = vec![("../evil.txt".to_string(), "x".to_string())];
        let err = check_files_inside(&evil).expect_err("escape must fail");
        assert_eq!(err.code(), "generation-conflict");
    }

    #[test]
    fn cancelled_interactive_leaves_nothing_behind() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "cancelled-app");
        let mut reader = Cursor::new(b"");
        let mut writer: Vec<u8> = Vec::new();
        let err = parse_interactive(
            &mut reader,
            &mut writer,
            &target,
            None,
            None,
            None,
            &[],
            None,
        )
        .expect_err("EOF must cancel");
        assert_eq!(err.code(), "generation-cancelled");
        assert!(!target.exists());
        let reg = open_registry(&tmp);
        assert!(reg.list().unwrap().is_empty());
    }

    #[test]
    fn unknown_profile_and_incompatible_features_fail_before_mutation() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "bad");
        let err = normalize_explicit(
            Some("not-a-real-profile"),
            Some("bad"),
            None,
            &[],
            &target,
            None,
        )
        .expect_err("unknown profile");
        assert_eq!(err.code(), "unknown-profile");
        assert!(!target.exists());

        let err = normalize_explicit(
            Some("flutter-app"),
            Some("bad"),
            None,
            &["postgres".to_string()],
            &target,
            None,
        )
        .expect_err("incompatible feature");
        assert_eq!(err.code(), "incompatible-profile");
        assert!(!target.exists());
    }

    #[test]
    fn id_collision_cleans_promoted_output_and_registers_nothing_new() {
        let tmp = TempDir::new().unwrap();
        let first = dest(&tmp, "first");
        let second = dest(&tmp, "second");
        let first_req =
            normalize_explicit(Some("rust-web"), Some("dupe-id"), None, &[], &first, None).unwrap();
        let mut reg = open_registry(&tmp);
        generate(&mut reg, &first_req).unwrap();

        let second_req =
            normalize_explicit(Some("rust-web"), Some("dupe-id"), None, &[], &second, None)
                .unwrap();
        let err = generate(&mut reg, &second_req).expect_err("id reuse must fail");
        assert_eq!(err.code(), "id-collision");
        assert!(!second.join("forge.yaml").exists());
        // Original record unchanged.
        assert_eq!(reg.inspect("dupe-id").unwrap().profile, "rust-web");
        assert_eq!(reg.list().unwrap().len(), 1);
    }

    #[test]
    fn missing_toolchain_reports_unverified_without_claiming_build() {
        let tmp = TempDir::new().unwrap();
        let empty = HashSet::new();
        let err = verify_native("rust-web", tmp.path(), Some(&empty)).expect_err("no cargo");
        assert_eq!(err.code(), "toolchain-missing");
        assert!(err.to_string().contains("not tested"));
    }

    #[test]
    fn rust_scaffold_builds_and_tests_with_native_toolchain() {
        // Probe availability only; native commands must run in the generated project below.
        if !toolchain_present("cargo", None) {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "native-app");
        let req = normalize_explicit(
            Some("rust-web"),
            Some("native-app"),
            None,
            &[],
            &target,
            None,
        )
        .unwrap();
        let files = render_files(&req).unwrap();
        fs::create_dir(&target).unwrap();
        for (rel, contents) in &files {
            if rel == "forge.yaml" {
                continue;
            }
            let path = target.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        let report = verify_native("rust-web", &target, None).expect("cargo build+test");
        assert!(report.verified);
    }

    #[test]
    fn react_web_renders_with_index_and_test() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "react-app");
        let req = normalize_explicit(
            Some("react-web"),
            Some("react-app"),
            Some("React App"),
            &["i18n".to_string()],
            &target,
            None,
        )
        .unwrap();
        let files = render_files(&req).unwrap();
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"forge.yaml"), "{paths:?}");
        assert!(paths.contains(&"README.md"), "{paths:?}");
        assert!(paths.contains(&"index.html"), "{paths:?}");
        assert!(paths.contains(&"src/main.jsx"), "{paths:?}");
        assert!(paths.contains(&"src/greeting.mjs"), "{paths:?}");
        assert!(paths.contains(&"src/app.test.mjs"), "{paths:?}");
        assert!(paths.contains(&"vite.config.js"), "{paths:?}");
        assert!(paths.contains(&"package.json"), "{paths:?}");
        // The client is real and previewable: a Vite dev script exists and
        // the dev server binds the Studio-reserved port.
        let package_json = files
            .iter()
            .find(|(p, _)| p == "package.json")
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(package_json.contains("\"dev\": \"vite\""), "{package_json}");
        assert!(
            package_json.contains("\"react\": \"18.3.1\""),
            "{package_json}"
        );
        let vite_config = files
            .iter()
            .find(|(p, _)| p == "vite.config.js")
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(vite_config.contains("FORGE_STUDIO_PORT"), "{vite_config}");
        assert!(vite_config.contains("strictPort: true"), "{vite_config}");
        // The generated manifest advertises react-web so doctor and
        // feature lifecycle contracts both agree.
        let manifest_text = files
            .iter()
            .find(|(p, _)| p == "forge.yaml")
            .map(|(_, c)| c.clone())
            .unwrap();
        let manifest = crate::core::manifest::Manifest::parse(
            Path::new("forge.yaml"),
            manifest_text.as_bytes(),
        )
        .expect("react-web manifest must validate");
        assert_eq!(manifest.project.profile, "react-web");
        // The closed feature set is recorded (i18n only here).
        let features: Vec<&str> = manifest.features.keys().map(String::as_str).collect();
        assert_eq!(features, vec!["i18n"]);
        // The README records the build/test commands from the descriptor.
        let readme = files
            .iter()
            .find(|(p, _)| p == "README.md")
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(readme.contains("npm run build"), "{readme}");
        assert!(readme.contains("npm test"), "{readme}");
    }

    #[test]
    fn react_web_scaffold_builds_and_tests_with_native_toolchain() {
        // Probe only with an isolated target directory; running npm in
        // the project root would block on missing project metadata.
        let toolchain: HashSet<String> = ["npm".to_string()].into_iter().collect();
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "native-react");
        let req = normalize_explicit(
            Some("react-web"),
            Some("native-react"),
            None,
            &[],
            &target,
            None,
        )
        .unwrap();
        let files = render_files(&req).unwrap();
        fs::create_dir(&target).unwrap();
        for (rel, contents) in &files {
            if rel == "forge.yaml" {
                continue;
            }
            let path = target.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
        if verify_native("react-web", &target, Some(&toolchain)).is_err() {
            return;
        }
        if !toolchain_present("npm", None) {
            return;
        }
        let report = verify_native("react-web", &target, None).expect("npm run build+test");
        assert!(report.verified);
    }

    #[test]
    fn planned_profile_generation_refuses_before_writes() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "aspnet-saas-app");
        let err = normalize_explicit(
            Some("aspnet-saas"),
            Some("aspnet-saas-app"),
            None,
            &[],
            &target,
            None,
        )
        .expect_err("planned must refuse");
        assert_eq!(err.code(), "unsupported-profile");
        let text = err.to_string();
        assert!(text.contains("planned"), "{text}");
        assert!(
            text.contains("no files were changed") || text.contains("no files were written"),
            "{text}"
        );
        // No file or registry side effects.
        assert!(!target.exists());
        let reg = open_registry(&tmp);
        assert!(reg.list().unwrap().is_empty());
    }

    #[test]
    fn native_profile_rendering_refuses_before_writes() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "forge-native");
        let request = normalize_explicit(
            Some("rust-product"),
            Some("forge-native"),
            None,
            &[],
            &target,
            None,
        )
        .expect("a known native profile normalizes");
        let err = render_files(&request).expect_err("a native profile has no scaffold");
        assert_eq!(err.code(), "unsupported-profile");
        let text = err.to_string();
        assert!(text.contains("native"), "{text}");
        assert!(!target.exists(), "refusal must write nothing");
    }

    #[test]
    fn compatible_generation_defaults_to_gate_pack_and_explicit_versions_stay_pinned() {
        let tmp = TempDir::new().unwrap();
        let plain = normalize_explicit(
            Some("rust-web"),
            Some("std-plain"),
            None,
            &[],
            &dest(&tmp, "std-plain"),
            None,
        )
        .unwrap();
        let baseline = render_files(&plain).unwrap();
        let baseline_receipt = baseline
            .iter()
            .find(|(p, _)| p == crate::standard::RECEIPT_PATH)
            .map(|(_, c)| c.clone())
            .expect("compatible generation stages the default standard receipt");
        let baseline_receipt: crate::standard::Receipt =
            serde_json::from_str(&baseline_receipt).unwrap();
        assert_eq!(baseline_receipt.version, "1.6.0");
        assert!(baseline
            .iter()
            .any(|(p, _)| p == crate::standard::PRE_COMMIT_PATH));
        let default_owned: Vec<String> = baseline_receipt
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect();
        // Explicit selection still pins an older supported version.
        let with_pack = normalize_explicit(
            Some("rust-web"),
            Some("std-plain"),
            None,
            &[],
            &dest(&tmp, "std-plain"),
            Some("baseline-service@1.1.0"),
        )
        .unwrap();
        let files = render_files(&with_pack).unwrap();
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&crate::standard::PROFILE_PATH), "{paths:?}");
        assert!(paths.contains(&crate::standard::RECEIPT_PATH), "{paths:?}");
        let receipt_text = files
            .iter()
            .find(|(p, _)| p == crate::standard::RECEIPT_PATH)
            .map(|(_, c)| c.clone())
            .unwrap();
        let receipt: crate::standard::Receipt = serde_json::from_str(&receipt_text).unwrap();
        assert_eq!(receipt.pack, "baseline-service");
        assert_eq!(receipt.version, "1.1.0");
        assert_eq!(receipt.project, "std-plain");
        for owned in &receipt.files {
            let content = files
                .iter()
                .find(|(p, _)| p == &owned.path)
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| panic!("{} staged", owned.path));
            assert_eq!(
                crate::standard::sha256_hex(content.as_bytes()),
                owned.digest,
                "{}",
                owned.path
            );
        }
        // Selection changes nothing else: minus each pack's owned paths the
        // two renders are identical.
        let explicit_owned: Vec<String> =
            receipt.files.iter().map(|file| file.path.clone()).collect();
        let rest: Vec<(String, String)> = files
            .iter()
            .filter(|(p, _)| {
                p != crate::standard::RECEIPT_PATH
                    && !default_owned.contains(p)
                    && !explicit_owned.contains(p)
            })
            .cloned()
            .collect();
        let baseline_rest: Vec<(String, String)> = baseline
            .iter()
            .filter(|(p, _)| {
                p != crate::standard::RECEIPT_PATH
                    && !default_owned.contains(p)
                    && !explicit_owned.contains(p)
            })
            .cloned()
            .collect();
        assert_eq!(rest, baseline_rest);
        // A bad selection refuses during normalization, before mutation.
        let err = normalize_explicit(
            Some("rust-web"),
            Some("std-bad"),
            None,
            &[],
            &dest(&tmp, "std-bad"),
            Some("baseline-service@2.0.0"),
        )
        .expect_err("proposed pack must refuse generation");
        assert_eq!(err.code(), "standard-invalid");
        let err = normalize_explicit(
            Some("react-web"),
            Some("std-incompat"),
            None,
            &[],
            &dest(&tmp, "std-incompat"),
            Some("baseline-service@1.1.0"),
        )
        .expect_err("incompatible profile must refuse generation");
        assert_eq!(err.code(), "standard-invalid");
    }
}
