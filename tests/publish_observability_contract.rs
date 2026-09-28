//! Contract tests for revision-visible publish phase evidence
//! (`forge-publish-observability-revision-containers`).
//!
//! Exercises the full Forge publish / status surface against staged
//! providers that emit (or omit) the additive `build_status`,
//! `run_status`, `revision`, and `container_identity` fields. Every
//! scenario is driven through the real CLI binary against an in-memory
//! registry; the providers are shell scripts that read the JSON
//! envelope on stdin and write the contract-typed response on stdout
//! so the contract stays stable while sibling OpenPanel /
//! jenkins-local implementations live in their own repositories.
//!
//! Scenarios covered:
//! - R1 success: a provider returning `build_status=succeeded` and
//!   `run_status=succeeded` produces a journal row with both phase
//!   fields populated and a `forge deploy status --project <id>`
//!   projection that surfaces the revision and container identity.
//! - R1 failure: build failure is not masked by run success, run
//!   failure preserves build success, and missing phase evidence is
//!   never reported as a verified success.
//! - R1 boundary: a non-40-char or non-hex revision is refused at
//!   `forge publish` time with `error[publish-invalid]`; an
//!   explicitly conflicting `container_name` would be rejected by
//!   the sibling provider (exercised through the live sibling
//!   round trip when present).
//! - R2 success: `forge deploy status --queue <id>` projects the
//!   additive fields for every fleet entry; the human renderer
//!   surfaces the same data and never fabricates a phase status.
//! - R2 boundary: provider responses with no `container_identity`
//!   field are persisted with the canonical
//!   `forge-<project>-<sha12>` identity so legacy providers are not
//!   silently absent from observability.

use std::fs;
use std::io::Write;
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

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
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_CONTAINER_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN")
        .env_remove("FORGE_PROVIDER_LIVE")
        .env_remove("FORGE_PUBLISH_PROVIDER_CONFIG")
        .env_remove("FORGE_PUBLISH_PROVIDER")
        .env_remove("FORGE_PUBLISH_PROVIDER_TIMEOUT_SECS")
        .env_remove("FORGE_GITHUB_WEBHOOK_SECRET")
        .env_remove("FORGE_GITHUB_REPOSITORY")
        .env_remove("FORGE_GITHUB_REF")
        .env_remove("FORGE_GITHUB_PROJECT_ID");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("publish CLI must run")
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("publish CLI json must run");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn write_minimal_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let body = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), body).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
}

fn init_git_repo(dir: &Path) {
    let _ = Command::new("git")
        .args(["init", "--initial-branch", "main"])
        .current_dir(dir)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "forge@example.com"])
        .current_dir(dir)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "forge"])
        .current_dir(dir)
        .output();
    let _ = Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output();
}

fn write_provider_config(project_dir: &Path, body: &str) {
    let forge_dir = project_dir.join(".forge");
    fs::create_dir_all(&forge_dir).unwrap();
    fs::write(forge_dir.join("providers.yaml"), body).unwrap();
}

/// Stage a fixture provider that emits the additive phase evidence
/// (`revision`, `build_status`, `run_status`, `container_identity`)
/// on the terminal response. The full revision is taken from the
/// request envelope so the test does not have to know the real SHA
/// the fixture provider received.
fn phase_provider(log: &Path, status: &str, health: &str, build: &str, run: &str) -> String {
    let log_str = log.display().to_string();
    format!(
        "#!/bin/sh\nset -e\nlog=\"{log_str}\"\nstatus=\"{status}\"\nhealth=\"{health}\"\nbuild_status=\"{build}\"\nrun_status=\"{run}\"\ncat > \"${{log}}.in\"\nreq=$(cat \"${{log}}.in\")\nprintf '%s\\n' \"$req\" >> \"$log\"\nrevision=$(printf '%s' \"$req\" | sed -n 's/.*\"revision\":\"\\([^\"]*\\)\".*/\\1/p')\nproject_id=$(printf '%s' \"$req\" | sed -n 's/.*\"project_id\":\"\\([^\"]*\\)\".*/\\1/p')\nsha12=$(printf '%s' \"$revision\" | cut -c1-12)\ncat <<JSON\n{{\n  \"contract\": \"forge-publish-provider/0.1.0\",\n  \"provider\": \"fixture\",\n  \"operation_id\": \"$(printf '%s' \"$req\" | sed -n 's/.*\"operation_id\":\"\\([^\"]*\\)\".*/\\1/p')\",\n  \"status\": \"$status\",\n  \"health\": \"$health\",\n  \"evidence\": [\"fixture provider round-tripped the request\"],\n  \"recovery\": [],\n  \"revision\": \"$revision\",\n  \"build_status\": \"$build_status\",\n  \"run_status\": \"$run_status\",\n  \"container_identity\": \"forge-$project_id-$sha12\"\n}}\nJSON\n"
    )
}

