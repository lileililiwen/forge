//! Sisyphusfy-supervised `run-spec`: verdicts, refusals, and spec-validity boundaries.

use super::*;

#[test]
fn run_spec_sisyphusfy_verified_loop_journals_done_with_supervisor_source() {
    let f = Fixture::new("sup-done");
    let spec_id = "spec-done-1234567890ab";
    write_bound_spec(&f, spec_id);
    write_legacy_session(&f.proj, "sup-done", "sess-d", "codex", spec_id);
    let doc = outcome_doc(
        &f,
        "done.json",
        r#"{"stop_reason":"complete","iterations":3,"verification":{"status":"success","source":"configured"}}"#,
    );
    let value = f.agent_json(
        &[("SISY_OUTCOME", doc.to_str().unwrap()), ("SISY_EXIT", "0")],
        &[
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-d",
            "--provider",
            "sisyphusfy",
        ],
    );
    assert_eq!(value["transition"]["verdict"], "done");
    assert_eq!(value["transition"]["state"], "active");
    assert_eq!(value["transition"]["requested"], "start");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("supervisor: sisyphusfy"),
        "evidence={evidence}"
    );
    assert!(
        evidence.contains("verification: status=success source=configured"),
        "evidence={evidence}"
    );
    let log = f.sisy_log();
    assert!(
        log.contains(
            "loop --task-path .forge/specs/spec-done-1234567890ab/tasks.md --json --adapter codex"
        ),
        "log={log}"
    );
    let registry = forge::registry::Registry::open(&f.db).unwrap();
    let rows = registry.operations_for_project("sup-done", 10).unwrap();
    assert!(
        rows.iter().any(|r| r.kind == "agent"
            && r.state == "done"
            && r.detail
                .as_deref()
                .unwrap_or_default()
                .contains("verdict `done`")),
        "rows={:?}",
        rows.iter()
            .map(|r| (&r.kind, &r.state, &r.detail))
            .collect::<Vec<_>>()
    );
}

#[test]
fn run_spec_sisyphusfy_blocked_journals_partial_with_named_reason() {
    let f = Fixture::new("sup-partial");
    let spec_id = "spec-part-1234567890ab";
    write_bound_spec(&f, spec_id);
    write_legacy_session(&f.proj, "sup-partial", "sess-p3", "codex", spec_id);
    let doc = outcome_doc(
        &f,
        "blocked.json",
        r#"{"stop_reason":"blocked","iterations":2,"verification":{"status":"skipped","source":"unavailable"},"blocked_reason":["NEED_PERMISSION"]}"#,
    );
    let value = f.agent_json(
        &[("SISY_OUTCOME", doc.to_str().unwrap()), ("SISY_EXIT", "1")],
        &[
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-p3",
            "--provider",
            "sisyphusfy",
        ],
    );
    assert_eq!(value["transition"]["verdict"], "partial");
    let note = value["transition"]["note"].as_str().unwrap_or_default();
    assert!(note.contains("NEED_PERMISSION"), "note={note}");
    assert!(note.contains("blocked"), "note={note}");
    assert_eq!(
        value["transition"]["spec_id"], spec_id,
        "spec binding intact"
    );
    let session = f.session_json("sess-p3");
    assert_eq!(session["spec_id"], spec_id, "persisted binding intact");
    let registry = forge::registry::Registry::open(&f.db).unwrap();
    let rows = registry.operations_for_project("sup-partial", 10).unwrap();
    assert!(rows
        .iter()
        .any(|r| r.kind == "agent" && r.state == "partial"));
}

#[test]
fn run_spec_sisyphusfy_malformed_outcome_is_unverified_never_done() {
    let f = Fixture::new("sup-mal-out");
    let spec_id = "spec-mal-1234567890ab";
    write_bound_spec(&f, spec_id);
    write_legacy_session(&f.proj, "sup-mal-out", "sess-mo", "codex", spec_id);
    let doc = outcome_doc(&f, "garbage.txt", "the loop crashed, no document\n");
    let value = f.agent_json(
        &[("SISY_OUTCOME", doc.to_str().unwrap()), ("SISY_EXIT", "1")],
        &[
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-mo",
            "--provider",
            "sisyphusfy",
        ],
    );
    assert_eq!(value["transition"]["verdict"], "unverified");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("raw-outcome: the loop crashed, no document"),
        "evidence={evidence}"
    );
}

