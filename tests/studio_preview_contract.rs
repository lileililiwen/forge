//! Preview lifecycle contract tests for the Site Studio
//! (`site-studio-preview-refinement`).
//!
//! These drive the shared Core functions (`studio::start_preview` /
//! `studio::stop_preview`) that both the API host and the CLI probe
//! call, using the in-process [`FakeRunner`] so the bounded lifecycle
//! is proven without a Node toolchain. The API/CLI transports are thin
//! adapters over exactly these functions.

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use forge::registry::Registry;
use forge::studio::state::SessionPreviewState;
use forge::studio::{
    load_session, parse_spec_text, save_spec, start_preview, stop_preview, FakeRunner,
    PreviewState, ProcessRunner, PORT_RANGE_WIDTH, RUNTIME_BIN_ENV, STUDIO_PORT_RANGE_START_ENV,
};

/// Serialize the tests: the Studio port range and the startup-timeout
/// env are process-global, so parallel tests would race the allocator.
static SERIAL: Mutex<()> = Mutex::new(());

const PROJECT_ID: &str = "preview-contract";

const APP_SPEC: &str = "schema_version: \"1\"\n\
project_id: preview-contract\n\
name: Preview Contract\n\
profile: react-web\n\
pages:\n\
\x20\x20- route: /\n\
\x20\x20\x20\x20title: Home\n\
\x20\x20\x20\x20sections:\n\
\x20\x20\x20\x20\x20\x20- id: hero-block\n\
\x20\x20\x20\x20\x20\x20\x20\x20kind: hero\n";

/// A distinct port range for this test binary so it cannot collide
/// with the CLI/API test processes running in parallel.
const TEST_PORT_BASE: u16 = 45800;

fn set_test_env() {
    std::env::set_var(STUDIO_PORT_RANGE_START_ENV, TEST_PORT_BASE.to_string());
    std::env::set_var("FORGE_STUDIO_STARTUP_TIMEOUT_SECS", "8");
}

fn setup(tmp: &Path) -> (Registry, PathBuf) {
    let project_root = tmp.join("project");
    fs::create_dir_all(&project_root).unwrap();
    fs::write(
        project_root.join("forge.yaml"),
        "schema: 1\nproject:\n  id: preview-contract\n  name: Preview Contract\n  profile: react-web\n  maturity: L0\n  target_maturity: L1\n",
    )
    .unwrap();
    let mut registry = Registry::open(&tmp.join("registry.db")).unwrap();
    registry.register(&project_root, None).unwrap();
    (registry, project_root)
}

fn save_minimal_spec(registry: &Registry, project_root: &Path) -> PathBuf {
    let path = project_root.join("forge.app.yaml");
    fs::write(&path, APP_SPEC).unwrap();
    let spec = parse_spec_text(APP_SPEC).unwrap();
    save_spec(registry, PROJECT_ID, project_root, spec, "r0").unwrap();
    path
}

fn journal_kinds(registry: &Registry) -> Vec<(String, String)> {
    registry
        .operations_for_project(PROJECT_ID, 50)
        .unwrap()
        .into_iter()
        .map(|entry| (entry.kind, entry.state))
        .collect()
}

#[test]
fn preview_reaches_ready_records_start_and_stop() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());
    save_minimal_spec(&registry, &project_root);

    let runner = Box::new(FakeRunner::default());
    let (session, live) = start_preview(&registry, &project_root, runner).unwrap();
    assert_eq!(session.preview.state, PreviewState::Ready);
    let port = session.preview.port.expect("reserved port");
    assert!((TEST_PORT_BASE..TEST_PORT_BASE + PORT_RANGE_WIDTH).contains(&port));

    // The status read reflects the persisted ready state.
    let loaded = load_session(&project_root).unwrap().unwrap();
    assert_eq!(loaded.preview.state, PreviewState::Ready);

    // A start journal row is recorded with the bounded state.
    let kinds = journal_kinds(&registry);
    assert!(
        kinds
            .iter()
            .any(|(kind, state)| kind == "studio.preview.start" && state == "ready"),
        "missing studio.preview.start row: {kinds:?}"
    );

    // Stop with the live handle: kills the child and records stopped.
    let stopped = stop_preview(&registry, &project_root, Some(live)).unwrap();
    assert_eq!(stopped.preview.state, PreviewState::Stopped);
    let loaded = load_session(&project_root).unwrap().unwrap();
    assert_eq!(loaded.preview.state, PreviewState::Stopped);

    let kinds = journal_kinds(&registry);
    let stop_rows = kinds
        .iter()
        .filter(|(kind, _)| kind == "studio.preview.stop")
        .count();
    assert_eq!(stop_rows, 1, "exactly one stop row: {kinds:?}");
}

