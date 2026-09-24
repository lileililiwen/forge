//! Contract suite for `supervised-agent-adapters`: Forge delegates
//! session lifecycle to the real ariadex runtime and spec execution
//! to the sisyphusfy supervisor. Every runtime is faked with a stub
//! binary emitting the documented shapes recorded in
//! `tests/fixtures/supervised/NOTES.md`; verdict/state claims must
//! come from what the stub reported, never from Forge.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_ARIADEX_BIN");
    cmd.env_remove("FORGE_SISYPHUSFY_BIN");
    cmd.env_remove("STUB_DIR");
    cmd.env_remove("STUB_MODE");
    cmd.env_remove("STUB_DAEMON");
    cmd.env_remove("STUB_HANDLE");
    cmd.env_remove("STUB_HANDLE_GONE");
    cmd.env_remove("STUB_INIT");
    cmd.env_remove("STUB_NO_CHANGES");
    cmd.env_remove("STUB_MALFORMED");
    cmd.env_remove("STUB_PAUSE_FAIL");
    cmd.env_remove("STUB_VERSION");
    cmd.env_remove("SISY_STUB_DIR");
    cmd.env_remove("SISY_OUTCOME");
    cmd.env_remove("SISY_EXIT");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn run_env(db: &Path, envs: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, envs: &[(&str, &str)], args: &[&str]) -> serde_json::Value {
    let mut all: Vec<&str> = vec!["--format", "json"];
    all.extend_from_slice(args);
    let out = run_env(db, envs, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn evidence_of(value: &serde_json::Value) -> String {
    value["transition"]["evidence"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|v| v.as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn write_rust_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
}

/// A session file in the pre-change (legacy) shape: no `backing`
/// key at all, exactly what the archived `agent-runtime-workflows`
/// contract wrote.
fn write_legacy_session(
    proj: &Path,
    project_id: &str,
    session_id: &str,
    provider: &str,
    spec_id: &str,
) {
    let path = proj
        .join(".forge/agents")
        .join(session_id)
        .join("session.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        r#"{{"contract":"0.1.0","session_id":"{session_id}","project_id":"{project_id}","project_path":"{}","provider":"{provider}","spec_id":"{spec_id}","state":"active","started_at":"{now}","last_transition_at":"{now}","transitions":[]}}"#,
        proj.display()
    );
    fs::write(&path, body).unwrap();
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/usr/bin/env bash\nset -u\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The ariadex stub: emits the documented shapes recorded in
/// `tests/fixtures/supervised/NOTES.md` and logs every invocation
/// line so tests can prove what Forge did (and never did) call.
/// Scenario knobs arrive through the `STUB_*` environment.
fn write_ariadex_stub(scripts: &Path) -> PathBuf {
    write_script(
        scripts,
        "ariadex-stub.sh",
        r#"
log="${STUB_DIR:?}/log"
state="${STUB_DIR:?}/state"
mkdir -p "$state"
echo "$*" >> "$log"
current_mode() { if [ -f "$state/mode" ]; then cat "$state/mode"; else printf '%s' "${STUB_MODE:-AUTO}"; fi; }
set_mode() { printf '%s' "$1" > "$state/mode"; }
emit_status_json() {
  mode="$(current_mode)"
  handle="${STUB_HANDLE:-cafe1234abcd}"
  daemon="${STUB_DAEMON:-alive}"
  if [ "${STUB_HANDLE_GONE:-0}" = 1 ]; then
    if [ "$daemon" = none ]; then printf '{"mode": "%s"}\n' "$mode"; else printf '{"daemon": {"alive": true, "mode": "%s"}}\n' "$mode"; fi
    return
  fi
  case "$daemon" in
    alive) printf '{"daemon": {"alive": true, "mode": "%s", "session": "%s"}}\n' "$mode" "$handle";;
    stale) printf '{"daemon": {"alive": false, "mode": "%s", "session": "%s"}}\n' "$mode" "$handle";;
    *)     printf '{"mode": "%s", "session": "%s", "agent": "opencode"}\n' "$mode" "$handle";;
  esac
}
case "${1:-}" in
  --version)
    echo "ariadex ${STUB_VERSION:-0.1.0}"
    exit 0;;
  start)
    if [ "${STUB_INIT:-1}" = 0 ]; then
      echo 'error: project is not initialized; run `ariadex init` first (no daemon, tmux, or provider work started)' >&2
      exit 1
    fi
    if [ "${STUB_NO_CHANGES:-0}" = 1 ]; then
      echo 'no active OpenSpec changes; provider not started'
      exit 0
    fi
    echo 'daemon started (pid 4242, endpoint .ariadex/daemon.sock); observing durable state'
    echo 'managed runtime started; daemon owns provider, watcher, and widget'
    exit 0;;
  stop)
    echo 'daemon: stopped (no running daemon)'
    exit 0;;
  pause)
    if [ "${STUB_PAUSE_FAIL:-0}" = 1 ]; then
      echo 'error: pause failed at the safe boundary, token AKIAABCDEFGHIJKLMNOP leaked' >&2
      exit 1
    fi
    set_mode PAUSE
    printf '{\n  "changed": true,\n  "mode": "PAUSE",\n  "note": "scheduling paused; CLI session preserved",\n  "ok": true,\n  "via": "pause"\n}\n'
    echo 'mode: PAUSE (no new scheduling operations; CLI session preserved)'
    exit 0;;
  resume)
    mode="$(current_mode)"
    if [ "$mode" = MANUAL ]; then
      echo 'error: resume rejected from MANUAL: only a PAUSED project may resume (use `takeover` or `auto` from other modes)' >&2
      exit 1
    fi
    set_mode AUTO
    printf '{\n  "changed": true,\n  "mode": "AUTO",\n  "note": "automatic scheduling resumed after reconciliation",\n  "ok": true,\n  "via": "resume"\n}\n'
    exit 0;;
  takeover)
    set_mode MANUAL
    echo 'mode: MANUAL (manual control active; automatic input disabled until `ariadex auto`; CLI session preserved)'
    exit 0;;
  status)
    if [ "${STUB_INIT:-1}" = 0 ]; then
      echo 'missing runtime state at .ariadex/state.json; run `ariadex init` first' >&2
      exit 1
    fi
    if [ "${STUB_MALFORMED:-0}" = 1 ]; then
      echo 'this is not json at all'
      exit 0
    fi
    if [ "${STUB_NO_CHANGES:-0}" = 1 ]; then
      printf '{"mode": "%s", "session": "%s", "agent": "opencode"}\n' "$(current_mode)" "${STUB_HANDLE:-cafe1234abcd}"
      exit 0
    fi
    emit_status_json
    echo 'blocker u-1: fixture blocker'
    exit 0;;
  attach)
    echo 'HIJACK-ATTEMPT' >> "$log"
    echo "tmux attach-session -t ariadex-${STUB_HANDLE:-cafe1234abcd}"
    exit 0;;
  *)
    echo "error: unknown verb $*" >&2
    exit 2;;
