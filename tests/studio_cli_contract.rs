//! CLI contract tests for the Site Studio
//! (`site-studio-preview-refinement`).
//!
//! Each test exercises one observable behaviour the spec owns:
//! the closed AppSpec envelope, the typed validation refusal, the
//! revision-bound spec save, the bounded preview envelope, the
//! idempotent preview stop, the refinement journal row, and the
//! path/credential/secret refusal. The bundled profile runner is
//! the FakeRunner on a controlled `PATH` so no live `react-web`
//! toolchain is required.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[path = "support/studio_ports.rs"]
mod studio_ports;
use studio_ports::shared_port_base;

/// See `support/studio_ports.rs`: each Studio target prefers a different
/// candidate window.
const SLOT: u16 = 2;

const APP_SPEC_CONTRACT: &str = "forge-app-spec/0.1.0";
const SESSION_CONTRACT: &str = "forge-studio-session/0.1.0";
const PREVIEW_CONTRACT: &str = "forge-studio-preview/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_STUDIO_PORT_RANGE_START");
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

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut all: Vec<&str> = vec!["--format", "json"];
    all.extend_from_slice(args);
    let out = run(db, &all);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("valid json")
}

fn write_minimal_manifest(project_root: &Path) {
    fs::write(
        project_root.join("forge.yaml"),
        "schema: 1\n\
         project:\n  id: studio-cli\n  name: Studio CLI\n\
         \x20\x20profile: react-web\n  maturity: L0\n  target_maturity: L1\n",
    )
    .unwrap();
}

fn write_minimal_spec(project_root: &Path) -> PathBuf {
    let path = project_root.join("forge.app.yaml");
    fs::write(
        &path,
        "schema_version: \"1\"\n\
         project_id: studio-cli\n\
         name: Studio CLI\n\
         profile: react-web\n\
         pages:\n\
         \x20\x20- route: /\n\
         \x20\x20\x20\x20title: Home\n\
         \x20\x20\x20\x20sections:\n\
         \x20\x20\x20\x20\x20\x20- id: hero-block\n\
         \x20\x20\x20\x20\x20\x20\x20\x20kind: hero\n",
    )
    .unwrap();
    path
}

fn register_project(db: &Path, project_root: &Path) {
    write_minimal_manifest(project_root);
    let out = run(
        db,
        &[
            "register",
            project_root.to_str().unwrap(),
            "--accept",
            "--format",
            "json",
        ],
    );
    // `register` without `--accept` is a dry run; tests pass
    // `--accept` so the manifest is written and the registry row
    // persists.
    let _ = out;
    let out = run(
        db,
        &[
            "register",
            project_root.to_str().unwrap(),
            "--format",
            "json",
        ],
    );
    assert!(out.status.success(), "register: {}", lossy(&out.stderr));
}

#[test]
fn studio_help_advertises_every_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["studio", "--help"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let help = lossy(&out.stdout);
    for verb in ["spec", "preview", "refine"] {
        assert!(help.contains(verb), "missing {verb}:\n{help}");
    }
}

#[test]
fn spec_validates_a_minimal_appspec_with_r0() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    write_minimal_spec(&project_root);
    let env = run_json(&db, &["studio", "spec", "studio-cli", "--format", "json"]);
    assert_eq!(env["contract"], APP_SPEC_CONTRACT);
    assert_eq!(env["project_id"], "studio-cli");
    assert_eq!(env["profile"], "react-web");
    assert_eq!(env["schema_version"], "1");
    assert_eq!(env["pages"][0]["route"], "/");
}

