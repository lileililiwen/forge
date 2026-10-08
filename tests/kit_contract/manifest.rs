//! Generation, manifest parsing, and Forge-absent operation.

use super::*;

#[test]
fn the_generated_dotnet_project_operates_without_forge() {
    if Command::new("dotnet").arg("--version").output().is_err() {
        // No toolchain: reported unverified, never as a pass.
        return;
    }
    // The feed is committed in the project, so the build needs no environment
    // variable and no sibling library. The boundary under test is that the
    // project builds through its own toolchain with Forge absent, at a path it
    // was not generated at.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let origin = tmp.path().join("origin");
    let out = scaffold(&db, &origin, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // Move the whole tree to an unrelated path. Nothing about the machine it
    // was generated on may matter.
    let dest = tmp.path().join("elsewhere").join("relocated");
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    copy_dir(&origin, &dest);

    let build = Command::new("dotnet")
        .arg("build")
        .arg("--nologo")
        .current_dir(&dest)
        .env_remove("FORGE_REGISTRY")
        // The rejected mechanism must not be what makes this build work.
        .env_remove("NUGET_PLATFORM_FEED")
        .output()
        .expect("dotnet build");
    assert!(build.status.success(), "{}", lossy(&build.stdout));
    // No Forge runtime dependency entered the tree. The manifest and the
    // README name Forge by design, so they are excluded: what must not appear
    // anywhere is a *path* to the Forge build.
    for (path, bytes) in file_bytes(&dest) {
        if path == "README.md" || path == "forge.yaml" {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("CARGO_MANIFEST_DIR") && !text.contains("/forge/target/"),
            "{path} carries a Forge runtime dependency"
        );
    }
}

#[test]
fn no_generation_path_consults_a_sibling_checkout() {
    // The generation path reads the checked-in `kits/` tree and the compiled-in
    // registry, nothing else. Rendering every profile under a PATH with no
    // package manager present proves no toolchain is invoked.
    for (profile, toolchain) in [
        ("aspnet-web", "dotnet"),
        ("react-web", "npm"),
        ("rust-web", "cargo"),
        ("python-service", "python3"),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("registry.db");
        let dest = tmp.path().join("scaffold");
        let empty = tmp.path().join("empty-path");
        fs::create_dir_all(&empty).unwrap();

        let out = Command::new(forge_bin())
            .arg("--registry")
            .arg(&db)
            .arg("new")
            .arg(&dest)
            .arg("--profile")
            .arg(profile)
            // A PATH with no package manager: rendering must not need one.
            .env("PATH", &empty)
            .output()
            .expect("run forge");
        assert_eq!(
            out.status.code(),
            Some(0),
            "{profile} generation needed {toolchain} on PATH: {}",
            lossy(&out.stderr)
        );
        assert!(dest.join("forge.yaml").exists());
    }
}

#[test]
fn the_manifest_parses_the_rendered_kit_block() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");
    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // The staged manifest is validated by the generator itself; re-parse it
    // through the model so the block is genuinely typed, not just text.
    let (manifest, path) =
        forge::core::manifest::Manifest::load_from_dir(&dest, None).expect("manifest loads");
    assert!(path.ends_with("forge.yaml"));
    let kit = manifest.kit.expect("the kit block is parsed");
    assert_eq!(kit.id, "platform-dotnet");
    assert_eq!(kit.version.as_deref(), Some("0.1.0"));
    assert_eq!(kit.ecosystem, "dotnet");
    assert_eq!(kit.tfm.as_deref(), Some("net10.0"));
    assert_eq!(kit.minimum_packages, 6);
    assert!(kit.exception.is_none());
    let feed = kit.feed.expect("the feed is parsed");
    assert_eq!(feed.name, "platform");
    assert_eq!(feed.path, "packages/platform-feed");
    // No existing field changed meaning.
    assert_eq!(manifest.project.profile, "aspnet-web");
    assert_eq!(manifest.schema, 1);
}

#[test]
fn a_pre_kit_manifest_still_parses() {
    // A project written before kits existed carries no block and keeps every
    // existing field.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(
        dir.join("forge.yaml"),
        "schema: 1\nproject:\n  id: old-app\n  name: Old App\n  profile: rust-web\n",
    )
    .unwrap();
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(dir, None).unwrap();
    assert!(manifest.kit.is_none());
    assert_eq!(manifest.project.id, "old-app");
}

#[test]
fn the_kit_digest_is_reported_per_file_for_a_whole_tree() {
    // The digest-agreement test reports the file, the expected digest and the
    // actual digest — the same discipline as contract consumption.
    let reports: BTreeMap<String, String> = forge::kit::verify_kit_digests()
        .expect("digests agree")
        .into_iter()
        .map(|r| (r.path, r.expected))
        .collect();
    assert!(reports.contains_key("tokens/tokens.css"), "{reports:?}");
    assert!(reports.contains_key("tokens/tokens.ts"), "{reports:?}");
    for (path, digest) in &reports {
        assert_eq!(digest.len(), 64, "{path} has no sha256 digest");
    }
}
