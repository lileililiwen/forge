//! Feed handling, source-mode refusal, and CLI feed validation.

use super::*;
use forge::kit;

#[test]
fn a_named_repo_relative_feed_is_rendered_with_pinned_versions() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let props = fs::read_to_string(dest.join("Directory.Packages.props")).unwrap();
    assert!(props.contains("ManagePackageVersionsCentrally"), "{props}");
    // Every confirmed package is pinned centrally at the kit version.
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    for name in descriptor.confirmed_names() {
        assert!(
            props.contains(&format!(
                "<PackageVersion Include=\"{name}\" Version=\"0.1.0\" />"
            )),
            "{name} is not centrally pinned: {props}"
        );
    }

    // The feed is named, declared with a path relative to the generated
    // project, and sits alongside the neutral public source with inherited
    // sources cleared.
    let nuget = fs::read_to_string(dest.join("NuGet.config")).unwrap();
    assert!(nuget.contains("<clear />"), "{nuget}");
    assert!(nuget.contains("nuget.org"), "{nuget}");
    assert!(
        nuget.contains("<add key=\"platform\" value=\"packages/platform-feed\" />"),
        "the feed value must be relative to the generated project: {nuget}"
    );
    // No machine-specific path anywhere in the tree.
    for (path, bytes) in file_bytes(&dest) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("/home/paul"),
            "{path} leaks a machine-specific absolute path"
        );
    }
}

#[test]
fn no_generated_file_resolves_the_feed_through_an_environment_variable() {
    // The rejected mechanism. A variable a CI runner does not carry is the
    // same failure class as a hard-coded absolute path, so neither the
    // variable nor the MSBuild restore-time source override may survive
    // anywhere in a generated tree.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    for (path, bytes) in file_bytes(&dest) {
        let text = String::from_utf8_lossy(&bytes);
        for banned in [
            "NUGET_PLATFORM_FEED",
            "RestoreAdditionalProjectSources",
            "NUGET_PACKAGES",
        ] {
            assert!(
                !text.contains(banned),
                "{path} still resolves the feed through `{banned}`"
            );
        }
    }
    // The generated Dockerfile takes no build argument and sets no environment
    // variable for the feed.
    let dockerfile = fs::read_to_string(dest.join("Dockerfile")).unwrap();
    assert!(!dockerfile.contains("ARG "), "{dockerfile}");
    assert!(!dockerfile.contains("ENV "), "{dockerfile}");
}

#[test]
fn the_committed_feed_bytes_travel_with_the_generated_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let feed_dir = dest.join(kit::PLATFORM_FEED_PATH);
    assert!(
        feed_dir.is_dir(),
        "the feed directory the NuGet.config names must exist: {}",
        feed_dir.display()
    );
    // The whole restore closure is present, not just the direct references:
    // a feed missing a transitive member does not restore.
    assert_eq!(descriptor.feed().len(), 9, "the closure is 9 packages");
    for entry in descriptor.feed() {
        let file = feed_dir.join(&entry.file);
        assert!(
            file.is_file(),
            "{} is not committed at {}",
            entry.file,
            file.display()
        );
        // A `.nupkg` is a ZIP archive, so the bytes must be copied verbatim
        // rather than round-tripped through a `String`.
        let bytes = fs::read(&file).unwrap();
        assert_eq!(
            &bytes[..2],
            b"PK",
            "{} is not a package archive",
            entry.file
        );
    }
    // Nothing else is committed in the feed: a symbol package or a stale
    // version would be a file no check accounts for.
    let committed: Vec<String> = fs::read_dir(&feed_dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        committed.len(),
        9,
        "unexpected feed contents: {committed:?}"
    );
}

#[test]
fn feed_version_drift_against_the_declared_kit_version_fails() {
    // The drift class of a pinned SDK the CI runner does not have: the feed
    // says one version, the project's `forge.yaml` says another.
    let tmp = tempfile::tempdir().unwrap();
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let feed = tmp.path().join("feed");
    fs::create_dir_all(&feed).unwrap();
    // Every package present, but one packed at a version the descriptor does
    // not declare.
    for entry in descriptor.feed() {
        let name = if entry.package == "Platform.Core" {
            "Platform.Core.0.2.0.nupkg"
        } else {
            entry.file.as_str()
        };
        fs::write(feed.join(name), b"PKstub").unwrap();
    }
    let err = kit::verify_committed_feed(
        &descriptor,
        "0.1.0",
        "the compiled-in kit descriptor",
        &feed,
    )
    .expect_err("a version mismatch must fail");
    assert_eq!(err.code(), "kit-feed-version-mismatch");
    let text = err.to_string();
    assert!(text.contains("Platform.Core"), "{text}");
    assert!(text.contains("0.2.0"), "{text}");
    assert!(text.contains("0.1.0"), "{text}");

    // The same feed passes when the version matches.
    for entry in descriptor.feed() {
        fs::write(feed.join(&entry.file), b"PKstub").unwrap();
    }
    fs::remove_file(feed.join("Platform.Core.0.2.0.nupkg")).unwrap();
    let report = kit::verify_committed_feed(
        &descriptor,
        "0.1.0",
        "the compiled-in kit descriptor",
        &feed,
    )
    .expect("a matching feed passes");
    assert_eq!(report.packages.len(), 9);
    assert!(report.packages.iter().all(|p| p.state == "present"));
}

