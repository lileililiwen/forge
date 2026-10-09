//! Kit version upgrade flow: explicit confirmation, path reporting,
//! version pin reconciliation, and registry observation.

use super::*;
use forge::kit;

#[test]
fn a_registered_kit_version_does_not_touch_an_existing_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("ui-app");

    let out = scaffold(&db, &dest, "react-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let before = file_bytes(&dest);

    // Inspecting and listing the project are read-only surfaces.
    for args in [
        vec!["inspect", "ui-app"],
        vec!["list"],
        vec!["doctor", &dest.display().to_string()],
    ] {
        let out = run_json(&db, &args);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{args:?}: {}",
            lossy(&out.stderr)
        );
    }
    assert_eq!(
        file_bytes(&dest),
        before,
        "a read-only command rewrote a pinned project"
    );

    // The project still reports the version it pinned.
    let out = run_json(&db, &["inspect", "ui-app"]);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["kit_version"], "0.2.0");
}

#[test]
fn an_explicit_upgrade_applies_on_confirmation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("ui-app");

    let out = scaffold(&db, &dest, "react-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // Without confirmation nothing is written.
    let err = kit::upgrade_kit_snapshot(
        &dest,
        "platform-ui-web@0.2.0",
        false,
        false,
        "2026-01-01T00:00:00Z",
    )
    .expect_err("an unconfirmed upgrade refuses");
    assert!(err.to_string().contains("explicit confirmation"), "{err}");

    // With confirmation the owned files and the receipt are rewritten and the
    // pinned version is recorded.
    let report = kit::upgrade_kit_snapshot(
        &dest,
        "platform-ui-web@0.2.0",
        true,
        false,
        "2026-01-01T00:00:00Z",
    )
    .expect("upgrade is readable")
    .expect("the kit upgrade path exists");
    assert!(report
        .written
        .contains(&".platform/receipt.json".to_string()));
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(dest.join(".platform/receipt.json")).unwrap()).unwrap();
    assert_eq!(receipt["version"], "0.2.0");
}

#[test]
fn an_upgrade_path_that_does_not_exist_is_reported_with_a_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    // A project that pinned no kit at all.
    let dest = tmp.path().join("rs-app");
    let out = scaffold(&db, &dest, "rust-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let unavailable = kit::diff_kit_snapshot(&dest, Some("platform-dotnet@0.1.0"))
        .expect("diff is readable")
        .expect_err("a declared zero has no kit upgrade path");
    assert!(
        unavailable.reason.contains("pinned no shared-layer kit"),
        "{}",
        unavailable.reason
    );

    // A target version this build does not know.
    let ui = tmp.path().join("ui-app");
    let out = scaffold(&db, &ui, "react-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let unavailable = kit::diff_kit_snapshot(&ui, Some("platform-ui-web@9.9.9"))
        .expect("diff is readable")
        .expect_err("an unknown target version has no upgrade path");
    assert!(
        unavailable.reason.contains("unavailable"),
        "{}",
        unavailable.reason
    );
}

#[test]
fn an_upgrade_records_the_version_the_project_pins() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("ui-app");

    let out = scaffold(&db, &dest, "react-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let manifest_path = dest.join("forge.yaml");
    let original = fs::read_to_string(&manifest_path).unwrap();
    assert_eq!(
        parse_yaml(&manifest_path)["kit"]["version"].as_str(),
        Some("0.2.0"),
        "the scaffold pins the version it was generated with"
    );

    // A manifest that disagrees with the project's own committed bytes. This
    // is the defect the upgrade has to reconcile rather than leave behind:
    // `forge kit verify <path>` deliberately reads what the project *says* it
    // pins, so a stale line here is a project that fails its own drift gate.
    let stale = original.replacen("  version: \"0.2.0\"", "  version: \"0.0.9\"", 1);
    assert_ne!(stale, original, "the kit block declares a version line");
    fs::write(&manifest_path, &stale).unwrap();

    let report = kit::upgrade_kit_snapshot(
        &dest,
        "platform-ui-web@0.2.0",
        true,
        false,
        "2026-01-01T00:00:00Z",
    )
    .expect("upgrade is readable")
    .expect("the kit upgrade path exists");
    assert!(
        report.written.contains(&"forge.yaml".to_string()),
        "the upgrade records the pin it moved to: {:?}",
        report.written
    );

    // The declared pin, the receipt and the owned files agree again.
    assert_eq!(
        parse_yaml(&manifest_path)["kit"]["version"].as_str(),
        Some("0.2.0"),
        "the manifest pins the version the upgrade moved to"
    );
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(dest.join(".platform/receipt.json")).unwrap()).unwrap();
    assert_eq!(receipt["version"], "0.2.0");

    // And the edit is exactly the one line it claims to be. A YAML round-trip
    // would have reordered and reformatted the whole manifest, breaking the
    // byte-identical render contract; this proves it does not.
    assert_eq!(
        fs::read_to_string(&manifest_path).unwrap(),
        original,
        "only the declared version line changed"
    );
}