#[test]
fn run_spec_sisyphusfy_complete_without_passing_verification_is_unverified() {
    let f = Fixture::new("sup-skipped");
    let spec_id = "spec-skip-1234567890ab";
    write_bound_spec(&f, spec_id);
    write_legacy_session(&f.proj, "sup-skipped", "sess-k", "codex", spec_id);
    let doc = outcome_doc(
        &f,
        "skipped.json",
        r#"{"stop_reason":"complete","iterations":1,"verification":{"status":"skipped","source":"unavailable"}}"#,
    );
    let value = f.agent_json(
        &[("SISY_OUTCOME", doc.to_str().unwrap()), ("SISY_EXIT", "0")],
        &[
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-k",
            "--provider",
            "sisyphusfy",
        ],
    );
    assert_eq!(value["transition"]["verdict"], "unverified");
    assert!(value["transition"]["note"]
        .as_str()
        .unwrap()
        .contains("without a passing verification"));
}

#[test]
fn run_spec_without_supervisor_keeps_bundled_path_and_has_no_verdict() {
    let f = Fixture::new("sup-bundled-run");
    let spec_id = "spec-br-1234567890ab";
    write_bound_spec(&f, spec_id);
    write_legacy_session(&f.proj, "sup-bundled-run", "sess-bu", "codex", spec_id);
    // Same host-dependent contract the archived suite records: with
    // the codex binary present the transition is active, otherwise
    // typed unavailable. Either way no supervisor ran and the
    // bundled envelope carries no `verdict` key.
    let json_out = f.agent_run(
        &[],
        &[
            "--format",
            "json",
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-bu",
        ],
    );
    assert!(f.sisy_log().is_empty(), "supervisor must not run");
    if code_of(&json_out) == 0 {
        let value: serde_json::Value = serde_json::from_slice(&json_out.stdout).expect("json");
        assert!(
            value["transition"].get("verdict").is_none(),
            "bundled envelope: {value}"
        );
    } else {
        let stderr = lossy(&json_out.stderr);
        assert!(
            stderr.contains("error[agent-unavailable]"),
            "stderr={stderr}"
        );
    }
}

#[test]
fn run_spec_absent_supervisor_refuses_and_preserves_the_session() {
    let f = Fixture::new("sup-absent-sisy");
    let spec_id = "spec-abs-1234567890ab";
    write_bound_spec(&f, spec_id);
    write_legacy_session(&f.proj, "sup-absent-sisy", "sess-a", "codex", spec_id);
    let path = f.proj.join(".forge/agents/sess-a/session.json");
    let before = fs::read(&path).unwrap();
    let empty_path = f.scripts.join("empty-bin");
    fs::create_dir_all(&empty_path).unwrap();
    let out = run_env(
        &f.db,
        &[
            ("FORGE_SISYPHUSFY_BIN", "/nonexistent/sisyphusfy"),
            ("PATH", empty_path.to_str().unwrap()),
        ],
        &[
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-a",
            "--provider",
            "sisyphusfy",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[agent-unavailable]"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("attempts:"), "stderr={stderr}");
    assert!(
        stderr.contains("/nonexistent/sisyphusfy"),
        "stderr={stderr}"
    );
    let after = fs::read(&path).unwrap();
    assert_eq!(before, after, "session preserved");
}

#[test]
fn run_spec_ariadex_session_without_supervisor_names_the_real_paths() {
    let f = Fixture::new("sup-ari-runspec");
    let spec_id = "spec-ari-1234567890ab";
    write_bound_spec(&f, spec_id);
    write_legacy_session(&f.proj, "sup-ari-runspec", "sess-ra", "ariadex", spec_id);
    let out = f.agent_run(
        &[],
        &[
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-ra",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[agent-unavailable]"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("scheduler"), "stderr={stderr}");
    assert!(stderr.contains("sisyphusfy"), "stderr={stderr}");
}

#[test]
fn run_spec_unknown_supervisor_is_refused_before_anything_runs() {
    let f = Fixture::new("sup-badsup");
    let spec_id = "spec-bad-1234567890ab";
    write_bound_spec(&f, spec_id);
    write_legacy_session(&f.proj, "sup-badsup", "sess-b", "codex", spec_id);
    let out = f.agent_run(
        &[],
        &[
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-b",
            "--provider",
            "made-up",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[agent-unavailable]"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("sisyphusfy"), "stderr={stderr}");
    assert!(f.sisy_log().is_empty(), "supervisor must not run");
}

#[test]
fn run_spec_without_tasks_file_is_spec_invalid_before_the_supervisor_runs() {
    let f = Fixture::new("sup-notasks");
    let spec_id = "spec-nt-1234567890ab";
    let dir = f.proj.join(".forge/specs").join(spec_id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("manifest.json"), "{\"contract\":\"0.1.0\"}").unwrap();
    write_legacy_session(&f.proj, "sup-notasks", "sess-nt", "codex", spec_id);
    let out = f.agent_run(
        &[],
        &[
            "agent",
            "run-spec",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-nt",
            "--provider",
            "sisyphusfy",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[spec-invalid]"), "stderr={stderr}");
    assert!(stderr.contains("tasks.md"), "stderr={stderr}");
    assert!(f.sisy_log().is_empty(), "supervisor must not run");
}