#[test]
fn a_feed_missing_a_needed_package_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let feed = tmp.path().join("feed");
    fs::create_dir_all(&feed).unwrap();
    for entry in descriptor.feed() {
        // The transitive member arrives only because the confirmed set needs
        // it, and a feed without it does not restore.
        if entry.package == "Platform.Eventing" {
            continue;
        }
        fs::write(feed.join(&entry.file), b"PKstub").unwrap();
    }
    let err = kit::verify_committed_feed(
        &descriptor,
        "0.1.0",
        "the compiled-in kit descriptor",
        &feed,
    )
    .expect_err("a missing package must fail");
    assert_eq!(err.code(), "kit-feed-incomplete");
    assert!(err.to_string().contains("Platform.Eventing"), "{err}");
}

#[test]
fn a_feed_carrying_an_undeclared_package_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let feed = tmp.path().join("feed");
    fs::create_dir_all(&feed).unwrap();
    for entry in descriptor.feed() {
        fs::write(feed.join(&entry.file), b"PKstub").unwrap();
    }
    // A package the kit does not declare: the feed has drifted into a
    // different set, which a restore would accept and a reviewer would not
    // see.
    fs::write(
        feed.join("Platform.Persistence.EfCore.0.1.0.nupkg"),
        b"PKstub",
    )
    .unwrap();
    let err = kit::verify_committed_feed(
        &descriptor,
        "0.1.0",
        "the compiled-in kit descriptor",
        &feed,
    )
    .expect_err("an undeclared package must fail");
    assert_eq!(err.code(), "kit-feed-incomplete");
    assert!(
        err.to_string().contains("Platform.Persistence.EfCore"),
        "{err}"
    );
}

#[test]
fn the_committed_feed_verifies_against_the_compiled_in_kit_version() {
    // The anti-drift gate. It runs in `cargo test` precisely so CI catches
    // what a hand-run check would not — which is how the `global.json`-versus-
    // CI defect reached production.
    let descriptor = kit::registry::inspect_kit("platform-dotnet", None).unwrap();
    let version = descriptor.reference.version.clone().unwrap();
    let report = kit::verify_committed_feed(
        &descriptor,
        &version,
        "the compiled-in kit descriptor",
        &kit::kits_dir().join(kit::KITS_FEED_DIR),
    )
    .unwrap_or_else(|err| panic!("the committed feed must match the kit version: {err}"));
    assert_eq!(report.packages.len(), 9);
    // Every confirmed package in the feed is one the floor counts, and every
    // transitive one is below the consumer bar: the feed may hold more than
    // the floor counts, but the floor counts only the confirmed set.
    let confirmed = descriptor.confirmed_names();
    for entry in report.packages.iter().filter(|p| p.role == "confirmed") {
        assert!(confirmed.contains(&entry.package), "{}", entry.package);
    }
}

