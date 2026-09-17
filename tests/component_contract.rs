//! Semantic component registry contract
//! (`semantic-component-registry`).
//!
//! Covers the new `forge component` command surface end to end through
//! the built binary: catalog discovery (R1 success: PaginatedQuery and
//! IdempotencyGuard resolved for supported profiles), contract inspection
//! with the typed inputs/outputs/evidence, primitive rejection (R1
//! failure: `if`/`loop` refused with the missing-criteria wording),
//! profile-incompatibility rejection (R1 boundary: a UI component not
//! installable on a server profile and a server component not
//! installable on a UI profile), quality-aware selection (R2 success:
//! the certified component is preferred and the evidence is reported),
//! evidence-gated promotion (R2 failure: missing security review or
//! stale verification refuses the promotion and the prior quality
//! level is preserved), and the policy-conflict boundary (R2
//! boundary: a deprecated-only request is reported as a quality
//! conflict rather than silently selected).

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
fn component_help_lists_the_new_subcommands() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["component", "--help"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    for sub in ["list", "inspect", "resolve", "qualify"] {
        assert!(
            stdout.contains(sub),
            "missing subcommand `{sub}` in help: {stdout}"
        );
    }
}

#[test]
fn list_returns_canonical_ids_and_qualities() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(&db, &["component", "list"]);
    assert_eq!(code, Some(0));
    let components = value["components"].as_array().unwrap();
    let ids: Vec<&str> = components
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    for id in [
        "paginated-query",
        "idempotency-guard",
        "validated-form",
        "audit-action",
        "soft-delete",
        "retry-external-call",
        "require-permission",
        "api-mutation",
        "loading-state",
        "error-boundary",
        "confirm-dialog",
        "empty-state",
        "toast",
        "file-picker",
        "webhook-receiver",
    ] {
        assert!(ids.contains(&id), "missing catalog id `{id}` in {ids:?}");
    }
    let qualities: std::collections::BTreeSet<String> = components
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
fn inspect_returns_contract_and_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(&db, &["component", "inspect", "paginated-query"]);
    assert_eq!(code, Some(0));
    assert_eq!(value["id"], "paginated-query");
    assert_eq!(value["version"], "0.1.0");
    assert_eq!(value["quality"], "certified");
    let profiles = value["profiles"].as_array().unwrap();
    let profiles: Vec<&str> = profiles.iter().map(|p| p.as_str().unwrap()).collect();
    assert!(profiles.contains(&"rust-web"));
    assert!(profiles.contains(&"python-service"));
    let inputs = value["contract"]["inputs"].as_array().unwrap();
    assert!(!inputs.is_empty(), "contract must declare inputs");
    let outputs = value["contract"]["outputs"].as_array().unwrap();
    assert!(!outputs.is_empty(), "contract must declare outputs");
    assert!(value["evidence"]["security_review"].as_bool().unwrap());
    assert!(
        value["evidence"]["test_coverage"].as_f64().unwrap() >= 0.85,
        "certified components must declare coverage at or above 0.85"
    );
}

#[test]
fn resolve_known_components_succeeds_with_certified_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "paginated-query",
            "--component",
            "idempotency-guard",
        ],
    );
    let code = out.status.code();
    assert_eq!(code, Some(0), "stderr: {}", lossy(&out.stderr));
    let (value, _) = run_json(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "paginated-query",
            "--component",
            "idempotency-guard",
        ],
    );
    let plan = &value["plan"];
    let steps = plan["steps"].as_array().unwrap();
    let step_ids: Vec<&str> = steps.iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert!(step_ids.contains(&"paginated-query"));
    assert!(step_ids.contains(&"idempotency-guard"));
    assert!(plan["rejections"].as_array().unwrap().is_empty());
    let summary = value["evidence_summary"].as_array().unwrap();
    assert_eq!(summary.len(), 2);
    for entry in summary {
        assert_eq!(entry["quality"], "certified");
    }
}

#[test]
fn resolve_profile_incompatibility_surfaces_typed_rejection() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "flutter-app",
            "--component",
            "paginated-query",
        ],
    );
    assert_eq!(code, Some(0));
    let plan = &value["plan"];
    assert!(plan["steps"].as_array().unwrap().is_empty());
    let rejections = plan["rejections"].as_array().unwrap();
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0]["code"], "component-invalid");
    assert!(rejections[0]["reason"]
        .as_str()
        .unwrap()
        .contains("flutter-app"));
}

