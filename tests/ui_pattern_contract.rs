//! Semantic UI pattern contract (`semantic-ui-patterns`).
//!
//! Covers the new `forge ui-pattern` command surface end to end
//! through the built binary: catalog discovery (R1 success: the
//! required intents are present), contract inspection with the
//! state/design/evidence envelopes, primitive rejection (R1
//! failure: `if`/`html-fragment` refused with the missing-
//! criteria wording), profile-incompatibility rejection (R1
//! boundary: a web-only pattern refuses `flutter-app` instead of
//! copying web markup), quality-aware selection (R2 success:
//! the certified pattern is preferred and the evidence is
//! reported), and ownership conflict (R2 failure: install
//! refuses to overwrite a customized file). The R2 boundary
//! (Forge removed after install: the artifact still builds
//! through the project native toolchain) is exercised as a
//! file-shape check: the installed artifact is ordinary source
//! at the expected path; Forge is not on the build path.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_DRIFTWATCH_BIN");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> (serde_json::Value, Option<i32>) {
    let mut cmd = clean_cmd();
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

#[test]
fn ui_pattern_help_lists_the_new_subcommands() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["ui-pattern", "--help"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    for sub in ["list", "inspect", "resolve", "install"] {
        assert!(
            stdout.contains(sub),
            "missing subcommand `{sub}` in help: {stdout}"
        );
    }
}

#[test]
fn list_returns_canonical_intents_and_qualities() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(&db, &["ui-pattern", "list"]);
    assert_eq!(code, Some(0));
    let patterns = value["ui_patterns"].as_array().unwrap();
    let ids: Vec<&str> = patterns.iter().map(|c| c["id"].as_str().unwrap()).collect();
    for id in [
        "login",
        "register",
        "forgot-password",
        "dashboard",
        "crud-table",
        "filter-bar",
        "form",
        "settings",
        "profile",
        "billing",
        "empty-state",
        "success-page",
        "error-page",
        "modal",
        "confirm-dialog",
        "file-upload",
        "navigation",
    ] {
        assert!(ids.contains(&id), "missing catalog id `{id}` in {ids:?}");
    }
    let qualities: std::collections::BTreeSet<String> = patterns
        .iter()
        .map(|c| c["quality"].as_str().unwrap().to_string())
        .collect();
    for q in ["certified", "verified", "experimental", "deprecated"] {
        assert!(
            qualities.contains(q),
            "catalog must include quality level `{q}` (got {qualities:?})"
        );
    }
}

#[test]
fn inspect_returns_state_design_and_evidence_envelopes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(&db, &["ui-pattern", "inspect", "form"]);
    assert_eq!(code, Some(0));
    assert_eq!(value["id"], "form");
    assert_eq!(value["intent"], "form");
    let states = value["states"].as_array().unwrap();
    let names: Vec<&str> = states.iter().map(|s| s["name"].as_str().unwrap()).collect();
    for required in [
        "loading",
        "error",
        "success",
        "form_validation",
        "empty",
        "keyboard_focus",
    ] {
        assert!(
            names.contains(&required),
            "missing required state {required} in {names:?}"
        );
    }
    assert!(!value["adapters"].as_array().unwrap().is_empty());
    assert!(value["evidence"]["test_coverage"].is_number());
    assert!(value["evidence"]["security_review"].is_boolean());
    assert!(value["accessibility"]["keyboard"].is_string());
    assert!(value["typography"]["family"].is_string());
}

#[test]
fn resolve_certified_patterns_for_react_web_reports_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(
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
    assert_eq!(code, Some(0));
    let steps = value["plan"]["steps"].as_array().unwrap();
    let ids: Vec<&str> = steps.iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"login"));
    assert!(ids.contains(&"form"));
    assert!(value["plan"]["rejections"].as_array().unwrap().is_empty());
    let summary = value["evidence_summary"].as_array().unwrap();
    for entry in summary {
        assert_eq!(entry["quality"], "certified");
        assert!(entry["evidence"]["security_review"].as_bool().unwrap());
    }
}

#[test]
fn resolve_flutter_app_refuses_web_only_pattern_without_copying_markup() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    // `billing` ships for react-web and nextjs-web only. The
    // R1 boundary scenario requires the resolver to refuse
    // `flutter-app` rather than substituting copied web markup.
    let (value, code) = run_json(
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
    assert_eq!(code, Some(0));
    assert!(value["plan"]["steps"].as_array().unwrap().is_empty());
    let rejections = value["plan"]["rejections"].as_array().unwrap();
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0]["code"], "ui-pattern-unsupported-platform");
    let reason = rejections[0]["reason"].as_str().unwrap();
    assert!(reason.contains("flutter-app"), "{reason}");
    assert!(reason.contains("react-web"), "{reason}");
    assert!(
        reason.contains("tested:"),
        "the reason must name the tested platforms so the operator sees the boundary: {reason}"
    );
}