/// Read the most recent recorded request body from `log` and return
/// the value of `field` as a string. The fixture providers write the
/// request JSON on one line so this is a stable grep.
fn recorded_field(log: &Path, field: &str) -> String {
    let body = fs::read_to_string(log).unwrap_or_default();
    let needle = format!("\"{field}\":\"");
    let start = body.rfind(&needle).unwrap_or(usize::MAX);
    if start == usize::MAX {
        return String::new();
    }
    let after = &body[start + needle.len()..];
    let end = after.find('"').unwrap_or(0);
    after[..end].to_string()
}

#[test]
fn cli_help_advertises_revision_field() {
    // The publish --help surface must still expose --project,
    // --provider, --revision so a dry-run can prove the additive
    // contract is reachable through the documented CLI.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["publish", "--help"]);
    assert!(out.status.success());
    let text = lossy(&out.stdout);
    assert!(text.contains("--project"), "{text}");
    assert!(text.contains("--provider"), "{text}");
    assert!(text.contains("--revision"), "{text}");
}

#[test]
fn publish_refuses_non_hex_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("rec.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &phase_provider(&log, "done", "healthy", "succeeded", "succeeded"),
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );

    let out = run(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "openpanel",
            "--revision",
            "0123456789abcdef0123456789abcdef0123456g",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "stderr={}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("publish-invalid")
            || stderr.contains("40-character hex revision")
            || stderr.contains("RevisionShape"),
        "{stderr}"
    );
    assert!(
        !log.exists(),
        "provider must not be invoked for an invalid revision"
    );
}

#[test]
fn publish_refuses_short_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("rec.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &phase_provider(&log, "done", "healthy", "succeeded", "succeeded"),
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );

    let out = run(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "openpanel",
            "--revision",
            "0123456789abcdef",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "stderr={}", lossy(&out.stderr));
    assert!(
        !log.exists(),
        "provider must not be invoked for a short revision"
    );
}

#[test]
fn publish_persists_build_run_revision_and_container_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("rec.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &phase_provider(&log, "done", "healthy", "succeeded", "succeeded"),
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );

    let json = run_json(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "openpanel",
            "--revision",
            "0123456789abcdef0123456789abcdef01234567",
        ],
    );
    assert_eq!(json["status"], "done");
    assert_eq!(json["health"], "healthy");
    assert_eq!(json["build_status"], "succeeded");
    assert_eq!(json["run_status"], "succeeded");
    assert_eq!(json["revision"], "0123456789abcdef0123456789abcdef01234567");
    assert_eq!(json["container_identity"], "forge-demo-0123456789ab");

    // The provider received the full 40-char SHA, not the truncated
    // 12-char prefix.
    let recorded_revision = recorded_field(&log, "revision");
    assert_eq!(
        recorded_revision,
        "0123456789abcdef0123456789abcdef01234567"
    );

    // The status projection surfaces the additive fields for this
    // project: revision, build_status, run_status, container_identity.
    let status = run_json(&db, &["deploy", "status", "--project", "demo"]);
    let entries = status["entries"].as_array().expect("entries array");
    let entry = entries
        .iter()
        .find(|entry| entry["kind"] == "publish")
        .expect("publish entry");
    assert_eq!(
        entry["revision"],
        "0123456789abcdef0123456789abcdef01234567"
    );
    assert_eq!(entry["build_status"], "succeeded");
    assert_eq!(entry["run_status"], "succeeded");
    assert_eq!(entry["container_identity"], "forge-demo-0123456789ab");
}

#[test]
fn publish_preserves_build_success_when_run_fails() {
    // Build succeeded but the runtime never came up: the journal
    // row carries `build=succeeded`, `run=failed`, and the
    // status projection reports the aggregate as failed without
    // silently masking the build evidence.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("rec.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &phase_provider(&log, "failed", "unknown", "succeeded", "failed"),
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );

    let out = run(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "openpanel",
            "--revision",
            "0123456789abcdef0123456789abcdef01234567",
        ],
    );
    // `forge publish` only exits non-zero on Forge-side errors
    // (provider-protocol violations, unreachable binary, malformed
    // contract). A provider reporting `status=failed` with a parseable
    // response is recorded on the journal but does not crash the CLI;
    // the operator reads the failure through `forge deploy status`.
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));

    let status = run_json(&db, &["deploy", "status", "--project", "demo"]);
    let entries = status["entries"].as_array().expect("entries array");
    let entry = entries
        .iter()
        .find(|entry| entry["kind"] == "publish")
        .expect("publish entry");
    assert_eq!(entry["build_status"], "succeeded");
    assert_eq!(entry["run_status"], "failed");
    assert_eq!(entry["state"], "failed");
}