#[test]
fn resolve_unknown_component_reports_typed_rejection() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _code) = run_json(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "made-up",
        ],
    );
    let rejections = value["plan"]["rejections"].as_array().unwrap();
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0]["code"], "component-invalid");
    assert!(rejections[0]["reason"]
        .as_str()
        .unwrap()
        .contains("not in the catalog"));
}

#[test]
fn resolve_only_deprecated_reports_quality_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _code) = run_json(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "webhook-receiver",
        ],
    );
    let rejections = value["plan"]["rejections"].as_array().unwrap();
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0]["code"], "component-quality-conflict");
    assert!(rejections[0]["reason"]
        .as_str()
        .unwrap()
        .contains("only candidate"));
}

#[test]
fn resolve_primitive_id_is_refused_with_component_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "if",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[component-invalid]"), "{stderr}");
    assert!(stderr.contains("programming primitive"), "{stderr}");
}

#[test]
fn resolve_ui_component_on_server_profile_reports_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _code) = run_json(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "toast",
        ],
    );
    let rejections = value["plan"]["rejections"].as_array().unwrap();
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0]["code"], "component-invalid");
    assert!(rejections[0]["reason"]
        .as_str()
        .unwrap()
        .contains("no implementation"));
}

#[test]
fn resolve_server_component_on_ui_profile_reports_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _code) = run_json(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "nextjs-web",
            "--component",
            "paginated-query",
        ],
    );
    let rejections = value["plan"]["rejections"].as_array().unwrap();
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0]["code"], "component-invalid");
    assert!(rejections[0]["reason"]
        .as_str()
        .unwrap()
        .contains("no implementation"));
}

#[test]
fn qualify_missing_security_review_refuses_and_preserves_prior_quality() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    fs::create_dir_all(&work).unwrap();
    let (value, _code) = run_json(
        &db,
        &[
            "component",
            "qualify",
            "toast",
            "--to",
            "certified",
            "--reason",
            "no security review",
            "--coverage",
            "0.95",
            "--security-review",
            "false",
            "--path",
            work.to_str().unwrap(),
        ],
    );
    assert_eq!(value["promoted"], false);
    assert_eq!(value["prior_quality"], "verified");
    assert_eq!(value["target_quality"], "certified");
    assert!(value["note"]
        .as_str()
        .unwrap()
        .contains("missing security review"));
    assert!(value["files_written"].as_array().unwrap().is_empty());
    // The receipt must not be written on refusal.
    assert!(!work.join(".forge/components/toast/qualify.json").exists());
}

#[test]
fn qualify_stale_verification_refuses_promotion() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    fs::create_dir_all(&work).unwrap();
    let (value, _code) = run_json(
        &db,
        &[
            "component",
            "qualify",
            "toast",
            "--to",
            "certified",
            "--reason",
            "stale evidence",
            "--coverage",
            "0.95",
            "--last-verified",
            "2000-01-01T00:00:00Z",
            "--path",
            work.to_str().unwrap(),
        ],
    );
    assert_eq!(value["promoted"], false);
    assert!(value["note"].as_str().unwrap().contains("freshness window"));
}

#[test]
fn qualify_complete_evidence_promotes_and_writes_receipt() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    fs::create_dir_all(&work).unwrap();
    let (value, _code) = run_json(
        &db,
        &[
            "component",
            "qualify",
            "toast",
            "--to",
            "certified",
            "--reason",
            "production ready",
            "--coverage",
            "0.95",
            "--path",
            work.to_str().unwrap(),
        ],
    );
    assert_eq!(value["promoted"], true);
    assert_eq!(value["prior_quality"], "verified");
    assert_eq!(value["target_quality"], "certified");
    let files = value["files_written"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], ".forge/components/toast/qualify.json");
    let body = fs::read_to_string(work.join(".forge/components/toast/qualify.json")).unwrap();
    assert!(body.contains("\"target_quality\": \"certified\""));
    assert!(body.contains("\"security_review\": true"));
}

#[test]
fn qualify_unknown_target_quality_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(
        &db,
        &[
            "component",
            "qualify",
            "toast",
            "--to",
            "shiny",
            "--reason",
            "unknown",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(lossy(&out.stderr).contains("error[component-invalid]"));
    assert!(lossy(&out.stderr).contains("unknown quality 'shiny'"));
}

#[test]
fn qualify_refusal_does_not_create_dot_forge_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    fs::create_dir_all(&work).unwrap();
    let _ = run(
        &db,
        &[
            "component",
            "qualify",
            "toast",
            "--to",
            "certified",
            "--reason",
            "no security review",
            "--coverage",
            "0.95",
            "--security-review",
            "false",
            "--path",
            work.to_str().unwrap(),
        ],
    );
    assert!(!work.join(".forge").exists());
}
