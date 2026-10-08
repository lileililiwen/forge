//! Legacy session shape, bundled-path behavior, and the documented help surface.

use super::*;

#[test]
fn help_surface_documents_supervised_provider_and_supervisor() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let start = run_env(&db, &[], &["agent", "start", "--help"]);
    let text = lossy(&start.stdout);
    assert!(text.contains("ariadex"), "start help: {text}");
    let run_spec = run_env(&db, &[], &["agent", "run-spec", "--help"]);
    let text = lossy(&run_spec.stdout);
    assert!(text.contains("sisyphusfy"), "run-spec help: {text}");
}

#[test]
fn bundled_pause_stays_unsupported_and_never_touches_the_runtime() {
    let f = Fixture::new("sup-bundled");
    write_legacy_session(&f.proj, "sup-bundled", "sess-bundled", "codex", "");
    let value = f.agent_json(
        &[],
        &[
            "agent",
            "pause",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-bundled",
        ],
    );
    assert_eq!(value["transition"]["state"], "unsupported");
    let evidence = evidence_of(&value);
    assert!(evidence.contains("pause primitive"), "evidence={evidence}");
    assert!(
        f.stub_log().is_empty(),
        "bundled path must not spawn the runtime: {}",
        f.stub_log()
    );
}

#[test]
fn legacy_session_file_without_backing_still_renders() {
    let f = Fixture::new("sup-legacy");
    write_legacy_session(&f.proj, "sup-legacy", "sess-legacy", "codex", "");
    let value = f.agent_json(
        &[],
        &[
            "agent",
            "status",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-legacy",
        ],
    );
    assert_eq!(value["session_id"], "sess-legacy");
    assert!(value["backing"].is_null(), "backing={}", value["backing"]);
    assert!(value.get("live").is_none(), "bundled has no live block");
    let human = f.agent_run(
        &[],
        &[
            "agent",
            "status",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-legacy",
        ],
    );
    let text = lossy(&human.stdout);
    assert!(text.contains("provider: codex"));
    assert!(!text.contains("backing:"), "human={text}");
}