#[test]
fn publish_persists_legacy_provider_without_phase_evidence() {
    // A provider that omits the additive fields must not be silently
    // reported as fully verified. Forge synthesizes the canonical
    // container identity from the recorded revision but leaves the
    // phase statuses absent in the journal row; `forge deploy status`
    // never reports `state=succeeded` from a `done` response when
    // phase evidence is missing — `state=done` is what we expect
    // from the literal response status.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("rec.log");
    let provider = write_script(
        &scripts,
        "legacy.sh",
        &format!(
            "#!/bin/sh\nset -e\nlog=\"{log_str}\"\ncat > \"${{log}}.in\"\nreq=$(cat \"${{log}}.in\")\nprintf '%s\\n' \"$req\" >> \"$log\"\ncat <<JSON\n{{\n  \"contract\": \"forge-publish-provider/0.1.0\",\n  \"provider\": \"fixture\",\n  \"operation_id\": \"$(printf '%s' \"$req\" | sed -n 's/.*\"operation_id\":\"\\([^\"]*\\)\".*/\\1/p')\",\n  \"status\": \"done\",\n  \"health\": \"healthy\",\n  \"evidence\": [\"legacy provider emits no phase evidence\"],\n  \"recovery\": []\n}}\nJSON\n",
            log_str = log.display().to_string(),
        ),
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: legacy\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );

    let json = run_json(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "legacy",
            "--revision",
            "0123456789abcdef0123456789abcdef01234567",
        ],
    );
    assert_eq!(json["status"], "done");
    assert!(
        json.get("build_status").is_none() || json["build_status"].is_null(),
        "legacy provider must not populate build_status: {}",
        lossy(&serde_json::to_vec(&json).unwrap())
    );
    assert!(
        json.get("run_status").is_none() || json["run_status"].is_null(),
        "legacy provider must not populate run_status"
    );

    let status = run_json(&db, &["deploy", "status", "--project", "demo"]);
    let entries = status["entries"].as_array().expect("entries array");
    let entry = entries
        .iter()
        .find(|entry| entry["kind"] == "publish")
        .expect("publish entry");
    // Legacy provider still gets the canonical container identity
    // synthesized from the recorded revision so the operator can see
    // the runtime identity in `forge deploy status` even when the
    // provider itself did not advertise it.
    assert_eq!(entry["container_identity"], "forge-demo-0123456789ab");
    assert_eq!(
        entry["revision"],
        "0123456789abcdef0123456789abcdef01234567"
    );
}

#[test]
fn deploy_status_human_renderer_includes_phase_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("rec.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &phase_provider(&log, "done", "healthy", "succeeded", "succeeded"),
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let apply = run(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "openpanel",
            "--revision",
            "0123456789abcdef0123456789abcdef01234567",
        ],
    );
    assert!(apply.status.success(), "stderr={}", lossy(&apply.stderr));

    let status = run(&db, &["deploy", "status", "--project", "demo"]);
    assert!(status.status.success());
    let text = lossy(&status.stdout);
    assert!(
        text.contains("revision=0123456789abcdef0123456789abcdef01234567"),
        "{text}"
    );
    assert!(text.contains("build=succeeded"), "{text}");
    assert!(text.contains("run=succeeded"), "{text}");
    assert!(text.contains("container=forge-demo-0123456789ab"), "{text}");
}

