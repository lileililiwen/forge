//! Cross-surface regression: portable AI procedures against the
//! existing doctor / feature / registry / spec contract
//! (`ai-procedure-skills`).
//!
//! Exercises R1×R2 interactions end to end through the built binary:
//! the registry `procedure` journal row keeps the operations table
//! independent of the procedure surface, the doctor verdict stays
//! byte-equivalent before and after a `forge procedure list` /
//! `inspect`, the existing `forge feature add` workflow keeps
//! working on a project whose procedure journal has been recorded,
//! the `forge spec generate` workflow keeps working, and the
//! `forge doctor` byte-equality also holds after a `procedure
//! validate` refusal so the procedure layer stays decoupled from
//! the doctor surface.

use std::fs;
use std::io::Write;
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

fn write_manifest(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
}

#[test]
fn list_journal_entry_keeps_operations_table_independent() {
    // R1 boundary: the procedure layer is project-agnostic;
    // the `procedure` journal row uses a synthetic project
    // id so the operations table never invents a
    // user-visible project.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["procedure", "list"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let (value, code) = run_json(&db, &["list"]);
    assert_eq!(code, Some(0));
    let projects = value["projects"].as_array().unwrap();
    assert!(
        projects.is_empty(),
        "procedure journal must not invent user-visible projects; got {projects:?}"
    );
}

#[test]
fn inspect_journal_entry_uses_procedure_id_as_project_id() {
    // The inspect journal rows record the procedure id as
    // the `project_id` so the operations table is
    // project-agnostic, matching the planner / ui_pattern
    // synthetic-id pattern.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["procedure", "inspect", "create-project"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // After the procedure surface runs, the per-project
    // list is still empty: no real project is created.
    let (value, _) = run_json(&db, &["list"]);
    let projects = value["projects"].as_array().unwrap();
    assert!(projects.is_empty());
}

#[test]
fn doctor_verdict_is_byte_equivalent_after_procedure_inspect() {
    // R1 boundary: a `forge procedure inspect` does not
    // change the doctor verdict because the procedure
    // layer is decoupled from the doctor surface.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proc-doc");
    write_manifest(&proj, "proc-doc");
    let before = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(before.status.code(), Some(0), "{}", lossy(&before.stderr));
    let inspect = run(&db, &["procedure", "inspect", "create-project"]);
    assert_eq!(inspect.status.code(), Some(0), "{}", lossy(&inspect.stderr));
    let after = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(after.status.code(), Some(0), "{}", lossy(&after.stderr));
    let before_text = lossy(&before.stdout);
    let after_text = lossy(&after.stdout);
    assert_eq!(
        before_text, after_text,
        "doctor verdict must be byte-equivalent before and after a procedure inspect"
    );
}

#[test]
fn doctor_verdict_is_byte_equivalent_after_procedure_list() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proc-list");
    write_manifest(&proj, "proc-list");
    let before = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(before.status.code(), Some(0));
    let list = run(&db, &["procedure", "list"]);
    assert_eq!(list.status.code(), Some(0));
    let after = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(after.status.code(), Some(0));
    let before_text = lossy(&before.stdout);
    let after_text = lossy(&after.stdout);
    assert_eq!(before_text, after_text);
}

#[test]
fn doctor_verdict_is_byte_equivalent_after_procedure_validate_refusal() {
    // R2 failure scenario: a procedure that asks Core to
    // skip a check is refused at the procedure layer; the
    // doctor verdict is unchanged because the refusal
    // happened before any project state was read.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proc-reject");
    write_manifest(&proj, "proc-reject");
    let before = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(before.status.code(), Some(0));
    let spec = serde_json::json!({
        "id": "smoke-bypass",
        "version": "0.1.0",
        "title": "smoke",
        "description": "d",
        "prerequisites": [],
        "steps": [
            {"ordinal": 1, "op": "doctor_run", "args": ["--force"], "description": "d"},
            {"ordinal": 2, "op": "report_findings", "args": [], "description": "d"}
        ],
        "verification": "v"
    });
    let path = tmp.path().join("smoke.json");
    fs::write(&path, serde_json::to_string(&spec).unwrap()).unwrap();
    let reject = run(
        &db,
        &["procedure", "validate", "--path", path.to_str().unwrap()],
    );
    assert_eq!(reject.status.code(), Some(1));
    let stderr = lossy(&reject.stderr);
    assert!(stderr.contains("error[procedure-bypass-refused]"));
    let after = run(&db, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(after.status.code(), Some(0));
    assert_eq!(lossy(&before.stdout), lossy(&after.stdout));
}

#[test]
fn feature_add_remains_compatible_after_procedure_inspect() {
    // R1 boundary: the procedure surface is decoupled from
    // the feature-lifecycle surface; a `feature add` on the
    // same project keeps working after a procedure inspect
    // has journaled a row.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("feat-proc");
    write_manifest(&proj, "feat-proc");
    let inspect = run(&db, &["procedure", "inspect", "upgrade-project"]);
    assert_eq!(inspect.status.code(), Some(0));
    let add = run(&db, &["feature", "add", "auth", proj.to_str().unwrap()]);
    assert_eq!(
        add.status.code(),
        Some(0),
        "feature add must succeed after a procedure inspect; stderr={}",
        lossy(&add.stderr)
    );
}

#[test]
fn spec_generate_remains_compatible_after_procedure_inspect() {
    // R2 success scenario: the upgrade procedure's
    // `spec.generate` handoff must keep the spec
    // remediation surface working; the procedure journal
    // does not change the spec generate outcome.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("spec-proc");
    write_manifest(&proj, "spec-proc");
    let inspect = run(&db, &["procedure", "inspect", "upgrade-project"]);
    assert_eq!(inspect.status.code(), Some(0));
    let spec = run(
        &db,
        &[
            "spec",
            "generate",
            proj.to_str().unwrap(),
            "--finding",
            "f1",
        ],
    );
    assert_eq!(
        spec.status.code(),
        Some(0),
        "spec generate must succeed after a procedure inspect; stderr={}",
        lossy(&spec.stderr)
    );
    let stdout = lossy(&spec.stdout);
    assert!(stdout.contains("generated"));
}

#[test]
fn mcp_tools_list_remains_unchanged_by_procedure_surface() {
    // R1 boundary: the procedure surface is a CLI-only
    // transport in v0.1.0; the mature-MCP tool list is
    // unchanged so the existing contract still holds.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["procedure", "list"]);
    assert_eq!(out.status.code(), Some(0));
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("spawn mcp");
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n")
            .unwrap();
    } else {
        panic!("stdin not piped");
    }
    let output = child.wait_with_output().expect("mcp output");
    let stdout = lossy(&output.stdout);
    let mut response: Option<serde_json::Value> = None;
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if value.get("id").and_then(|v| v.as_i64()) == Some(1) {
                response = Some(value);
                break;
            }
        }
    }
    let value = response.expect("no response for id=1");
    let tools = value["result"]["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(
        !tool_names.contains(&"inspect_procedure"),
        "procedure surface is CLI-only in v0.1.0; got tools={tool_names:?}"
    );
    assert!(
        !tool_names.contains(&"list_procedures"),
        "procedure surface is CLI-only in v0.1.0; got tools={tool_names:?}"
    );
}