#[test]
fn resolve_unknown_pattern_surfaces_typed_rejection() {
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
            "nosuch",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("ui-pattern-invalid"),
        "stdout should carry the typed code: {stdout}"
    );
    assert!(stdout.contains("nosuch"));
}

#[test]
fn resolve_primitive_id_surfaces_typed_rejection() {
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
            "if",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[ui-pattern-invalid]"),
        "stderr should carry the typed code: {stderr}"
    );
    assert!(stderr.contains("programming primitive"), "{stderr}");
}

#[test]
fn resolve_placeholder_id_surfaces_typed_rejection() {
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
            "screenshot",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[ui-pattern-invalid]"),
        "stderr should carry the typed code: {stderr}"
    );
    assert!(stderr.contains("screenshot"), "{stderr}");
}

#[test]
fn install_writes_artifact_and_receipt_for_react_web() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    let (value, code) = run_json(
        &db,
        &[
            "ui-pattern",
            "install",
            "form",
            "--profile",
            "react-web",
            "--reason",
            "studio needs the standard form",
            "--path",
            project.to_str().unwrap(),
        ],
    );
    assert_eq!(
        code,
        Some(0),
        "stderr: {}",
        lossy(&run(&db, &["ui-pattern", "list"]).stderr)
    );
    assert_eq!(value["installed"], true);
    let artifact = fs::read_to_string(project.join("src/ui/form.tsx")).unwrap();
    assert!(
        artifact.contains("function Form("),
        "artifact body: {artifact}"
    );
    assert!(
        artifact.contains("data-state="),
        "artifact must surface the typed state: {artifact}"
    );
    let receipt = fs::read_to_string(project.join(".forge/ui-patterns/form/install.json")).unwrap();
    assert!(receipt.contains("\"pattern_id\": \"form\""));
    assert!(receipt.contains("\"profile\": \"react-web\""));
    assert!(receipt.contains("\"quality\": \"certified\""));
}

#[test]
fn install_writes_artifact_for_flutter_app() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    let (value, code) = run_json(
        &db,
        &[
            "ui-pattern",
            "install",
            "form",
            "--profile",
            "flutter-app",
            "--reason",
            "mobile build needs the form surface",
            "--path",
            project.to_str().unwrap(),
        ],
    );
    assert_eq!(code, Some(0));
    assert_eq!(value["installed"], true);
    let artifact = fs::read_to_string(project.join("lib/ui/form.dart")).unwrap();
    assert!(artifact.contains("class Form"), "{artifact}");
    assert!(artifact.contains("Semantics"), "{artifact}");
    assert!(artifact.contains("'form'"), "{artifact}");
}

#[test]
fn install_refuses_to_overwrite_a_customized_artifact() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    fs::create_dir_all(project.join("src/ui")).unwrap();
    fs::write(
        project.join("src/ui/form.tsx"),
        "// user already customized this\n",
    )
    .unwrap();
    let out = run(
        &db,
        &[
            "ui-pattern",
            "install",
            "form",
            "--profile",
            "react-web",
            "--reason",
            "should refuse",
            "--path",
            project.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[ui-pattern-ownership-conflict]"),
        "{stderr}"
    );
    // Customized file left intact.
    let preserved = fs::read_to_string(project.join("src/ui/form.tsx")).unwrap();
    assert!(preserved.contains("user already customized"));
    // Receipt must not have been written.
    assert!(!project
        .join(".forge/ui-patterns/form/install.json")
        .exists());
}

#[test]
fn install_refuses_unsupported_platform_with_typed_code() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    let out = run(
        &db,
        &[
            "ui-pattern",
            "install",
            "billing",
            "--profile",
            "flutter-app",
            "--reason",
            "no flutter surface",
            "--path",
            project.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[ui-pattern-unsupported-platform]"),
        "{stderr}"
    );
}

#[test]
fn install_refuses_unknown_id_with_typed_code() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    let out = run(
        &db,
        &[
            "ui-pattern",
            "install",
            "nosuch",
            "--profile",
            "react-web",
            "--reason",
            "missing",
            "--path",
            project.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[ui-pattern-invalid]"), "{stderr}");
    assert!(stderr.contains("nosuch"), "{stderr}");
}

#[test]
fn resolve_reports_quality_conflict_for_deprecated_only_request() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    // `webhook-receiver` is the deprecated test entry; its only
    // adapter ships for nextjs-web.
    let (value, code) = run_json(
        &db,
        &[
            "ui-pattern",
            "resolve",
            "--profile",
            "nextjs-web",
            "--pattern",
            "webhook-receiver",
        ],
    );
    assert_eq!(code, Some(0));
    assert!(value["plan"]["steps"].as_array().unwrap().is_empty());
    let rejections = value["plan"]["rejections"].as_array().unwrap();
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0]["code"], "ui-pattern-quality-conflict");
    let reason = rejections[0]["reason"].as_str().unwrap();
    assert!(reason.contains("only candidate"), "{reason}");
}

