//! Kit registration, declared-zero and floor-exception tests.

use super::*;
use forge::kit;

#[test]
fn declared_kit_is_rendered_into_the_native_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let manifest = parse_yaml(&dest.join("forge.yaml"));
    assert_eq!(manifest["kit"]["id"], "platform-dotnet");
    assert_eq!(manifest["kit"]["version"], "0.1.0");
    assert_eq!(manifest["kit"]["ecosystem"], "dotnet");
    // The TFM comes from the kit descriptor, not a hard-coded profile default.
    assert_eq!(manifest["kit"]["tfm"], "net10.0");

    // Native build state is never claimed from rendering alone.
    let json = run_json(&db, &["inspect", "net-app"]);
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(value["kit_id"], "platform-dotnet");
    assert_eq!(value["kit_version"], "0.1.0");
}

#[test]
fn kit_outside_the_compiled_in_registry_is_refused() {
    let reference = kit::registry::KitReference {
        id: "platform-does-not-exist".to_string(),
        version: Some("0.1.0".to_string()),
        ecosystem: kit::registry::Ecosystem::Dotnet,
        feed: None,
        tfm: Some("net10.0".to_string()),
        minimum_packages: 1,
        zero_reason: None,
    };
    let err = kit::registry::inspect_kit(&reference.id, reference.version.as_deref())
        .expect_err("an unregistered kit must refuse");
    assert_eq!(err.code(), "kit-unknown");
    // The message names the id and the registry that was consulted.
    let text = err.to_string();
    assert!(text.contains("platform-does-not-exist"), "{text}");
    assert!(text.contains("compiled-in kit registry"), "{text}");

    // A known kit at a version this build does not carry is equally refused.
    let err = kit::registry::inspect_kit("platform-dotnet", Some("9.9.9"))
        .expect_err("an unknown kit version must refuse");
    assert_eq!(err.code(), "kit-unknown");
    assert!(err.to_string().contains("9.9.9"), "{err}");
}

#[test]
fn kit_declared_for_another_ecosystem_is_refused() {
    // A .NET kit on a Node profile.
    let reference = kit::registry::KitReference {
        id: "platform-dotnet".to_string(),
        version: Some("0.1.0".to_string()),
        ecosystem: kit::registry::Ecosystem::Dotnet,
        feed: None,
        tfm: Some("net10.0".to_string()),
        minimum_packages: 1,
        zero_reason: None,
    };
    let err = kit::registry::kit_for_profile("react-web", &reference, "npm")
        .expect_err("a .NET kit on a Node profile must refuse");
    assert_eq!(err.code(), "kit-ecosystem-mismatch");
    // The message names both ecosystems.
    let text = err.to_string();
    assert!(text.contains("dotnet"), "{text}");
    assert!(text.contains("npm"), "{text}");
}

#[test]
fn profile_with_no_registered_kit_records_a_zero_and_warns() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("mbl-app");

    let out = scaffold(&db, &dest, "flutter-app");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let manifest = parse_yaml(&dest.join("forge.yaml"));
    assert_eq!(manifest["kit"]["id"], "none");
    // A declared zero is 0, and the reason names the missing evidence.
    assert_eq!(manifest["kit"]["minimum_packages"], 0);
    let reason = manifest["kit"]["zero_reason"]
        .as_str()
        .expect("zero_reason");
    assert!(reason.contains("pub"), "{reason}");

    // The console output distinguishes the zero from a floor failure and from
    // a met floor.
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("WARN"), "{stdout}");
    assert!(stdout.contains("zero"), "{stdout}");
    assert!(!stdout.contains("floor met"), "{stdout}");

    // The pattern layer stays available through its existing adapter and no
    // Dart tokens are synthesized.
    assert!(dest.join("pubspec.yaml").exists());
    assert!(!dest.join(".platform/tokens").exists());
}

#[test]
fn generation_resolves_no_feed_and_reports_native_state_unverified() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    // Rendering alone never yields a native pass: the note says the native
    // commands still *require* the toolchain, and never claims one ran.
    assert!(stdout.contains("rendering verified"), "{stdout}");
    assert!(stdout.contains("native build/test require"), "{stdout}");
    assert!(!stdout.contains("succeeded"), "{stdout}");

    // No `obj/`, no `bin/`, no `project.assets.json`: Forge performed no
    // restore of any kind.
    assert!(!dest.join("obj").exists(), "no restore was performed");
    assert!(!dest.join("bin").exists(), "no build was performed");
}

#[test]
fn profile_meeting_its_floor_renders_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // A met floor is not a WARN.
    assert!(
        !lossy(&out.stdout).contains("WARN"),
        "{}",
        lossy(&out.stdout)
    );
    let readme = fs::read_to_string(dest.join("README.md")).unwrap();
    assert!(readme.contains("Floor met"), "{readme}");
}

#[test]
fn profile_below_its_floor_refuses_with_a_typed_error() {
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    // Raise the declared floor above the confirmed set.
    let mut raised = descriptor.clone();
    raised.reference.minimum_packages = descriptor.confirmed_names().len() + 1;

    let err = kit::floor::check_floor("aspnet-web", &raised, None, "2026-01-01T00:00:00Z")
        .expect_err("an unmet floor must refuse");
    assert_eq!(err.code(), "kit-floor-not-met");
    // The message names the profile, the declared minimum, the confirmed set
    // it would have rendered and the provisional set.
    let text = err.to_string();
    assert!(text.contains("aspnet-web"), "{text}");
    assert!(text.contains("minimum shared-layer consumption"), "{text}");
    assert!(text.contains("Platform.Core"), "{text}");
    assert!(text.contains("Platform.Web.Composition"), "{text}");
    // There is no warn-and-continue downgrade.
    assert!(!text.contains("WARN"), "{text}");
}

