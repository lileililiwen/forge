//! Native toolchain resolution: runtime discovery, version probe, provider routing, and env precedence.

use super::*;

#[test]
fn runtime_absent_lists_resolution_attempts_and_bundled_still_works() {
    let f = Fixture::new("sup-absent");
    let empty_path = f.scripts.join("empty-bin");
    fs::create_dir_all(&empty_path).unwrap();
    let out = run_env(
        &f.db,
        &[("PATH", empty_path.to_str().unwrap())],
        &[
            "agent",
            "start",
            f.proj.to_str().unwrap(),
            "--session",
            "sess-x",
            "--provider",
            "ariadex",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[agent-unavailable]"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("attempts:"), "stderr={stderr}");
    assert!(stderr.contains("not found"), "stderr={stderr}");
    assert!(lossy(&out.stdout).is_empty(), "stdout must stay empty");
    assert!(
        stderr.contains("opencode"),
        "bundled guidance missing: {stderr}"
    );
    // The bundled vocabulary is untouched even with an empty PATH:
    // the failure names only `codex`, never the supervised runtime.
    let bundled = run_env(
        &f.db,
        &[("PATH", empty_path.to_str().unwrap())],
        &start_args(&f.proj, "sess-b2", "codex"),
    );
    let bundled_stderr = lossy(&bundled.stderr);
    if let Some(code) = bundled.status.code() {
        assert!(code == 0 || code == 1, "unexpected exit {code}");
    }
    assert!(
        !bundled_stderr.contains("ariadex"),
        "bundled failure must not mention ariadex: {bundled_stderr}"
    );
    if code_of(&bundled) == 1 {
        assert!(bundled_stderr.contains("codex"), "stderr={bundled_stderr}");
    }
}

#[test]
fn version_probe_mismatch_refuses_best_effort_parsing() {
    let f = Fixture::new("sup-verprobe");
    let value = f.agent_json(
        &[("STUB_VERSION", "dev-build")],
        &start_args(&f.proj, "sess-v", "ariadex"),
    );
    assert_eq!(value["transition"]["state"], "unsupported");
    let log = f.stub_log();
    assert!(log.contains("--version"), "log={log}");
    assert!(!log.contains("start\n"), "no verb delegated: log={log}");
    let evidence = evidence_of(&value);
    assert!(evidence.contains("version-probe"), "evidence={evidence}");
    assert!(
        evidence.contains("ariadex dev-build"),
        "evidence={evidence}"
    );
}

#[test]
fn sisyphusfy_as_session_provider_is_refused_with_the_real_path() {
    let f = Fixture::new("sup-sisy-provider");
    let out = f.agent_run(&[], &start_args(&f.proj, "sess-sp", "sisyphusfy"));
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("spec-execution supervisor"),
        "stderr={stderr}"
    );
    assert!(!f.stub_dir.join("log").exists());
}

#[test]
fn env_override_wins_over_path_for_the_runtime_binary() {
    let f = Fixture::new("sup-envprec");
    // Put a second, handle-distinct stub on PATH; the env override
    // (the fixture stub with handle cafe1234abcd) must still win.
    // /usr/bin stays on PATH so the stub shebangs still resolve.
    let alt = write_script(&f.scripts, "ariadex", "echo 'ariadex 9.9.9'; exit 0");
    let _ = alt;
    let path_value = format!("{}:/usr/bin:/bin", f.scripts.display());
    let value = f.agent_json(
        &[("PATH", path_value.as_str())],
        &start_args(&f.proj, "sess-ep", "ariadex"),
    );
    assert_eq!(value["transition"]["state"], "active");
    let session = f.session_json("sess-ep");
    assert_eq!(session["backing"]["handle"], "cafe1234abcd");
    assert_eq!(session["backing"]["adapter_version"], "0.1.0");
}

#[test]
fn no_changes_started_maps_to_disconnected_because_no_daemon_runs() {
    let f = Fixture::new("sup-nochanges");
    let value = f.agent_json(
        &[("STUB_NO_CHANGES", "1")],
        &start_args(&f.proj, "sess-nc", "ariadex"),
    );
    // The verb exited 0 but the status surface shows no daemon;
    // Forge must not claim an active session for a runtime that
    // deliberately did not start the provider.
    assert_eq!(value["transition"]["state"], "disconnected");
    let evidence = evidence_of(&value);
    assert!(
        evidence.contains("handle: cafe1234abcd"),
        "evidence={evidence}"
    );
}