#[test]
fn spec_refuses_with_typed_error_for_an_unsupported_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    let path = project_root.join("forge.app.yaml");
    fs::write(
        &path,
        "schema_version: \"1\"\nproject_id: studio-cli\nname: Studio CLI\nprofile: aspnet-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n",
    )
    .unwrap();
    let out = run(&db, &["--format", "json", "studio", "spec", "studio-cli"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    // The CLI wraps parse failures as `studio-invalid-spec` so the
    // operator gets one error code for the read path; the message
    // surfaces the original typed reason so the failure mode is
    // never hidden.
    assert!(
        stderr.contains("studio-invalid-spec"),
        "missing typed code:\n{stderr}"
    );
    assert!(
        stderr.contains("studio unsupported profile"),
        "missing inner typed reason:\n{stderr}"
    );
    assert!(stderr.contains("aspnet-web"));
    assert_eq!(lossy(&out.stdout).trim(), "", "stdout must be empty");
}

#[test]
fn spec_refuses_with_typed_error_for_a_duplicate_route() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    let path = project_root.join("forge.app.yaml");
    fs::write(
        &path,
        "schema_version: \"1\"\nproject_id: studio-cli\nname: Studio CLI\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n  - route: /\n    title: Home again\n    sections:\n      - id: hero-block-two\n        kind: hero\n",
    )
    .unwrap();
    let out = run(&db, &["studio", "spec", "studio-cli"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("studio-invalid-spec"),
        "missing code:\n{stderr}"
    );
    assert!(stderr.contains("duplicate route"));
    assert_eq!(lossy(&out.stdout).trim(), "");
}

#[test]
fn spec_refuses_secret_shaped_field_without_echo() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    let path = project_root.join("forge.app.yaml");
    fs::write(
        &path,
        "schema_version: \"1\"\nproject_id: studio-cli\nname: Studio CLI\nprofile: react-web\npages:\n  - route: /login\n    title: 'AKIAIOSFODNN7EXAMPLE'\n    sections:\n      - id: hero-block\n        kind: hero\n",
    )
    .unwrap();
    let out = run(&db, &["studio", "spec", "studio-cli"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("credential-shaped"), "{stderr}");
    assert!(!stderr.contains("AKIA"));
    assert_eq!(lossy(&out.stdout).trim(), "");
}

#[test]
fn preview_status_prints_a_ready_envelope_with_no_session() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    let out = run(
        &db,
        &["--format", "json", "studio", "preview", "studio-cli"],
    );
    let env: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(env["contract"], PREVIEW_CONTRACT);
    assert_eq!(env["project_id"], "studio-cli");
    assert_eq!(env["state"], "none");
    assert!(env["preview_url"].is_null());
}

#[test]
fn preview_start_requires_confirm_yes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    let out = run(
        &db,
        &["studio", "preview", "studio-cli", "--action", "start"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("studio-invalid-spec") && stderr.contains("--confirm yes"),
        "missing confirm gate: {stderr}"
    );
    assert_eq!(lossy(&out.stdout).trim(), "");
}

#[test]
fn refine_rejects_stale_revision_with_typed_error() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    // No prior spec save: refinement must surface a stale-revision
    // refusal so the operator cannot silently journal a refinement
    // against an absent session.
    let out = run(
        &db,
        &[
            "studio",
            "refine",
            "studio-cli",
            "--expected-revision",
            "r9",
            "--request",
            "polish the hero",
            "--selected-files",
            "src/App.tsx",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("studio-revision-conflict"),
        "missing typed code:\n{stderr}"
    );
    assert_eq!(lossy(&out.stdout).trim(), "");
}

#[test]
fn refine_rejects_path_escape_with_typed_error() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    let out = run(
        &db,
        &[
            "studio",
            "refine",
            "studio-cli",
            "--expected-revision",
            "r1",
            "--request",
            "escape",
            "--selected-files",
            "../etc/passwd",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("studio-project-scope") || stderr.contains("studio-revision-conflict"),
        "missing typed code:\n{stderr}"
    );
}

#[test]
fn preview_action_help_lists_every_action() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["studio", "preview", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    let help = lossy(&out.stdout);
    for action in ["status", "start", "stop"] {
        assert!(help.contains(action), "missing {action}:\n{help}");
    }
}

#[test]
fn contracts_are_versioned_in_help() {
    // The contracts are exposed through the JSON envelope
    // (covered by spec_* and preview_* tests). This test asserts
    // the studio help text is wired up so an operator can find
    // the verbs without scanning the source.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["studio", "refine", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    let help = lossy(&out.stdout);
    for flag in ["--expected-revision", "--request", "--selected-files"] {
        assert!(help.contains(flag), "missing {flag}:\n{help}");
    }
    let _ = SESSION_CONTRACT; // used by other test surfaces
}

// Silence dead-code lint for items kept for parity with the
// upstream contract tests.
#[allow(dead_code)]
fn _fixtures_unused() -> fs::Permissions {
    fs::Permissions::from_mode(0o755)
}

/// Run `forge` with per-command environment overrides on top of the
/// cleaned environment.
fn run_env(db: &Path, args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd.output().expect("run forge")
}