#[test]
fn forge_kit_verify_checks_a_project_against_its_own_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");
    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // The project's own `kit.version` and `kit.feed.path` are what the check
    // reads, so the check cannot silently pass against a version the project
    // does not actually pin.
    let out = run(&db, &["kit", "verify", dest.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("kit: platform-dotnet@0.1.0"), "{stdout}");
    assert!(stdout.contains("forge.yaml"), "{stdout}");

    // A project whose manifest pins a different version fails, even though
    // the compiled-in descriptor still says 0.1.0.
    let manifest_path = dest.join("forge.yaml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap()
        .replace("  version: \"0.1.0\"", "  version: \"0.2.0\"");
    fs::write(&manifest_path, manifest).unwrap();
    let out = run(&db, &["kit", "verify", dest.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("kit-feed-version-mismatch"), "{stderr}");
    assert!(stderr.contains("0.2.0"), "{stderr}");
}

#[test]
fn forge_kit_pack_refuses_without_a_sibling_checkout() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(
        &db,
        &[
            "kit",
            "pack",
            "--platform-libs",
            tmp.path().join("absent").to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("kit-pack-unavailable"), "{stderr}");
    assert!(stderr.contains("No vendored file was changed"), "{stderr}");
}

#[test]
fn feed_values_that_are_not_reproducible_are_refused() {
    // Machine-specific absolute path.
    assert_eq!(
        kit::feed::validate_feed_value("/home/paul/feed").unwrap_err(),
        kit::FeedRejection::Absolute
    );
    assert_eq!(
        kit::feed::validate_feed_value("~/feed").unwrap_err(),
        kit::FeedRejection::Absolute
    );
    assert_eq!(
        kit::feed::validate_feed_value("C:\\feeds\\platform").unwrap_err(),
        kit::FeedRejection::Absolute
    );
    // Escaping file:// URL.
    assert_eq!(
        kit::feed::validate_feed_value("file:///etc/feed").unwrap_err(),
        kit::FeedRejection::FileUrl
    );
    // A `..` segment: not absolute, but still not relative to the project.
    assert_eq!(
        kit::feed::validate_feed_value("../dotnet-platform-libs/artifacts").unwrap_err(),
        kit::FeedRejection::Escapes
    );
    assert_eq!(
        kit::feed::validate_feed_value("packages/../../elsewhere").unwrap_err(),
        kit::FeedRejection::Escapes
    );
    // Shell metacharacter.
    assert_eq!(
        kit::feed::validate_feed_value("feed; rm -rf /").unwrap_err(),
        kit::FeedRejection::Metacharacter
    );
    assert_eq!(
        kit::feed::validate_feed_value("$(whoami)").unwrap_err(),
        kit::FeedRejection::Metacharacter
    );
    // Secret-shaped value.
    assert_eq!(
        kit::feed::validate_feed_value("https://user:hunter2@feed.example").unwrap_err(),
        kit::FeedRejection::SecretShaped
    );
    assert_eq!(
        kit::feed::validate_feed_value("feed?token=abc").unwrap_err(),
        kit::FeedRejection::Metacharacter
    );
    // Empty.
    assert_eq!(
        kit::feed::validate_feed_value("  ").unwrap_err(),
        kit::FeedRejection::Empty
    );
    // A project-relative named feed renders.
    assert!(kit::feed::validate_feed_value("packages/platform-feed").is_ok());
}

#[test]
fn a_refused_feed_value_is_never_echoed_back() {
    for value in [
        "https://ci:hunter2@feed.example",
        "/home/paul/feed",
        "file:///etc/feed",
        "feed; rm -rf /",
    ] {
        let rejection = kit::feed::validate_feed_value(value).expect_err("must refuse");
        let err = kit::feed::refuse_feed_value(rejection);
        assert_eq!(err.code(), "kit-feed-invalid");
        let text = err.to_string();
        // The shape is named so the operator knows what to fix; the value is
        // never echoed back into a log or a terminal.
        assert!(
            !text.contains(value),
            "the offending value was echoed: {text}"
        );
        assert!(!text.contains("hunter2"), "{text}");
        assert!(text.contains("never echoed back"), "{text}");
    }
}

#[test]
fn a_source_mode_reference_is_named_so_the_operator_can_find_it() {
    // A refused *reference* is named, so the operator can locate it — the
    // spec requires this for the reference case even though the feed *value*
    // case must never be echoed.
    let err = kit::feed::refuse_source_reference(
        r#"<ProjectReference Include="..\dotnet-platform-libs\src\Platform.Core\Platform.Core.csproj" />"#,
    );
    assert_eq!(err.code(), "kit-feed-invalid");
    assert!(err.to_string().contains("Platform.Core.csproj"), "{}", err);
}

#[test]
fn a_source_mode_project_reference_is_refused() {
    for reference in [
        r#"<ProjectReference Include="..\dotnet-platform-libs\src\Platform.Core\Platform.Core.csproj" />"#,
        "platform-core = { path = \"../rust-platform-libs/platform-core\" }",
    ] {
        let err = kit::feed::refuse_source_reference(reference);
        assert_eq!(err.code(), "kit-feed-invalid");
        let text = err.to_string();
        assert!(text.contains("sibling checkout"), "{text}");
        // The generated project cannot stop working when the sibling is gone.
        assert!(text.contains("nothing was staged"), "{text}");
    }
}
