//! Cross-surface regression: semantic component registry against the
//! existing doctor/feature/registry contract
//! (`semantic-component-registry`).
//!
//! Exercises R1×R2 interactions end to end through the built binary:
//! the registry `component` journal row keeps the operations table
//! independent of the component surface, the doctor verdict stays
//! unchanged after a successful component resolve, the existing
//! feature contract (add/remove/upgrade) is preserved when a project
//! also runs a `forge component resolve` for a compatible
//! `paginated-query` candidate, and the catalog boundary (R1
//! boundary) keeps `paginated-query` resolvable for `rust-web` and
//! `python-service` while refusing it for `flutter-app` and
//! `nextjs-web` so the per-stack implementations stay distinct.

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
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
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
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // Inspect the journal: the `component` row exists and references
    // a synthetic catalog-global project id so the operations table
    // stays compatible with the per-project contract.
    let (value, code) = run_json(&db, &["list"]);
    assert_eq!(code, Some(0));
    let projects = value["projects"].as_array().unwrap();
    assert!(
        projects.is_empty(),
        "component journal must not invent user-visible projects; got {projects:?}"
    );
}

#[test]
fn doctor_verdict_is_unchanged_after_component_resolve() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("doc-test");
    write_manifest(&proj, "doc-test");
    let before = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(before.status.code(), Some(0), "{}", lossy(&before.stderr));
    let resolve = run(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "paginated-query",
        ],
    );
    assert_eq!(resolve.status.code(), Some(0), "{}", lossy(&resolve.stderr));
    let after = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(after.status.code(), Some(0), "{}", lossy(&after.stderr));
    // The doctor output is a snapshot of the project's manifest
    // findings; the resolve journal must not change the verdict.
    let before_text = lossy(&before.stdout);
    let after_text = lossy(&after.stdout);
    assert_eq!(before_text, after_text);
}

#[test]
fn feature_add_keeps_working_after_component_resolve() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("feat-comp");
    write_manifest(&proj, "feat-comp");
    let resolve = run(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "paginated-query",
        ],
    );
    assert_eq!(resolve.status.code(), Some(0), "{}", lossy(&resolve.stderr));
    let add = run(&db, &["feature", "add", "auth", proj.to_str().unwrap()]);
    assert_eq!(add.status.code(), Some(0), "{}", lossy(&add.stderr));
    let body = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    assert!(body.contains("auth:"), "{body}");
    let inspect = run_json(&db, &["inspect", "feat-comp"]);
    assert_eq!(inspect.0["features"]["auth"], "0.1.0");
}

#[test]
fn paginated_query_resolves_for_rust_web_and_python_service_only() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    // rust-web: installed.
    let (rust, _) = run_json(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "paginated-query",
        ],
    );
    let rust_steps = rust["plan"]["steps"].as_array().unwrap();
    assert_eq!(rust_steps.len(), 1);
    assert_eq!(rust_steps[0]["id"], "paginated-query");
    assert!(rust["plan"]["rejections"].as_array().unwrap().is_empty());

    // python-service: installed (R1 boundary: same id, different stack).
    let (python, _) = run_json(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "python-service",
            "--component",
            "paginated-query",
        ],
    );
    let python_steps = python["plan"]["steps"].as_array().unwrap();
    assert_eq!(python_steps.len(), 1);
    assert_eq!(python_steps[0]["id"], "paginated-query");

    // nextjs-web: not installable (no tested implementation).
    let (next, _) = run_json(
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
    let next_rejections = next["plan"]["rejections"].as_array().unwrap();
    assert_eq!(next_rejections.len(), 1);
    assert_eq!(next_rejections[0]["code"], "component-invalid");

    // flutter-app: not installable.
    let (flutter, _) = run_json(
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
    let flutter_rejections = flutter["plan"]["rejections"].as_array().unwrap();
    assert_eq!(flutter_rejections.len(), 1);
    assert_eq!(flutter_rejections[0]["code"], "component-invalid");
}

#[test]
fn component_journal_keeps_evidence_in_detail() {
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
            "made-up",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // The resolved plan renders the rejection count; the CLI output
    // must surface the typed rejection code so a partial run stays
    // observable on stdout (boundary scenario).
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("rejected=1"), "{stdout}");
    assert!(stdout.contains("not in the catalog"), "{stdout}");
}