#[test]
fn stop_is_idempotent_and_records_one_row_per_request() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());
    save_minimal_spec(&registry, &project_root);

    // Stop with no live session, twice.
    let first = stop_preview(&registry, &project_root, None).unwrap();
    assert_eq!(first.preview.state, PreviewState::Stopped);
    let second = stop_preview(&registry, &project_root, None).unwrap();
    assert_eq!(second.preview.state, PreviewState::Stopped);

    let stop_rows = journal_kinds(&registry)
        .iter()
        .filter(|(kind, _)| kind == "studio.preview.stop")
        .count();
    assert_eq!(stop_rows, 2, "one stop row per request");
}

#[test]
fn preview_startup_failure_persists_failed_with_typed_code() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());
    save_minimal_spec(&registry, &project_root);

    // The fake runner exits before binding the reserved port.
    let runner = Box::new(FakeRunner {
        exit_code: Some(1),
        bind_delay: Duration::from_millis(0),
        bind_listener: false,
        ..FakeRunner::default()
    });
    let err = start_preview(&registry, &project_root, runner)
        .map(|_| ())
        .unwrap_err();
    assert_eq!(err.code(), "studio-start-timeout");

    let loaded = load_session(&project_root).unwrap().unwrap();
    assert_eq!(loaded.preview.state, PreviewState::Failed);
    assert_eq!(
        loaded.preview.last_error_code.as_deref(),
        Some("studio-start-timeout")
    );
    // A later status must not claim a stale ready.
    assert_ne!(loaded.preview.state, PreviewState::Ready);
}

#[test]
fn preview_port_collision_is_refused_without_killing_a_listener() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());
    save_minimal_spec(&registry, &project_root);

    // Occupy the whole range so the allocator refuses.
    let mut listeners = Vec::new();
    for offset in 0..PORT_RANGE_WIDTH {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", TEST_PORT_BASE + offset)) {
            listeners.push(listener);
        }
    }
    assert_eq!(listeners.len(), PORT_RANGE_WIDTH as usize);

    let runner = Box::new(FakeRunner::default());
    let err = start_preview(&registry, &project_root, runner)
        .map(|_| ())
        .unwrap_err();
    assert_eq!(err.code(), "studio-port-unavailable");

    // The listeners the test owns are all still alive: the allocator
    // never killed an unrelated process.
    assert_eq!(listeners.len(), PORT_RANGE_WIDTH as usize);
    drop(listeners);

    let loaded = load_session(&project_root).unwrap().unwrap();
    assert_eq!(loaded.preview.state, PreviewState::Failed);
    assert_eq!(
        loaded.preview.last_error_code.as_deref(),
        Some("studio-port-unavailable")
    );
}

#[test]
fn preview_start_requires_a_saved_session() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());
    // No save: the preview must refuse before touching a process.
    let runner = Box::new(FakeRunner::default());
    let err = start_preview(&registry, &project_root, runner)
        .map(|_| ())
        .unwrap_err();
    assert_eq!(err.code(), "studio-invalid-spec");
    assert!(load_session(&project_root).unwrap().is_none());
}

