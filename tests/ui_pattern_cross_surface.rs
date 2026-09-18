//! Cross-surface regression: semantic UI patterns against the
//! existing doctor/feature/registry contract
//! (`semantic-ui-patterns`).
//!
//! Exercises R1×R2 interactions end to end through the built
//! binary: the registry `ui_pattern` journal row keeps the
//! operations table independent of the UI surface, the doctor
//! verdict stays unchanged after a successful UI pattern
//! resolve and install, the existing feature contract
//! (add/remove/upgrade) is preserved when a project also runs a
//! `forge ui-pattern install`, the R1 boundary keeps a web-only
//! pattern from being silently copied into a Flutter profile,
//! and the R2 boundary keeps the installed source artifact on
//! disk after the receipt is removed (Forge is no longer the
//! build path).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_DRIFTWATCH_BIN");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> (serde_json::Value, Option<i32>) {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_DRIFTWATCH_BIN");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    let code = out.status.code();
    let value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stderr={}", lossy(&out.stderr)));
    (value, code)
}

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_manifest(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: react-web\n  maturity: L1\nruntime:\n  language: typescript\n"
        ),
    )
    .unwrap();
}

#[test]
fn resolve_journal_entry_keeps_operations_table_independent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(
        &db,
        &[
            "ui-pattern",
            "resolve",
            "--profile",
            "react-web",
            "--pattern",
            "login",
            "--pattern",
            "form",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // The `ui_pattern` journal row uses a synthetic catalog-
    // global project id so the operations table stays
    // compatible with the per-project contract.
    let (value, code) = run_json(&db, &["list"]);
    assert_eq!(code, Some(0));
    let projects = value["projects"].as_array().unwrap();
    assert!(
        projects.is_empty(),
        "ui-pattern journal must not invent user-visible projects; got {projects:?}"
    );
}

#[test]
fn doctor_verdict_is_unchanged_after_ui_pattern_resolve_and_install() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("ui-doc");
    write_manifest(&proj, "ui-doc");
    let before = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(before.status.code(), Some(0), "{}", lossy(&before.stderr));
    let resolve = run(
        &db,
        &[
            "ui-pattern",
            "resolve",
            "--profile",
            "react-web",
            "--pattern",
            "form",
        ],
    );
    assert_eq!(resolve.status.code(), Some(0), "{}", lossy(&resolve.stderr));
    let install = run(
        &db,
        &[
            "ui-pattern",
            "install",
            "form",
            "--profile",
            "react-web",
            "--reason",
            "doctor regression smoke",
            "--path",
            proj.to_str().unwrap(),
        ],
    );
    assert_eq!(install.status.code(), Some(0), "{}", lossy(&install.stderr));
    let after = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(after.status.code(), Some(0), "{}", lossy(&after.stderr));
    let before_text = lossy(&before.stdout);
    let after_text = lossy(&after.stdout);
    assert_eq!(before_text, after_text);
}

#[test]
fn feature_add_keeps_working_after_ui_pattern_install() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("ui-feat");
    write_manifest(&proj, "ui-feat");
    let install = run(
        &db,
        &[
            "ui-pattern",
            "install",
            "form",
            "--profile",
            "react-web",
            "--reason",
            "feature-cross smoke",
            "--path",
            proj.to_str().unwrap(),
        ],
    );
    assert_eq!(install.status.code(), Some(0), "{}", lossy(&install.stderr));
    let add = run(&db, &["feature", "add", "auth", proj.to_str().unwrap()]);
    assert_eq!(add.status.code(), Some(0), "{}", lossy(&add.stderr));
    let body = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    assert!(body.contains("auth:"), "{body}");
    let inspect = run_json(&db, &["inspect", "ui-feat"]);
    assert_eq!(inspect.0["features"]["auth"], "0.1.0");
    // The UI pattern artifact and the feature receipt coexist
    // in the same project; the install path is independent of
    // the feature path.
    assert!(proj.join("src/ui/form.tsx").exists());
    assert!(proj.join(".forge/features/auth.receipt").exists());
}

#[test]
fn flutter_app_resolve_does_not_substitute_copied_web_markup() {
    // R1 boundary: a request for `billing` on `flutter-app`
    // must be refused with the typed
    // `ui-pattern-unsupported-platform` rejection, listing the
    // tested platforms, and the planner must not silently
    // substitute a copy-pasted web artifact into the Flutter
    // project.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("ui-flt");
    fs::create_dir_all(&proj).unwrap();
    let out = run(
        &db,
        &[
            "ui-pattern",
            "resolve",
            "--profile",
            "flutter-app",
            "--pattern",
            "billing",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("ui-pattern-unsupported-platform"),
        "{stdout}"
    );
    assert!(stdout.contains("flutter-app"));
    assert!(stdout.contains("react-web"));
    // No artifact was written.
    assert!(!proj.join("lib/ui/billing.dart").exists());
    assert!(!proj.join("src/ui/billing.tsx").exists());
}

#[test]
fn installed_artifact_survives_receipt_removal() {
    // R2 boundary: after `forge ui-pattern install`, the
    // installed source artifact is ordinary source at the
    // expected path; removing the receipt (simulating "Forge
    // is removed") must leave the source on disk and the
    // project must continue to build through the native
    // toolchain. The contract checks the file shape (ordinary
    // source with the documented export) so a real
    // native-tool build remains a downstream integration step.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("ui-boundary");
    write_manifest(&proj, "ui-boundary");
    let out = run(
        &db,
        &[
            "ui-pattern",
            "install",
            "navigation",
            "--profile",
            "react-web",
            "--reason",
            "boundary smoke",
            "--path",
            proj.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let artifact = proj.join("src/ui/navigation.tsx");
    assert!(artifact.exists());
    fs::remove_file(proj.join(".forge/ui-patterns/navigation/install.json")).unwrap();
    let body = fs::read_to_string(&artifact).unwrap();
    assert!(body.contains("function Navigation("), "{body}");
    assert!(body.contains("import { useState } from 'react';"));
    assert!(body.contains("export default Navigation;"));
    // The receipt is gone, but the source is on disk: Forge is
    // not on the build path.
    assert!(!proj
        .join(".forge/ui-patterns/navigation/install.json")
        .exists());
    assert!(artifact.exists());
}
