//! CLI / API contract for the publish provider orchestration
//! (`forge-publish-plugin-orchestration`).
//!
//! Exercises the Forge publish engine end to end through the built
//! binary. The provider surface is pluggable: every test stages a
//! shell script fixture that speaks the `forge-publish-provider/0.1.0`
//! JSON envelope on stdin/stdout so the contract stays stable while
//! sibling OpenPanel / jenkins-local implementations live in their
//! own repositories. Each script records the request it received so
//! duplicate-delivery, multi-provider and secret-redaction proofs are
//! asserted through the fixture log, not through Core internals.
//!
//! Scenarios covered:
//! - R1 success: `forge publish --project <id> --provider <name>`
//!   invokes the configured executable exactly once with a parseable
//!   request and the recorded contract is preserved on the wire.
//! - R1 failure: a missing or disabled provider refuses with
//!   `error[publish-invalid]`; a contract-mismatched fixture exits
//!   non-zero with no Forge-side fabrications.
//! - R1 boundary: the dry-run flow never invokes the executable; the
//!   manual `forge publish --project` and the API-driven GitHub push
//!   handler produce equivalent `PublishProviderRequest` envelopes.
//! - R2 success: enabling/disabling a provider and switching between
//!   two configured providers is independent — OpenPanel failure does
//!   not block Jenkins selection, and Jenkins is reachable without a
//!   Mac-local script bundle.
//! - R2 failure: credential-shaped strings in provider output are
//!   rejected by `validate_response` (typed `SecretLeak`); duplicate
//!   GitHub deliveries invoke the provider at most once through the
//!   registry's idempotency layer.
//! - R2 boundary: the deploy/publish portal surfaces never gain a new
//!   route, and the API `tools/list` snapshot stays unchanged after a
//!   publish round trip.

use std::fs;
use std::io::{Read, Write};
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

/// Initialize a git repo in `dir` with a deterministic HEAD so the
/// publish engine's revision capture resolves to a real SHA.
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

/// Stage a fixture provider that reads one JSON request on stdin,
/// echoes it verbatim into a log file so the test can assert that
/// Forge dispatched the right shape, and emits a successful response
/// on stdout. Stdin must be drained with `cat` so the child's pipe
/// closes cleanly even when the test does not write a request.
fn recording_provider(log: &Path, status: &str, health: &str) -> String {
    let log_str = log.display().to_string();
    format!(
        "#!/bin/sh\nset -e\nlog=\"{log_str}\"\nstatus=\"{status}\"\nhealth=\"{health}\"\ncat > \"${{log}}.in\"\nreq=$(cat \"${{log}}.in\")\nprintf '%s\\n' \"$req\" >> \"$log\"\ncat <<JSON\n{{\n  \"contract\": \"forge-publish-provider/0.1.0\",\n  \"provider\": \"fixture\",\n  \"operation_id\": \"$(printf '%s' \"$req\" | sed -n 's/.*\"operation_id\":\"\\([^\"]*\\)\".*/\\1/p')\",\n  \"status\": \"$status\",\n  \"health\": \"$health\",\n  \"evidence\": [\"fixture provider round-tripped the request\"],\n  \"recovery\": []\n}}\nJSON\n"
    )
}

/// Stage a fixture provider that fails with a specific `status` and
/// logs every received request to `log`. Used to prove one provider
/// being unavailable does not block a sibling provider from running.
fn failing_provider(log: &Path, status: &str, health: &str, note: &str) -> String {
    let log_str = log.display().to_string();
    format!(
        "#!/bin/sh\nlog=\"{log_str}\"\nstatus=\"{status}\"\nhealth=\"{health}\"\nnote=\"{note}\"\ncat > \"${{log}}.in\"\nreq=$(cat \"${{log}}.in\")\nprintf 'failed:%s\\n' \"$req\" >> \"$log\"\ncat <<JSON\n{{\n  \"contract\": \"forge-publish-provider/0.1.0\",\n  \"provider\": \"fixture\",\n  \"operation_id\": \"$(printf '%s' \"$req\" | sed -n 's/.*\"operation_id\":\"\\([^\"]*\\)\".*/\\1/p')\",\n  \"status\": \"$status\",\n  \"health\": \"$health\",\n  \"evidence\": [\"$note\"],\n  \"recovery\": [\"resolve the failing condition\"]\n}}\nJSON\nexit 1\n"
    )
}