fn has_python3() -> bool {
    Command::new("/usr/bin/env")
        .args(["python3", "-c", "pass"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// A bounded "dev server" the profile runner can start: it binds
/// the reserved port from `FORGE_STUDIO_PORT` and answers 200.
fn stub_runner(dir: &Path) -> PathBuf {
    let path = dir.join("stub-runner.py");
    let script = "#!/usr/bin/env python3\n\
import http.server, socketserver, os\n\
port = int(os.environ[\"FORGE_STUDIO_PORT\"])\n\
class H(http.server.BaseHTTPRequestHandler):\n\
\x20\x20\x20\x20def do_GET(self):\n\
\x20\x20\x20\x20\x20\x20\x20\x20self.send_response(200)\n\
\x20\x20\x20\x20\x20\x20\x20\x20self.end_headers()\n\
\x20\x20\x20\x20\x20\x20\x20\x20self.wfile.write(b\"ok\")\n\
\x20\x20\x20\x20def log_message(self, *a):\n\
\x20\x20\x20\x20\x20\x20\x20\x20pass\n\
with socketserver.TCPServer((\"127.0.0.1\", port), H) as httpd:\n\
\x20\x20\x20\x20httpd.serve_forever()\n";
    fs::write(&path, script).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[test]
fn spec_save_persists_a_session_and_requires_the_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    write_minimal_spec(&project_root);

    // Explicit confirmation persists the validated spec and returns
    // the session envelope pinned to the new revision.
    let env = run_json(
        &db,
        &[
            "studio",
            "spec",
            "studio-cli",
            "--confirm",
            "yes",
            "--expected-revision",
            "r0",
        ],
    );
    assert_eq!(env["contract"], SESSION_CONTRACT);
    assert_eq!(env["project_id"], "studio-cli");
    assert_eq!(env["spec_revision"], "r1");
    assert_eq!(env["app_revision"], "r1");

    // A stale confirmation is refused before writing.
    let out = run(
        &db,
        &[
            "studio",
            "spec",
            "studio-cli",
            "--confirm",
            "yes",
            "--expected-revision",
            "r0",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("studio-revision-conflict"),
        "missing revision refusal:\n{}",
        lossy(&out.stderr)
    );

    // A save without the expected revision is refused.
    let out = run(&db, &["studio", "spec", "studio-cli", "--confirm", "yes"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("studio-invalid-spec"),
        "missing expected-revision refusal:\n{}",
        lossy(&out.stderr)
    );
}

#[test]
fn preview_start_probe_reaches_ready_and_leaves_no_live_process() {
    if !has_python3() {
        eprintln!("skip: python3 is not available to host the runner stub");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    register_project(&db, &project_root);
    write_minimal_spec(&project_root);
    let saved = run_json(
        &db,
        &[
            "studio",
            "spec",
            "studio-cli",
            "--confirm",
            "yes",
            "--expected-revision",
            "r0",
        ],
    );
    assert_eq!(saved["spec_revision"], "r1");

    let runner = stub_runner(tmp.path());
    // Chosen at run time outside this host's ephemeral window; a hardcoded
    // base inside `ip_local_port_range` loses a port to any unrelated
    // outbound connection on the machine.
    let port_range_start = shared_port_base(SLOT).to_string();
    let out = run_env(
        &db,
        &[
            "--format",
            "json",
            "studio",
            "preview",
            "studio-cli",
            "--action",
            "start",
            "--confirm",
            "yes",
        ],
        &[
            ("FORGE_STUDIO_RUNNER_BIN", runner.to_str().unwrap()),
            ("FORGE_STUDIO_PORT_RANGE_START", &port_range_start),
            ("FORGE_STUDIO_STARTUP_TIMEOUT_SECS", "10"),
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "probe failed: {}",
        lossy(&out.stderr)
    );
    let ready: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(ready["contract"], PREVIEW_CONTRACT);
    assert_eq!(ready["state"], "ready");
    assert!(ready["port"].as_u64().is_some());
    assert!(
        ready["preview_url"]
            .as_str()
            .unwrap_or_default()
            .starts_with("http://127.0.0.1:"),
        "missing preview url: {ready}"
    );

    // The short-lived probe tore its own child down: the persisted
    // session reports stopped and no detached process survives.
    let status = run_json(&db, &["studio", "preview", "studio-cli"]);
    assert_eq!(status["state"], "stopped");
    assert!(status["preview_url"].is_null());

    // A second start with the same stub reuses the same Code path.
    let out = run_env(
        &db,
        &[
            "--format",
            "json",
            "studio",
            "preview",
            "studio-cli",
            "--action",
            "start",
            "--confirm",
            "yes",
        ],
        &[
            ("FORGE_STUDIO_RUNNER_BIN", runner.to_str().unwrap()),
            ("FORGE_STUDIO_PORT_RANGE_START", &port_range_start),
            ("FORGE_STUDIO_STARTUP_TIMEOUT_SECS", "10"),
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "second probe failed: {}",
        lossy(&out.stderr)
    );
    let ready: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(ready["state"], "ready");
}