#[test]
fn the_explicit_upgrade_is_operator_reachable_and_reviews_before_it_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("ui-app");

    let out = scaffold(&db, &dest, "react-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let before = file_bytes(&dest);
    let to = "platform-ui-web@0.2.0";
    let args = ["kit", "upgrade", &dest.display().to_string(), "--to", to];

    // Review is the default, and it is read-only: a reviewable per-file diff
    // is what the operator sees before anything is applied.
    let out = run(&db, &args);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let human = lossy(&out.stdout);
    assert!(human.contains("nothing was written"), "{human}");
    assert!(human.contains(".platform/tokens/tokens.css"), "{human}");
    assert!(
        human.contains("apply with `forge kit upgrade --confirm`"),
        "{human}"
    );
    assert_eq!(before, file_bytes(&dest), "a review wrote nothing");

    // Confirmation applies it, and says which files it touched.
    let mut confirmed = args.to_vec();
    confirmed.push("--confirm");
    let out = run_json(&db, &confirmed);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["to"], to);
    let written: Vec<String> = serde_json::from_value(value["written"].clone()).unwrap();
    assert!(
        written.contains(&".platform/receipt.json".to_string()),
        "{written:?}"
    );
}

#[test]
fn an_unavailable_upgrade_is_refused_with_a_reason_and_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("rs-app");

    let out = scaffold(&db, &dest, "rust-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let before = file_bytes(&dest);

    // A declared zero pinned no kit, so there is nothing to move. The command
    // has to refuse: reporting "already current" is the one outcome this state
    // must never produce.
    let out = run(
        &db,
        &[
            "kit",
            "upgrade",
            &dest.display().to_string(),
            "--to",
            "platform-dotnet@0.1.0",
            "--confirm",
        ],
    );
    assert_ne!(
        out.status.code(),
        Some(0),
        "an unavailable upgrade is a refusal, not a pass: {}",
        lossy(&out.stdout)
    );
    let err = lossy(&out.stderr);
    assert!(err.contains("error[kit-unknown]"), "{err}");
    assert!(err.contains("kit upgrade unavailable"), "{err}");
    assert!(err.contains("pinned no shared-layer kit"), "{err}");
    assert_eq!(
        before,
        file_bytes(&dest),
        "a refused upgrade changed nothing"
    );
}

#[test]
fn the_pinned_kit_is_observed_on_the_project_record() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    // A kit-bearing project records its pinned id and version.
    let net = tmp.path().join("net-app");
    let out = scaffold(&db, &net, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let out = run_json(&db, &["inspect", "net-app"]);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["kit_id"], "platform-dotnet");
    assert_eq!(value["kit_version"], "0.1.0");

    // A declared zero records the sentinel, not a null.
    let flut = tmp.path().join("mbl-app");
    let out = scaffold(&db, &flut, "flutter-app");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let out = run_json(&db, &["inspect", "mbl-app"]);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["kit_id"], "none");
    assert_eq!(value["kit_version"], serde_json::Value::Null);
}

#[test]
fn profile_inspect_renders_the_declared_kit() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let out = run(&db, &["profile", "inspect", "aspnet-web"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("kit: platform-dotnet@0.1.0"), "{stdout}");
    assert!(stdout.contains("kit ecosystem: dotnet"), "{stdout}");
    assert!(stdout.contains("kit tfm: net10.0"), "{stdout}");
    assert!(
        stdout.contains("kit feed: platform (nuget) at packages/platform-feed"),
        "{stdout}"
    );
    assert!(stdout.contains("kit minimum_packages: 6"), "{stdout}");
    assert!(stdout.contains("kit confirmed:"), "{stdout}");

    // A declared zero is shown as a zero, never as a met floor.
    let out = run(&db, &["profile", "inspect", "rust-web"]);
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("kit minimum_packages: 0 (declared zero, not a met floor)"),
        "{stdout}"
    );
    assert!(stdout.contains("kit zero_reason:"), "{stdout}");
    // A declared zero is a *recorded* state, not a missing registry row. The
    // `none` sentinel is by design, so it must never be reported as an
    // unregistered reference.
    assert!(
        stdout.contains("kit confirmed: none (declared zero)"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("not in the compiled-in kit registry"),
        "a declared zero is not an unregistered kit: {stdout}"
    );
}