#[test]
fn persisted_failed_record_survives_reload() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());
    save_minimal_spec(&registry, &project_root);
    let runner = Box::new(FakeRunner {
        exit_code: Some(2),
        bind_delay: Duration::from_millis(0),
        bind_listener: false,
        ..FakeRunner::default()
    });
    let _ = start_preview(&registry, &project_root, runner)
        .map(|_| ())
        .unwrap_err();
    let loaded = load_session(&project_root).unwrap().unwrap();
    let expected: SessionPreviewState = loaded.preview.clone();
    assert_eq!(expected.state, PreviewState::Failed);
    assert_eq!(loaded.spec_revision, "r1");
}

#[cfg(unix)]
fn has_python3() -> bool {
    std::process::Command::new("/usr/bin/env")
        .args(["python3", "-c", "pass"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// A bounded "dev server" that spawns a descendant binding
/// `FORGE_STUDIO_PORT + 1`. Killing only the direct child would leave
/// that descendant holding its port, so the process-tree kill is
/// observable from the test.
#[cfg(unix)]
fn stub_runner_with_descendant(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("stub-runner-descendant.py");
    let script = "#!/usr/bin/env python3\n\
import os, socket, subprocess, sys, time\n\
port = int(os.environ[\"FORGE_STUDIO_PORT\"])\n\
gp = port + 1\n\
child_code = \"import socket,time;s=socket.socket();s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1);s.bind(('127.0.0.1',%d));s.listen(1);time.sleep(600)\" % gp\n\
subprocess.Popen([sys.executable, \"-c\", child_code])\n\
for _ in range(200):\n\
\x20\x20\x20\x20try:\n\
\x20\x20\x20\x20\x20\x20\x20\x20c = socket.create_connection((\"127.0.0.1\", gp), 0.05)\n\
\x20\x20\x20\x20\x20\x20\x20\x20c.close()\n\
\x20\x20\x20\x20\x20\x20\x20\x20break\n\
\x20\x20\x20\x20except OSError:\n\
\x20\x20\x20\x20\x20\x20\x20\x20time.sleep(0.05)\n\
s = socket.socket()\n\
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)\n\
s.bind((\"127.0.0.1\", port))\n\
s.listen(5)\n\
print(\"stub ready\", flush=True)\n\
time.sleep(600)\n";
    fs::write(&path, script).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
fn wait_port_busy(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpListener::bind(("127.0.0.1", port)).is_err() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

#[cfg(unix)]
fn wait_port_free(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

/// The real runner is `npm` → `sh` → the dev server. Stopping must kill
/// the whole tree, not just the direct child, or the descendant keeps
/// the reserved port bound. This uses a python stub whose descendant
/// binds the adjacent port so the leak is directly observable.
#[cfg(unix)]
#[test]
fn preview_stop_kills_the_whole_process_tree() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    if !has_python3() {
        eprintln!("skip: python3 is not available to host the runner stub");
        return;
    }
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());
    save_minimal_spec(&registry, &project_root);

    let stub = stub_runner_with_descendant(tmp.path());
    std::env::set_var(RUNTIME_BIN_ENV, &stub);
    let runner = Box::new(ProcessRunner::react_web());
    std::env::remove_var(RUNTIME_BIN_ENV);

    let (session, live) = start_preview(&registry, &project_root, runner).unwrap();
    assert_eq!(session.preview.state, PreviewState::Ready);
    let port = session.preview.port.expect("reserved port");

    // Precondition: the descendant is bound to the adjacent port.
    assert!(
        wait_port_busy(port + 1, Duration::from_secs(2)),
        "descendant should hold port {}",
        port + 1
    );

    stop_preview(&registry, &project_root, Some(live)).unwrap();

    // After stop both the reserved port and the descendant's port are
    // immediately bindable: the whole tree was killed.
    assert!(
        wait_port_free(port, Duration::from_secs(2)),
        "reserved port {port} not released after stop"
    );
    assert!(
        wait_port_free(port + 1, Duration::from_secs(2)),
        "descendant on port {} survived the stop",
        port + 1
    );
}