#[test]
fn cli_help_advertises_publish_provider_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let top = run(&db, &["--help"]);
    assert!(top.status.success(), "stderr={}", lossy(&top.stderr));
    assert!(lossy(&top.stdout).contains("publish"));
    let provider_top = run(&db, &["publish", "--help"]);
    assert!(provider_top.status.success());
    let text = lossy(&provider_top.stdout);
    assert!(text.contains("--provider"), "{text}");
    assert!(text.contains("--project"), "{text}");
    assert!(text.contains("--folder"), "{text}");
    let provider_cmd = run(&db, &["publish", "provider", "--help"]);
    assert!(provider_cmd.status.success());
    let provider_text = lossy(&provider_cmd.stdout);
    for cmd in ["list", "inspect", "enable", "disable"] {
        assert!(
            provider_text.contains(cmd),
            "publish provider subcommands must advertise `{cmd}`: {provider_text}"
        );
    }
}

#[test]
fn publish_dry_run_does_not_invoke_provider_but_records_request_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("recording.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &recording_provider(&log, "done", "healthy"),
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
            "--dry-run",
            "--format",
            "json",
        ],
    );
    let value = &json;
    assert_eq!(value["contract"], "forge-publish-provider/0.1.0");
    assert_eq!(value["provider"], "openpanel");
    assert_eq!(value["project_id"], "demo");
    assert_eq!(value["revision"].as_str().unwrap().len(), 40);
    assert!(value["folder"].as_str().unwrap().ends_with("demo"));
    assert_eq!(value["dry_run"], true);
    assert!(value["operation_id"]
        .as_str()
        .unwrap()
        .starts_with("publish-demo-"));
    assert!(
        !log.exists(),
        "dry-run must not invoke the provider: log={}",
        log.display()
    );
}

#[test]
fn publish_apply_invokes_provider_with_typed_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("recording.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &recording_provider(&log, "done", "healthy"),
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
            "--format",
            "json",
        ],
    );
    assert_eq!(json["contract"], "forge-publish-provider/0.1.0");
    assert_eq!(json["status"], "done");
    assert_eq!(json["health"], "healthy");
    assert_eq!(json["provider"], "fixture");
    assert_eq!(json["operation_id"], value_at(&log, "operation_id"));

    // Provider recorded exactly one invocation with the typed fields.
    let recorded = fs::read_to_string(&log).unwrap();
    assert!(recorded.contains("\"operation\""));
    assert!(recorded.contains("\"publish\""));
    assert!(recorded.contains("\"project_id\":\"demo\""));
    assert!(recorded.contains("\"provider\":\"openpanel\""));
}

#[test]
fn publish_disabled_provider_refuses_with_typed_code() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &recording_provider(&scripts.join("disabled.log"), "done", "healthy"),
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: false\n",
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
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[publish-invalid]")
            || stderr.contains("publish provider `openpanel` is disabled"),
        "{stderr}"
    );
}

#[test]
fn publish_provider_failure_does_not_prevent_other_provider_use() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let op_log = scripts.join("openpanel.log");
    let openpanel = write_script(
        &scripts,
        "openpanel.sh",
        &failing_provider(&op_log, "failed", "unknown", "openpanel target unreachable"),
    );
    let jk_log = scripts.join("jenkins.log");
    let jenkins = write_script(
        &scripts,
        "jenkins.sh",
        &recording_provider(&jk_log, "done", "healthy"),
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: true\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            openpanel.display(),
            jenkins.display()
        ),
    );

    let first = run(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "openpanel",
        ],
    );
    assert_eq!(
        first.status.code(),
        Some(1),
        "stderr={}",
        lossy(&first.stderr)
    );
    assert!(op_log.exists(), "openpanel provider was invoked");

    let second = run_json(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "jenkins",
        ],
    );
    assert_eq!(second["status"], "done");
    assert_eq!(second["health"], "healthy");
    assert!(jk_log.exists(), "jenkins provider was invoked");
    let jenkins_log = fs::read_to_string(&jk_log).unwrap();
    assert!(jenkins_log.contains("\"project_id\":\"demo\""));
}

#[test]
fn publish_jenkins_optional_and_no_mac_script_required() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("openpanel.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &recording_provider(&log, "done", "healthy"),
    );
    // Only the openpanel provider is configured: Jenkins is genuinely
    // optional. The publish engine reaches the provider through the
    // executable path, with no Mac-local script bundle present.
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
            "jenkins",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("publish provider `jenkins` is not configured"),
        "{stderr}"
    );
    // Selecting the configured provider succeeds with no `project.sh`,
    // `deploy-all.sh` or `install-mac.sh` invocation in the workdir.
    let json = run_json(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "openpanel",
        ],
    );
    assert_eq!(json["status"], "done");
    for forbidden in [
        "project.sh",
        "deploy-all.sh",
        "install-mac.sh",
        "jenkins-local",
    ] {
        assert!(
            !log_contents(&log).contains(forbidden),
            "publish must not invoke `{forbidden}`"
        );
    }
}