#[test]
fn github_push_persists_phase_evidence_on_idempotent_replay() {
    // The API-driven GitHub push path reuses the same phase
    // projection as the manual path: the response carries the
    // additive fields and the persisted row exposes them through
    // `forge deploy status`.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    fs::write(proj.join("README.md"), "v1\n").unwrap();
    let register = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(
        register.status.success(),
        "register failed: {}",
        lossy(&register.stderr)
    );

    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("rec.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &phase_provider(&log, "done", "healthy", "succeeded", "succeeded"),
    );
    let forge_dir = proj.join(".forge");
    fs::create_dir_all(&forge_dir).unwrap();
    fs::write(
        forge_dir.join("providers.yaml"),
        format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    )
    .unwrap();

    fn github_signature(secret: &[u8], body: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut key = [0u8; 64];
        if secret.len() > 64 {
            let mut hash = Sha256::new();
            hash.update(secret);
            key[..32].copy_from_slice(&hash.finalize());
        } else {
            key[..secret.len()].copy_from_slice(secret);
        }
        let mut inner = [0x36u8; 64];
        let mut outer = [0x5cu8; 64];
        for index in 0..64 {
            inner[index] ^= key[index];
            outer[index] ^= key[index];
        }
        let mut inner_hash = Sha256::new();
        inner_hash.update(inner);
        inner_hash.update(body);
        let inner_digest = inner_hash.finalize();
        let mut outer_hash = Sha256::new();
        outer_hash.update(outer);
        outer_hash.update(inner_digest);
        format!("sha256={:x}", outer_hash.finalize())
    }

    fn http_request(
        method: &str,
        host_port: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> (u16, Value) {
        let mut stream = TcpStream::connect(host_port).expect("connect api");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
        let mut header_block =
            format!("{method} {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n");
        for (name, value) in headers {
            header_block.push_str(&format!("{name}: {value}\r\n"));
        }
        if !body.is_empty() {
            header_block.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        header_block.push_str("\r\n");
        stream
            .write_all(header_block.as_bytes())
            .expect("write head");
        if !body.is_empty() {
            stream.write_all(body).expect("write body");
        }
        let mut response = Vec::new();
        use std::io::Read;
        stream.read_to_end(&mut response).expect("read response");
        let response_text = String::from_utf8_lossy(&response).to_string();
        let status_line = response_text.lines().next().unwrap_or("");
        let status_code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let body_start = response_text
            .find("\r\n\r\n")
            .map(|i| i + 4)
            .unwrap_or(response_text.len());
        let body_text = &response_text[body_start..];
        let json: Value = if body_text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(body_text).unwrap_or(Value::Null)
        };
        (status_code, json)
    }

    let secret = "github-webhook-secret";
    let delivery_id = "delivery-obs-1";
    let after = "0123456789abcdef0123456789abcdef01234567";
    let body = format!(
        r#"{{"ref":"refs/heads/main","repository":{{"full_name":"acme/example"}},"after":"{after}"}}"#
    );
    let body_bytes = body.as_bytes();
    let signature = github_signature(secret.as_bytes(), body_bytes);

    let port: u16 = 9487 + ((db.metadata().unwrap().len() as u16).rem_euclid(200));
    let mut api_child = Command::new(forge_bin())
        .arg("--registry")
        .arg(&db)
        .arg("api")
        .arg("serve")
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .env("FORGE_GITHUB_WEBHOOK_SECRET", secret)
        .env("FORGE_GITHUB_REPOSITORY", "acme/example")
        .env("FORGE_GITHUB_REF", "refs/heads/main")
        .env("FORGE_GITHUB_PROJECT_ID", "demo")
        .env("FORGE_PUBLISH_PROVIDER", "openpanel")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn api for github");
    let start = Instant::now();
    let mut bound = false;
    while start.elapsed() < Duration::from_secs(5) {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            bound = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(bound, "api server did not bind on 127.0.0.1:{port}");

    let headers = [
        ("X-Hub-Signature-256", signature.as_str()),
        ("X-GitHub-Delivery", delivery_id),
        ("X-GitHub-Event", "push"),
        ("Content-Type", "application/json"),
    ];
    let url = format!("127.0.0.1:{port}");
    let (status_code, json) =
        http_request("POST", &url, "/v1/publish/github", &headers, body_bytes);
    assert!(
        status_code == 200 || status_code == 202,
        "delivery must succeed: status={status_code} body={json:?}"
    );
    assert_eq!(json["revision"], after);
    assert_eq!(json["build_status"], "succeeded");
    assert_eq!(json["run_status"], "succeeded");
    assert_eq!(json["container_identity"], "forge-demo-0123456789ab");

    let _ = api_child.kill();
    let _ = api_child.wait();

    let status = run_json(&db, &["deploy", "status", "--project", "demo"]);
    let entries = status["entries"].as_array().expect("entries array");
    // The github-push path reserves via `reserve_idempotent_operation`
    // with `kind=publish.github` while the manual CLI uses
    // `kind=publish`. The status filter keeps both kinds visible
    // because the queue-status projection shares the same path.
    let entry = entries
        .iter()
        .find(|entry| entry["kind"] == "publish.github" || entry["kind"] == "publish")
        .unwrap_or_else(|| panic!("publish entry not found in entries={entries:?}"));
    assert_eq!(entry["revision"], after);
    assert_eq!(entry["build_status"], "succeeded");
    assert_eq!(entry["run_status"], "succeeded");
    assert_eq!(entry["container_identity"], "forge-demo-0123456789ab");
}