esac
"#,
    )
}

/// The sisyphusfy stub: `loop --task-path <file> --json --adapter
/// <name>` prints the outcome document named by SISY_OUTCOME and
/// exits with SISY_EXIT.
fn write_sisyphusfy_stub(scripts: &Path) -> PathBuf {
    write_script(
        scripts,
        "sisyphusfy-stub.sh",
        r#"
echo "$*" >> "${SISY_STUB_DIR:?}/log"
case "${1:-}" in
  loop)
    cat "${SISY_OUTCOME:?}"
    exit "${SISY_EXIT:-0}";;
  *)
    echo "error: unknown $*" >&2
    exit 2;;
esac
"#,
    )
}

struct Fixture {
    _tmp: tempfile::TempDir,
    db: PathBuf,
    proj: PathBuf,
    ariadex_bin: PathBuf,
    sisyphusfy_bin: PathBuf,
    stub_dir: PathBuf,
    sisy_dir: PathBuf,
    scripts: PathBuf,
}

impl Fixture {
    fn new(id: &str) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("proj");
        write_rust_project(&proj, id);
        let scripts = tmp.path().join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        let stub_dir = tmp.path().join("stub-state");
        fs::create_dir_all(&stub_dir).unwrap();
        let sisy_dir = tmp.path().join("sisy-state");
        fs::create_dir_all(&sisy_dir).unwrap();
        Fixture {
            db: tmp.path().join("registry.db"),
            ariadex_bin: write_ariadex_stub(&scripts),
            sisyphusfy_bin: write_sisyphusfy_stub(&scripts),
            stub_dir,
            sisy_dir,
            proj,
            scripts,
            _tmp: tmp,
        }
    }

    fn env_pairs(&self, extra: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut base: Vec<(String, String)> = vec![
            (
                "FORGE_ARIADEX_BIN".to_string(),
                self.ariadex_bin.display().to_string(),
            ),
            ("STUB_DIR".to_string(), self.stub_dir.display().to_string()),
            (
                "FORGE_SISYPHUSFY_BIN".to_string(),
                self.sisyphusfy_bin.display().to_string(),
            ),
            (
                "SISY_STUB_DIR".to_string(),
                self.sisy_dir.display().to_string(),
            ),
        ];
        for (k, v) in extra {
            base.push(((*k).to_string(), (*v).to_string()));
        }
        base
    }

    fn agent_json(&self, extra: &[(&str, &str)], args: &[&str]) -> serde_json::Value {
        let base = self.env_pairs(extra);
        let refs: Vec<(&str, &str)> = base.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        run_json(&self.db, &refs, args)
    }

    fn agent_run(&self, extra: &[(&str, &str)], args: &[&str]) -> std::process::Output {
        let base = self.env_pairs(extra);
        let refs: Vec<(&str, &str)> = base.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        run_env(&self.db, &refs, args)
    }

    fn stub_log(&self) -> String {
        fs::read_to_string(self.stub_dir.join("log")).unwrap_or_default()
    }

    fn sisy_log(&self) -> String {
        fs::read_to_string(self.sisy_dir.join("log")).unwrap_or_default()
    }

    fn set_stub_mode(&self, mode: &str) {
        fs::create_dir_all(self.stub_dir.join("state")).unwrap();
        fs::write(self.stub_dir.join("state/mode"), mode).unwrap();
    }

    fn session_json(&self, session: &str) -> serde_json::Value {
        let body = fs::read_to_string(
            self.proj
                .join(".forge/agents")
                .join(session)
                .join("session.json"),
        )
        .unwrap();
        serde_json::from_str(&body).expect("session json")
    }
}

fn start_args<'a>(proj: &'a Path, session: &'a str, provider: &'a str) -> Vec<&'a str> {
    vec![
        "agent",
        "start",
        proj.to_str().unwrap(),
        "--session",
        session,
        "--provider",
        provider,
    ]
}

fn write_bound_spec(f: &Fixture, spec_id: &str) {
    let dir = f.proj.join(".forge/specs").join(spec_id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("manifest.json"),
        format!("{{\"contract\":\"0.1.0\",\"id\":\"{spec_id}\"}}"),
    )
    .unwrap();
    fs::write(
        dir.join("tasks.md"),
        "## 1. Section\n\n- [ ] implement the thing\n",
    )
    .unwrap();
}

fn outcome_doc(f: &Fixture, name: &str, body: &str) -> PathBuf {
    let path = f.sisy_dir.join(name);
    fs::write(&path, body).unwrap();
    path
}

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

fn code_of(out: &std::process::Output) -> i32 {
    out.status.code().unwrap_or(-1)
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
