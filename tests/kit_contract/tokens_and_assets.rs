//! Vendored design tokens, ownership receipts, and deterministic rendering.

use super::*;
use forge::kit;

#[test]
fn token_artifacts_are_vendored_and_receipted() {
    for profile in ["react-web", "nextjs-web"] {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("registry.db");
        let dest = tmp.path().join("ui-app");

        let out = scaffold(&db, &dest, profile);
        assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

        // The vendored token source lives under the owned subtree.
        for name in ["tokens.css", "tokens.ts", "verify-tokens.mjs"] {
            assert!(
                dest.join(".platform/tokens").join(name).exists(),
                "{profile} did not vendor {name}"
            );
        }
        // The ownership receipt records one digest per owned file.
        let receipt_path = dest.join(".platform/receipt.json");
        let receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        assert_eq!(receipt["kit"], "platform-ui-web");
        assert_eq!(receipt["profile"], profile);
        let files = receipt["files"].as_array().expect("receipt files");
        assert_eq!(files.len(), 3, "one digest per owned file: {receipt}");
        for entry in files {
            let path = entry["path"].as_str().expect("receipt path");
            let digest = entry["digest"].as_str().expect("receipt digest");
            let bytes = fs::read(dest.join(path)).expect("owned file");
            assert_eq!(
                forge::standard::sha256_hex(&bytes),
                digest,
                "{profile} receipt digest drift for {path}"
            );
        }

        // The manifest records exactly one token source.
        let manifest = parse_yaml(&dest.join("forge.yaml"));
        assert_eq!(manifest["kit"]["id"], "platform-ui-web");
        let assets = manifest["kit"]["assets"].as_sequence().expect("kit assets");
        assert_eq!(assets.len(), 3);

        // No registry dependency was added: the offline build contract holds.
        let package = fs::read_to_string(dest.join("package.json")).unwrap();
        for npm_package in ["@platform/react-ui", "@platform/react-shell"] {
            assert!(
                !package.contains(npm_package),
                "{profile} added a registry dependency for {npm_package}: {package}"
            );
        }
    }
}

#[test]
fn the_vendored_tokens_build_and_test_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("ui-app");

    let out = scaffold(&db, &dest, "react-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // The vendored kit verifier runs with the project's own toolchain and
    // needs no registry dependency.
    let verify = Command::new("node")
        .arg(".platform/tokens/verify-tokens.mjs")
        .current_dir(&dest)
        .output()
        .expect("run kit verifier");
    assert!(verify.status.success(), "{}", lossy(&verify.stdout));

    // The existing offline, dependency-free build/test contract is unchanged.
    for script in ["build", "test"] {
        let built = Command::new("npm")
            .arg("run")
            .arg(script)
            .current_dir(&dest)
            .output()
            .expect("npm");
        assert!(
            built.status.success(),
            "`npm run {script}` failed: {}",
            lossy(&built.stdout)
        );
    }
}

#[test]
fn the_pattern_catalog_defines_no_token_value() {
    // The behaviour source and the token source are separate: the catalog
    // supplies semantics, never a colour, spacing, radius or typography value.
    let catalog =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui_pattern/mod.rs"))
            .expect("ui_pattern source");
    for line in catalog.lines() {
        let trimmed = line.trim();
        // Skip comments and doc text, which may *name* the rule.
        if trimmed.starts_with("//") || trimmed.starts_with("///") {
            continue;
        }
        assert!(
            !trimmed.contains("--color-") && !trimmed.contains("var(--"),
            "the pattern catalog defines a token value: {trimmed}"
        );
        assert!(
            !has_literal_hex_colour(trimmed),
            "the pattern catalog defines a literal colour: {trimmed}"
        );
    }
}

#[test]
fn a_profile_with_no_token_kit_records_the_absence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("mbl-app");

    let out = scaffold(&db, &dest, "flutter-app");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // No Dart tokens are synthesized and no web markup is substituted.
    assert!(!dest.join(".platform").exists());
    assert!(!dest.join("tokens.css").exists());
    let main = fs::read_to_string(dest.join("lib/main.dart")).unwrap();
    assert!(!main.contains("#0f172a"), "{main}");

    // The absence is recorded with its reason in both surfaces.
    let manifest = parse_yaml(&dest.join("forge.yaml"));
    assert!(manifest["kit"]["zero_reason"].is_string());
    let readme = fs::read_to_string(dest.join("README.md")).unwrap();
    assert!(
        readme.contains("Declared minimum consumption: 0"),
        "{readme}"
    );
}