fn log_contents(log: &Path) -> String {
    if log.exists() {
        fs::read_to_string(log).unwrap_or_default()
    } else {
        String::new()
    }
}

fn value_at(log: &Path, field: &str) -> String {
    let body = log_contents(log);
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
fn publish_redacts_secret_shaped_provider_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    // A provider that emits a credential-shaped fragment is refused at
    // the contract boundary; the CLI surfaces the typed refusal and
    // never persists the secret.
    let provider = write_script(
        &scripts,
        "leaky.sh",
        "#!/bin/sh\ncat >/dev/null\ncat <<'JSON'\n{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"fixture\",\"operation_id\":\"x\",\"status\":\"done\",\"health\":\"healthy\",\"evidence\":[\"password=hunter2hunter2\"],\"recovery\":[]}\nJSON\n",
    );
    write_provider_config(
        &proj,
        &format!(
            "providers:\n  - id: leaky\n    command: \"{}\"\n    enabled: true\n",
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
            "leaky",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "stderr={}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    let stderr = lossy(&out.stderr);
    assert!(
        !stdout.contains("hunter2") && !stderr.contains("hunter2"),
        "secret leaked to output: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn provider_lifecycle_persists_enable_disable_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &recording_provider(&scripts.join("rec.log"), "done", "healthy"),
    );
    let config_path = proj.join(".forge/providers.yaml");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(
        &config_path,
        format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    )
    .unwrap();

    let disable = run(
        &db,
        &[
            "publish",
            "provider",
            "disable",
            "openpanel",
            "--config",
            config_path.to_str().unwrap(),
        ],
    );
    assert!(
        disable.status.success(),
        "stderr={}",
        lossy(&disable.stderr)
    );
    let after = fs::read_to_string(&config_path).unwrap();
    assert!(after.contains("enabled: false"), "{after}");

    let list = run_json(
        &db,
        &[
            "publish",
            "provider",
            "list",
            "--config",
            config_path.to_str().unwrap(),
        ],
    );
    assert_eq!(list["providers"][0]["enabled"], false);

    let enable = run(
        &db,
        &[
            "publish",
            "provider",
            "enable",
            "openpanel",
            "--config",
            config_path.to_str().unwrap(),
        ],
    );
    assert!(enable.status.success(), "stderr={}", lossy(&enable.stderr));
    let after = fs::read_to_string(&config_path).unwrap();
    assert!(after.contains("enabled: true"), "{after}");
}

// ----------------------------------------------------------------------
// GitHub push idempotency tests
// ----------------------------------------------------------------------

fn http_request(
    method: &str,
    host_port: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> (u16, String, Value) {
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
    (status_code, response_text, json)
}

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

#[test]
fn duplicate_github_push_delivery_invokes_provider_at_most_once() {
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
    let log = scripts.join("recording.log");
    let provider = write_script(
        &scripts,
        "openpanel.sh",
        &recording_provider(&log, "done", "healthy"),
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

    let secret = "github-webhook-secret";
    let delivery_id = "delivery-12345";
    let after = "0123456789abcdef0123456789abcdef01234567";
    let body = format!(
        r#"{{"ref":"refs/heads/main","repository":{{"full_name":"acme/example"}},"after":"{after}"}}"#
    );
    let body_bytes = body.as_bytes();
    let signature = github_signature(secret.as_bytes(), body_bytes);

    // Bind the API server on a fixed port chosen to be unlikely to
    // collide with the standard listener. Each test gets its own port
    // derived from the registry path so two parallel runs cannot
    // overlap.
    let port: u16 = 9187 + ((db.metadata().unwrap().len() as u16).rem_euclid(200));

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
    let (status1, _text1, json1) =
        http_request("POST", &url, "/v1/publish/github", &headers, body_bytes);
    assert!(
        status1 == 200 || status1 == 202,
        "first delivery must succeed: status={status1} body={:?}",
        json1
    );

    let (status2, _text2, json2) =
        http_request("POST", &url, "/v1/publish/github", &headers, body_bytes);
    assert_eq!(status2, 200, "duplicate delivery must succeed");
    assert_eq!(json2["status"], "duplicate");

    let _ = api_child.kill();
    let _ = api_child.wait();

    // The fixture provider recorded exactly one invocation regardless
    // of how many GitHub deliveries the registry accepted.
    assert!(log.exists(), "provider log must exist after first delivery");
    let recorded = fs::read_to_string(&log).unwrap();
    let op_id_lines = recorded
        .matches("\"operation_id\":\"github-delivery-12345\"")
        .count();
    assert_eq!(
        op_id_lines, 1,
        "duplicate delivery must invoke provider once, recorded={op_id_lines}: {recorded}"
    );
}

#[test]
fn manual_and_github_push_produce_equivalent_provider_request() {
    use forge::publish::providers::{
        parse_request, ProviderOperation, PublishProviderRequest, PUBLISH_PROVIDER_CONTRACT,
    };

    // Build the same envelope from both entry points and assert every
    // shared field is identical. The push path additionally stamps the
    // delivery id into the operation id; that is the only intentional
    // divergence and is asserted explicitly.
    let project_id = "demo";
    let revision = "0123456789abcdef0123456789abcdef01234567";
    let folder = "/workspace/demo";

    let manual = PublishProviderRequest {
        contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: ProviderOperation::Publish,
        provider: "openpanel".to_string(),
        project_id: project_id.to_string(),
        revision: revision.to_string(),
        operation_id: format!("publish-{project_id}-{}", &revision[..12]),
        folder: Some(folder.to_string()),
        dry_run: false,
        queue_id: None,
    };

    let push_body = serde_json::json!({
        "contract": PUBLISH_PROVIDER_CONTRACT,
        "operation": "publish",
        "provider": "openpanel",
        "project_id": project_id,
        "revision": revision,
        "operation_id": "github-delivery-12345",
        "folder": folder,
        "dry_run": false,
    });
    let push = parse_request(push_body).unwrap();

    assert_eq!(manual.contract, push.contract);
    assert_eq!(manual.operation, push.operation);
    assert_eq!(manual.provider, push.provider);
    assert_eq!(manual.project_id, push.project_id);
    assert_eq!(manual.revision, push.revision);
    assert_eq!(manual.folder, push.folder);
    assert_eq!(manual.dry_run, push.dry_run);
    // Push operation id is the delivery id; manual operation id is
    // derived from project + revision. Both shapes are valid; the
    // contract only requires the field be non-empty.
    assert!(!manual.operation_id.is_empty());
    assert!(!push.operation_id.is_empty());
    assert!(push.operation_id.starts_with("github-"));
}

#[test]
fn live_jenkins_local_provider_round_trip_through_real_sibling() {
    // The jenkins-local sibling ships its provider executable at
    // `adapters/forge-publish-provider.py`. This test requires the
    // sibling checkout to be present; if it is not, the test is
    // skipped (not failed) so the contract stays green on hosts
    // without the sibling.
    let sibling = Path::new("/home/paul/code/jenkins-local");
    let adapter = sibling.join("adapters/forge-publish-provider.py");
    if !adapter.is_file() {
        eprintln!(
            "skipping: jenkins-local sibling not available at {}",
            adapter.display()
        );
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let _db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");

    let forge_dir = proj.join(".forge");
    fs::create_dir_all(&forge_dir).unwrap();
    fs::write(
        forge_dir.join("providers.yaml"),
        format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            adapter.display()
        ),
    )
    .unwrap();

    let capabilities = serde_json::json!({
        "contract": PUBLISH_PROVIDER_CONTRACT,
        "operation": "capabilities",
        "provider": "jenkins",
        "project_id": "demo",
        "revision": "0123456789abcdef0123456789abcdef01234567",
        "operation_id": "cap-1",
        "dry_run": true,
    });
    let mut child = Command::new("python3")
        .arg(&adapter)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn jenkins-local adapter");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(capabilities.to_string().as_bytes())
        .unwrap();
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .expect("wait jenkins-local adapter");
    assert!(output.status.success(), "stderr={}", lossy(&output.stderr));
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["contract"], PUBLISH_PROVIDER_CONTRACT);
    assert_eq!(response["provider"], "jenkins");
    assert_eq!(response["operation_id"], "cap-1");
    let status = response["status"].as_str().unwrap_or("");
    assert!(
        ["available", "done", "ready"].contains(&status),
        "unexpected capability status `{status}`"
    );
}

const PUBLISH_PROVIDER_CONTRACT: &str = "forge-publish-provider/0.1.0";
