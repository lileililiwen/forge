//! Ariadex session lifecycle: start, pause/resume, takeover, restart, new-session, and live-status mapping.

use super::*;

#[test]
fn ariadex_start_delegates_and_records_backing_runtime_handle_version() {
    let f = Fixture::new("sup-start");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-1", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    assert_eq!(value["transition"]["provider"], "ariadex");
    let log = f.stub_log();
    assert!(log.contains("--version"), "log={log}");
    assert!(log.contains("start\n"), "log={log}");
    assert!(log.contains("status --json"), "log={log}");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("verb: ariadex start"),
        "evidence={evidence}"
    );
    assert!(
        evidence.contains("handle: cafe1234abcd"),
        "evidence={evidence}"
    );
    assert!(evidence.contains("version: 0.1.0"), "evidence={evidence}");
    let session = f.session_json("sess-1");
    assert_eq!(session["backing"]["runtime"], "ariadex");
    assert_eq!(session["backing"]["handle"], "cafe1234abcd");
    assert_eq!(session["backing"]["adapter_version"], "0.1.0");
    assert_eq!(session["contract"], "0.1.0");
    assert_eq!(session["provider"], "ariadex");
    let registry = forge::registry::Registry::open(&f.db).unwrap();
    let rows = registry.operations_for_project("sup-start", 10).unwrap();
    let row = rows
        .iter()
        .find(|r| r.kind == "agent" && r.detail.as_deref().unwrap_or_default().contains("backing"))
        .expect("journal names the backing runtime");
    assert!(row.detail.as_ref().unwrap().contains("cafe1234abcd"));
    assert_eq!(row.state, "done");
}

#[test]
fn ariadex_start_uninitialized_records_disconnected_never_active() {
    let f = Fixture::new("sup-uninit");
    let value = f.agent_json(
        &[("STUB_INIT", "0")],
        &start_args(&f.proj, "sess-u", "ariadex"),
    );
    assert_eq!(value["transition"]["state"], "disconnected");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("project is not initialized"),
        "evidence={evidence}"
    );
    let note = value["transition"]["note"].as_str().unwrap_or_default();
    assert!(note.contains("no live state"), "note={note}");
    // The runtime's own refusal is attributed verbatim (bounded).
    assert!(evidence.contains("ariadex init"), "evidence={evidence}");
}

#[test]
fn pause_and_resume_delegate_to_the_real_primitives() {
    let f = Fixture::new("sup-pause");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-p", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    let value = f.agent_json(
        &[],
        &[
            "agent",
            "pause",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-p",
        ],
    );
    assert_eq!(value["transition"]["state"], "paused");
    let log = f.stub_log();
    assert!(log.contains("pause\n"), "log={log}");
    let value = f.agent_json(
        &[],
        &[
            "agent",
            "resume",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-p",
        ],
    );
    assert_eq!(value["transition"]["state"], "active");
    let session = f.session_json("sess-p");
    let kinds: Vec<&str> = session["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["kind"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(kinds, vec!["start", "pause", "resume"]);
    let log = f.stub_log();
    assert!(log.contains("resume\n"), "log={log}");
}

#[test]
fn resume_refused_from_manual_records_the_runtimes_truth_not_active() {
    let f = Fixture::new("sup-manual");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-rf", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    // The operator drove the session into MANUAL through the runtime
    // itself (Forge's takeover never mutates runtime state).
    f.set_stub_mode("MANUAL");
    let value = f.agent_json(
        &[],
        &[
            "agent",
            "resume",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-rf",
        ],
    );
    assert_eq!(value["transition"]["state"], "disconnected");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("resume rejected from MANUAL"),
        "evidence={evidence}"
    );
    let note = value["transition"]["note"].as_str().unwrap_or_default();
    assert!(note.contains("not a simulated success"), "note={note}");
    let session = f.session_json("sess-rf");
    let transitions = session["transitions"].as_array().unwrap();
    assert_eq!(transitions.len(), 2, "prior transitions preserved");
    assert_eq!(transitions[0]["kind"], "start");
}

#[test]
fn takeover_is_attach_guidance_and_never_hijacks_the_terminal() {
    let f = Fixture::new("sup-takeover");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-t", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    let value = f.agent_json(
        &[],
        &[
            "agent",
            "takeover",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-t",
        ],
    );
    assert_eq!(value["transition"]["state"], "active");
    let next = value["transition"]["next_step"]
        .as_str()
        .unwrap_or_default();
    assert!(next.contains("ariadex attach"), "next_step={next}");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("takeover: guidance only"),
        "evidence={evidence}"
    );
    let log = f.stub_log();
    assert!(!log.contains("attach"), "stub log={log}");
    assert!(!log.contains("HIJACK"), "stub log={log}");
    assert!(!log.contains("takeover"), "stub log={log}");
}