#[test]
fn install_writes_artifact_removable_after_install_keeps_source() {
    // R2 boundary: after `forge ui-pattern install`, the
    // installed artifact is ordinary source at the expected
    // path; removing the receipt must not delete the source.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    let out = run(
        &db,
        &[
            "ui-pattern",
            "install",
            "form",
            "--profile",
            "react-web",
            "--reason",
            "boundary smoke",
            "--path",
            project.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let artifact_path = project.join("src/ui/form.tsx");
    assert!(artifact_path.exists());
    // Simulate "Forge is removed": drop the receipt and
    // verify the source artifact is still on disk and remains
    // ordinary source.
    fs::remove_file(project.join(".forge/ui-patterns/form/install.json")).unwrap();
    let body = fs::read_to_string(&artifact_path).unwrap();
    assert!(body.contains("function Form("));
    assert!(body.contains("data-state="));
    assert!(body.contains("import { useState } from 'react';"));
}

#[test]
fn billing_lists_in_two_web_adapters() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "billing"]);
    let adapters: Vec<&str> = value["adapters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["profile"].as_str().unwrap())
        .collect();
    assert!(adapters.contains(&"react-web"));
    assert!(adapters.contains(&"nextjs-web"));
    assert!(!adapters.contains(&"flutter-app"));
}

#[test]
fn confirm_dialog_declares_choice_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "confirm-dialog"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"choice"));
    let choices: Vec<&str> = value["interaction"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(choices.contains(&"confirm"));
    assert!(choices.contains(&"cancel"));
}

#[test]
fn crud_table_exposes_sort_filter_paginate() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "crud-table"]);
    let choices: Vec<&str> = value["interaction"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    for expected in ["sort", "filter", "paginate"] {
        assert!(
            choices.contains(&expected),
            "missing {expected} in {choices:?}"
        );
    }
    let deps: Vec<&str> = value["depends_on"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap())
        .collect();
    assert!(deps.contains(&"paginated-query"), "{deps:?}");
}

#[test]
fn dashboard_lists_recent_activity() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "dashboard"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"recent_activity"));
}

#[test]
fn empty_state_supports_three_platforms() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "empty-state"]);
    let adapters: Vec<&str> = value["adapters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["profile"].as_str().unwrap())
        .collect();
    assert!(adapters.contains(&"react-web"));
    assert!(adapters.contains(&"nextjs-web"));
    assert!(adapters.contains(&"flutter-app"));
}

#[test]
fn error_page_lists_recovery_actions() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "error-page"]);
    let choices: Vec<&str> = value["interaction"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(choices.contains(&"retry"));
    assert!(choices.contains(&"go_home"));
}

#[test]
fn file_upload_lists_progress_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "file-upload"]);
    assert_eq!(value["quality"], "experimental");
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"upload_progress"));
}

#[test]
fn filter_bar_lists_clear_all() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "filter-bar"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"clear_all"));
    let choices: Vec<&str> = value["interaction"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(choices.contains(&"clear_all"));
}

#[test]
fn forgot_password_lists_pending_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "forgot-password"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"pending_recovery"));
}

#[test]
fn form_lists_pending_failure_success() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "form"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"pending"));
    assert!(states.contains(&"failure"));
    assert!(states.contains(&"success"));
}

#[test]
fn login_lists_authenticated_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "login"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"authenticated"));
}

#[test]
fn modal_lists_return_focus() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "modal"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"return_focus"));
}

#[test]
fn navigation_lists_skip_to_content() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "navigation"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"skip_to_content"));
}

#[test]
fn profile_lists_save_discard() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "profile"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"identity_fields"));
    let choices: Vec<&str> = value["interaction"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(choices.contains(&"save"));
    assert!(choices.contains(&"discard"));
}

#[test]
fn register_lists_welcome_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "register"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"welcome"));
}

#[test]
fn settings_lists_grouped_preferences() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "settings"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"grouped_preferences"));
}

#[test]
fn success_page_lists_continue() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "success-page"]);
    let states: Vec<&str> = value["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"confirmation"));
    let choices: Vec<&str> = value["interaction"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(choices.contains(&"continue"));
}

#[test]
fn webhook_receiver_is_deprecated() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["ui-pattern", "inspect", "webhook-receiver"]);
    assert_eq!(value["quality"], "deprecated");
}
