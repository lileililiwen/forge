//! Contract suite for `supervised-agent-adapters`: Forge delegates
//! session lifecycle to the real ariadex runtime and spec execution
//! to the sisyphusfy supervisor. Every runtime is faked with a stub
//! binary emitting the documented shapes recorded in
//! `tests/fixtures/supervised/NOTES.md`; verdict/state claims must
//! come from what the stub reported, never from Forge.
//! The target was a single 1,201-line file; it is now a directory of focused
//! submodules, each owning one provider scenario group. The shared helpers below stay
//! reachable to every submodule through `super::`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

pub(crate) fn clean_cmd() -> Command {
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

pub(crate) fn run_env(db: &Path, envs: &[(&str, &str)], args: &[&str]) -> std::process::Output {
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

pub(crate) fn run_json(db: &Path, envs: &[(&str, &str)], args: &[&str]) -> serde_json::Value {
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

pub(crate) fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

pub(crate) fn evidence_of(value: &serde_json::Value) -> String {
    value["transition"]["evidence"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|v| v.as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" | ")
}

pub(crate) fn write_rust_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
}

/// A session file in the pre-change (legacy) shape: no `backing`
/// key at all, exactly what the archived `agent-runtime-workflows`
/// contract wrote.
pub(crate) fn write_legacy_session(
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

pub(crate) fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/usr/bin/env bash\nset -u\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The ariadex stub: emits the documented shapes recorded in
/// `tests/fixtures/supervised/NOTES.md` and logs every invocation
/// line so tests can prove what Forge did (and never did) call.
/// Scenario knobs arrive through the `STUB_*` environment.
pub(crate) fn write_ariadex_stub(scripts: &Path) -> PathBuf {
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
pub(crate) fn write_sisyphusfy_stub(scripts: &Path) -> PathBuf {
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

pub(crate) struct Fixture {
    pub(crate) _tmp: tempfile::TempDir,
    pub(crate) db: PathBuf,
    pub(crate) proj: PathBuf,
    pub(crate) ariadex_bin: PathBuf,
    pub(crate) sisyphusfy_bin: PathBuf,
    pub(crate) stub_dir: PathBuf,
    pub(crate) sisy_dir: PathBuf,
    pub(crate) scripts: PathBuf,
}

impl Fixture {
    pub(crate) fn new(id: &str) -> Self {
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

    pub(crate) fn env_pairs(&self, extra: &[(&str, &str)]) -> Vec<(String, String)> {
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

    pub(crate) fn agent_json(&self, extra: &[(&str, &str)], args: &[&str]) -> serde_json::Value {
        let base = self.env_pairs(extra);
        let refs: Vec<(&str, &str)> = base.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        run_json(&self.db, &refs, args)
    }

    pub(crate) fn agent_run(&self, extra: &[(&str, &str)], args: &[&str]) -> std::process::Output {
        let base = self.env_pairs(extra);
        let refs: Vec<(&str, &str)> = base.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        run_env(&self.db, &refs, args)
    }

    pub(crate) fn stub_log(&self) -> String {
        fs::read_to_string(self.stub_dir.join("log")).unwrap_or_default()
    }

    pub(crate) fn sisy_log(&self) -> String {
        fs::read_to_string(self.sisy_dir.join("log")).unwrap_or_default()
    }

    pub(crate) fn set_stub_mode(&self, mode: &str) {
        fs::create_dir_all(self.stub_dir.join("state")).unwrap();
        fs::write(self.stub_dir.join("state/mode"), mode).unwrap();
    }

    pub(crate) fn session_json(&self, session: &str) -> serde_json::Value {
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

pub(crate) fn start_args<'a>(proj: &'a Path, session: &'a str, provider: &'a str) -> Vec<&'a str> {
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

pub(crate) fn write_bound_spec(f: &Fixture, spec_id: &str) {
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

pub(crate) fn outcome_doc(f: &Fixture, name: &str, body: &str) -> PathBuf {
    let path = f.sisy_dir.join(name);
    fs::write(&path, body).unwrap();
    path
}

pub(crate) fn code_of(out: &std::process::Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

mod ariadex;
mod legacy_session;
mod native_toolchain;
mod sisyphusfy;