#[test]
fn restart_stops_then_starts_in_sibling_order() {
    let f = Fixture::new("sup-restart");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-r", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    let before = f.stub_log().lines().count();
    let value = f.agent_json(
        &[],
        &[
            "agent",
            "restart",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-r",
        ],
    );
    assert_eq!(value["transition"]["state"], "active");
    let log = f.stub_log();
    let new_lines: Vec<&str> = log.lines().skip(before).collect();
    let stop_at = new_lines.iter().position(|l| *l == "stop").expect("stop");
    let start_at = new_lines.iter().position(|l| *l == "start").expect("start");
    assert!(stop_at < start_at, "log lines={new_lines:?}");
}

#[test]
fn stale_daemon_maps_to_disconnected_in_transitions() {
    let f = Fixture::new("sup-stale");
    let value = f.agent_json(
        &[("STUB_DAEMON", "stale")],
        &start_args(&f.proj, "sess-st", "ariadex"),
    );
    assert_eq!(value["transition"]["state"], "disconnected");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("ariadex-daemon: stale"),
        "evidence={evidence}"
    );
    // The backing pointer still records where the truth lives.
    let session = f.session_json("sess-st");
    assert_eq!(session["backing"]["runtime"], "ariadex");
}

#[test]
fn unknown_stored_handle_surfaces_loss_without_mutating_the_record() {
    let f = Fixture::new("sup-lost");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-l", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    let record_before =
        fs::read_to_string(f.proj.join(".forge/agents/sess-l/session.json")).unwrap();
    let value = f.agent_json(
        &[("STUB_HANDLE_GONE", "1")],
        &[
            "agent",
            "status",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-l",
        ],
    );
    assert_eq!(value["live"]["state"], "disconnected");
    assert!(value["live"]["note"]
        .as_str()
        .unwrap()
        .contains("stored handle `cafe1234abcd`"));
    assert_eq!(value["state"], "active", "recorded state untouched");
    let record_after =
        fs::read_to_string(f.proj.join(".forge/agents/sess-l/session.json")).unwrap();
    assert_eq!(record_before, record_after, "session file preserved");
}

#[test]
fn local_no_daemon_status_surfaces_unreachable_state_via_live_probe() {
    let f = Fixture::new("sup-nodaemon");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-nd", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    let value = f.agent_json(
        &[("STUB_DAEMON", "none")],
        &[
            "agent",
            "status",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-nd",
        ],
    );
    assert_eq!(value["live"]["state"], "disconnected");
    assert_eq!(value["live"]["daemon"], "absent");
    assert_eq!(value["live"]["mode"], "AUTO", "raw mode preserved");
}

#[test]
fn malformed_status_document_maps_to_disconnected() {
    let f = Fixture::new("sup-malformed");
    let value = f.agent_json(
        &[("STUB_MALFORMED", "1")],
        &start_args(&f.proj, "sess-m", "ariadex"),
    );
    assert_eq!(value["transition"]["state"], "disconnected");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("no parsable status document"),
        "evidence={evidence}"
    );
}

#[test]
fn credential_shaped_runtime_output_is_redacted_in_evidence() {
    let f = Fixture::new("sup-redact");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-c", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    let value = f.agent_json(
        &[("STUB_PAUSE_FAIL", "1")],
        &[
            "agent",
            "pause",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-c",
        ],
    );
    let rendered = serde_json::to_string(&value).unwrap();
    assert!(
        !rendered.contains("AKIAABCDEFGHIJKLMNOP"),
        "credential escaped: {rendered}"
    );
    assert!(
        rendered.contains("pause failed"),
        "evidence lost: {rendered}"
    );
    let file = fs::read_to_string(f.proj.join(".forge/agents/sess-c/session.json")).unwrap();
    assert!(
        !file.contains("AKIAABCDEFGHIJKLMNOP"),
        "persisted credential"
    );
}

#[test]
fn new_session_supersedes_through_the_sibling_stop_then_start_verbs() {
    let f = Fixture::new("sup-newsess");
    let value = f.agent_json(&[], &start_args(&f.proj, "sess-n", "ariadex"));
    assert_eq!(value["transition"]["state"], "active");
    let value = f.agent_json(
        &[],
        &[
            "agent",
            "new-session",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-n",
            "--new-session",
            "sess-n2",
        ],
    );
    assert_eq!(value["transition"]["session_id"], "sess-n2");
    assert_eq!(value["transition"]["state"], "active");
    assert_eq!(value["transition"]["provider"], "ariadex");
    let log = f.stub_log();
    assert!(log.contains("stop\n"), "log={log}");
    let session = f.session_json("sess-n2");
    assert_eq!(session["backing"]["runtime"], "ariadex");
    assert_eq!(session["backing"]["handle"], "cafe1234abcd");
    // The prior session keeps its own record untouched.
    let prior = f.session_json("sess-n");
    let prior_kinds: Vec<&str> = prior["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["kind"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(prior_kinds, vec!["start"]);
}