#[test]
fn provisional_packages_never_satisfy_the_floor() {
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let confirmed = kit::floor::confirmed_count(&descriptor);
    assert_eq!(
        confirmed,
        descriptor.confirmed_names().len(),
        "only confirmed packages count"
    );
    // A confirmed set short of the minimum still refuses.
    let mut short = descriptor.clone();
    short.reference.minimum_packages = confirmed + 1;
    let err = kit::floor::check_floor("aspnet-web", &short, None, "2026-01-01T00:00:00Z")
        .expect_err("an unmet floor must refuse");
    assert_eq!(err.code(), "kit-floor-not-met");
}

#[test]
fn a_refusal_leaves_no_partial_scaffold() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");
    fs::create_dir_all(&dest).expect("destination exists");
    fs::write(dest.join("keep.txt"), "operator file").expect("seed");

    let out = scaffold(&db, &dest, "aspnet-web");
    // The pre-existing nonempty destination refuses before staging, and the
    // operator's file is untouched.
    assert!(!out.status.success(), "{}", lossy(&out.stdout));
    assert_eq!(
        fs::read_to_string(dest.join("keep.txt")).unwrap(),
        "operator file"
    );
    let staged: Vec<_> = collect_files(&dest)
        .into_iter()
        .map(|p| p.strip_prefix(&dest).unwrap().to_path_buf())
        .collect();
    assert_eq!(staged, vec![PathBuf::from("keep.txt")]);

    // The registry is row-identical: nothing was registered.
    let list = run_json(&db, &["list"]);
    let value: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(value["projects"].as_array().map(Vec::len), Some(0));
}

#[test]
fn a_declared_zero_is_not_reported_as_a_met_floor() {
    let reference = kit::registry::declared_zero(
        kit::registry::Ecosystem::Pub,
        "no registered token kit for pub",
    );
    let descriptor = kit::registry::kit_for_profile("flutter-app", &reference, "flutter").unwrap();
    let decision =
        kit::floor::check_floor("flutter-app", &descriptor, None, "2026-01-01T00:00:00Z").unwrap();
    assert!(matches!(decision, kit::FloorDecision::DeclaredZero { .. }));
    assert!(
        !decision.floor_met(),
        "a declared zero is never a met floor"
    );
    let warning = decision.warning("flutter-app").expect("a zero warns");
    assert!(warning.starts_with("WARN"), "{warning}");
}

#[test]
fn an_exception_records_its_reason_in_the_manifest_and_readme() {
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let mut raised = descriptor.clone();
    raised.reference.minimum_packages = descriptor.confirmed_names().len() + 1;

    let decision = kit::floor::check_floor(
        "aspnet-web",
        &raised,
        Some("deliberate greenfield spike"),
        "2026-01-01T00:00:00Z",
    )
    .expect("a reasoned exception records");
    let kit::FloorDecision::Exception { exception, .. } = &decision else {
        panic!("expected a recorded exception");
    };
    assert_eq!(exception.reason, "deliberate greenfield spike");
    assert_eq!(exception.floor, raised.reference.minimum_packages);
    assert_eq!(exception.declared, descriptor.confirmed_names().len());
    assert_eq!(exception.recorded_at, "2026-01-01T00:00:00Z");
    assert!(decision.warning("aspnet-web").unwrap().starts_with("WARN"));
}

#[test]
fn an_exception_without_a_reason_refuses() {
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let mut raised = descriptor.clone();
    raised.reference.minimum_packages = descriptor.confirmed_names().len() + 1;

    for empty in ["", "   ", "\t"] {
        let err =
            kit::floor::check_floor("aspnet-web", &raised, Some(empty), "2026-01-01T00:00:00Z")
                .expect_err("an empty reason must refuse");
        assert_eq!(err.code(), "kit-exception-reason-required");
    }
}

#[test]
fn an_exception_is_never_recorded_for_a_met_floor() {
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let decision = kit::floor::check_floor(
        "aspnet-web",
        &descriptor,
        Some("unnecessary"),
        "2026-01-01T00:00:00Z",
    )
    .expect("a met floor renders");
    assert!(matches!(decision, kit::FloorDecision::Met { .. }));
    assert!(decision.warning("aspnet-web").is_none());
}

#[test]
fn the_kit_exception_flag_is_rejected_when_the_floor_is_met() {
    // Every shipped profile meets its floor, so the flag is a no-op rather
    // than a way to manufacture a recorded exception.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = run(
        &db,
        &[
            "new",
            &dest.display().to_string(),
            "--profile",
            "aspnet-web",
            "--kit-exception",
            "unnecessary",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let manifest = parse_yaml(&dest.join("forge.yaml"));
    assert!(
        manifest["kit"].get("exception").is_none(),
        "no exception is recorded for a floor that did not fail: {manifest:?}"
    );
    assert!(
        !lossy(&out.stdout).contains("WARN"),
        "{}",
        lossy(&out.stdout)
    );
}