#[test]
fn an_edited_owned_token_file_refuses_the_upgrade() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("ui-app");

    let out = scaffold(&db, &dest, "react-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // An unmodified project plans no change and reports every file unchanged.
    let plan = kit::diff_kit_snapshot(&dest, Some("platform-ui-web@0.2.0"))
        .expect("diff is readable")
        .expect("the kit upgrade path exists");
    assert!(plan.conflicts.is_empty(), "{plan:?}");
    assert!(
        plan.entries
            .iter()
            .all(|e| e.change == kit::KitDiffChange::Unchanged),
        "{:?}",
        plan.entries
    );

    // Edit an owned file, then plan again: it is a reviewable conflict.
    let owned = dest.join(".platform/tokens/tokens.css");
    let edited = format!(
        "{}\n/* operator edit */\n",
        fs::read_to_string(&owned).unwrap()
    );
    fs::write(&owned, &edited).unwrap();
    let plan = kit::diff_kit_snapshot(&dest, Some("platform-ui-web@0.2.0"))
        .expect("diff is readable")
        .expect("the kit upgrade path exists");
    assert_eq!(
        plan.conflicts,
        vec![".platform/tokens/tokens.css".to_string()]
    );
    let entry = plan
        .entries
        .iter()
        .find(|e| e.path == ".platform/tokens/tokens.css")
        .unwrap();
    assert_eq!(entry.change, kit::KitDiffChange::Modified);
    // The plan is reviewable: it carries both sides of the change.
    assert!(entry.before.contains("operator edit"));
    assert!(!entry.after.contains("operator edit"));

    // The upgrade refuses with the existing ownership-conflict code and
    // leaves the edited file exactly as the operator wrote it.
    let err = kit::upgrade_kit_snapshot(
        &dest,
        "platform-ui-web@0.2.0",
        true,
        false,
        "2026-01-01T00:00:00Z",
    )
    .expect_err("an edited owned file refuses the upgrade");
    assert_eq!(err.code(), "standard-invalid");
    assert!(err.to_string().contains("left untouched"), "{err}");
    assert_eq!(fs::read_to_string(&owned).unwrap(), edited);
}

#[test]
fn repeated_render_is_byte_identical_including_the_kit_manifest() {
    for profile in ["aspnet-web", "react-web", "flutter-app", "rust-web"] {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");

        // Same identity in both runs: the only thing that may differ is
        // nothing, so any byte difference is a determinism regression.
        for (dest, db) in [(&a, tmp.path().join("a.db")), (&b, tmp.path().join("b.db"))] {
            let out = run(
                &db,
                &[
                    "new",
                    &dest.display().to_string(),
                    "--profile",
                    profile,
                    "--id",
                    "shared-id",
                    "--name",
                    "Shared",
                ],
            );
            assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
        }
        assert_eq!(
            file_bytes(&a),
            file_bytes(&b),
            "{profile} render is not byte-identical across runs"
        );
        // The rendered kit manifest and the ownership receipt are part of
        // that byte-equality.
        let manifest = parse_yaml(&a.join("forge.yaml"));
        assert!(
            manifest["kit"].is_mapping(),
            "{profile} rendered no kit block"
        );
        if profile == "react-web" {
            assert!(a.join(".platform/receipt.json").exists());
        }
    }
}

#[test]
fn flag_and_interactive_renders_are_byte_identical() {
    let tmp = tempfile::tempdir().unwrap();
    // Separate registries, matching the pre-existing equivalence test: the
    // two renders must not collide on project identity.
    let db_flags = tmp.path().join("registry-flags.db");
    let db_interactive = tmp.path().join("registry-interactive.db");
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    let out = run(
        &db_flags,
        &[
            "new",
            &a.display().to_string(),
            "--profile",
            "aspnet-web",
            "--id",
            "ui-app",
            "--name",
            "UI App",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let out = run_with_stdin(
        &db_interactive,
        &["new", &b.display().to_string()],
        &["aspnet-web", "ui-app", "UI App", ""],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert_eq!(
        file_bytes(&a),
        file_bytes(&b),
        "flag and interactive renders differ"
    );
}

#[test]
fn vendored_asset_digests_agree_with_the_kit_manifest() {
    let reports = kit::verify_kit_digests().expect("every vendored asset matches its digest");
    assert!(!reports.is_empty(), "the kits tree is empty");
    for report in reports {
        assert_eq!(report.state, kit::AssetState::Match, "{report:?}");
        assert_eq!(report.expected, report.actual);
    }
}
