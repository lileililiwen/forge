use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn run(db: &Path, args: &[&str], cwd: Option<&Path>) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    cmd.output().expect("cwd publish CLI must run")
}

fn run_json(db: &Path, args: &[&str], cwd: Option<&Path>) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    let out = cmd.output().expect("cwd publish json must run");
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

fn recording_provider(log: &Path, status: &str, health: &str) -> String {
    let log_str = log.display().to_string();
    format!(
        "#!/bin/sh\nset -e\nlog=\"{log_str}\"\nstatus=\"{status}\"\nhealth=\"{health}\"\ncat > \"${{log}}.in\"\nreq=$(cat \"${{log}}.in\")\nprintf '%s\\n' \"$req\" >> \"$log\"\ncat <<JSON\n{{\n  \"contract\": \"forge-publish-provider/0.1.0\",\n  \"provider\": \"fixture\",\n  \"operation_id\": \"$(printf '%s' \"$req\" | sed -n 's/.*\"operation_id\":\"\\([^\"]*\\)\".*/\\1/p')\",\n  \"status\": \"$status\",\n  \"health\": \"$health\",\n  \"evidence\": [\"fixture\"],\n  \"recovery\": []\n}}\nJSON\n"
    )
}

#[test]
fn publish_help_advertises_cwd_and_bare_publish() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["publish", "--help"], None);
    assert!(out.status.success());
    let text = lossy(&out.stdout);
    assert!(text.contains("--cwd"), "{text}");
    assert!(text.contains("--provider"), "{text}");
}

#[test]
fn cwd_publish_discovers_project_json() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("alethefy");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(
        sibling.join(".project.json"),
        r#"{"schema_version":1,"id":"alethefy","profile":"python-product"}"#,
    )
    .unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&sibling);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("recording.log");
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&log, "done", "healthy"),
    );
    write_provider_config(
        &sibling,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let json = run_json(
        &db,
        &["publish", "--provider", "jenkins", "--dry-run"],
        Some(&sibling),
    );
    assert_eq!(json["project_id"], "alethefy");
    assert_eq!(json["provider"], "jenkins");
    assert_eq!(json["contract"], "forge-publish-provider/0.1.0");
    assert!(!log.exists(), "dry-run must not spawn provider");
}

#[test]
fn cwd_publish_falls_back_to_forge_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("demo-app");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(
        sibling.join("forge.yaml"),
        "schema: 1\nproject:\n  id: demo-app\n  name: demo\n  profile: rust-web\n  maturity: L1\n",
    )
    .unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&sibling);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("recording.log");
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&log, "done", "healthy"),
    );
    write_provider_config(
        &sibling,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let json = run_json(
        &db,
        &["publish", "--provider", "jenkins", "--dry-run"],
        Some(&sibling),
    );
    assert_eq!(json["project_id"], "demo-app");
}

#[test]
fn cwd_publish_falls_back_to_directory_basename() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("my-service");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&sibling);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&scripts.join("log"), "done", "healthy"),
    );
    write_provider_config(
        &sibling,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let json = run_json(
        &db,
        &["publish", "--provider", "jenkins", "--dry-run"],
        Some(&sibling),
    );
    assert_eq!(json["project_id"], "my-service");
}

#[test]
fn cwd_explicit_flag_overrides_process_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("alethefy");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(
        sibling.join(".project.json"),
        r#"{"schema_version":1,"id":"alethefy"}"#,
    )
    .unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&sibling);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&scripts.join("log"), "done", "healthy"),
    );
    write_provider_config(
        &sibling,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let other = tmp.path().join("other");
    fs::create_dir_all(&other).unwrap();
    let json = run_json(
        &db,
        &[
            "publish",
            "--cwd",
            sibling.to_str().unwrap(),
            "--provider",
            "jenkins",
            "--dry-run",
        ],
        Some(&other),
    );
    assert_eq!(json["project_id"], "alethefy");
}

#[test]
fn cwd_folder_flag_beats_cwd_discovery() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let cwd_sibling = tmp.path().join("cwd-proj");
    fs::create_dir_all(&cwd_sibling).unwrap();
    fs::write(cwd_sibling.join(".project.json"), r#"{"id":"cwd-proj"}"#).unwrap();
    fs::write(cwd_sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&cwd_sibling);
    write_provider_config(
        &cwd_sibling,
        "providers:\n  - id: jenkins\n    command: \"/bin/false\"\n    enabled: true\n",
    );

    let folder = tmp.path().join("folder-proj");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("README.md"), "hi\n").unwrap();
    init_git_repo(&folder);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&scripts.join("log"), "done", "healthy"),
    );
    write_provider_config(
        &folder,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let json = run_json(
        &db,
        &[
            "publish",
            "--folder",
            folder.to_str().unwrap(),
            "--provider",
            "jenkins",
            "--dry-run",
        ],
        Some(&cwd_sibling),
    );
    assert_eq!(json["project_id"], "folder-proj");
}

#[test]
fn malformed_project_json_is_typed_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("bad");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join(".project.json"), "not json").unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    let out = run(&db, &["publish", "--dry-run"], Some(&sibling));
    assert_ne!(out.status.code(), Some(0));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("publish-invalid") || stderr.contains("malformed"),
        "{stderr}"
    );
    assert!(
        lossy(&out.stdout).is_empty(),
        "typed failure has empty stdout"
    );
}

#[test]
fn invalid_basename_cwd_is_typed_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("bad name");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    let out = run(&db, &["publish", "--dry-run"], Some(&sibling));
    assert_ne!(out.status.code(), Some(0));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("publish-invalid"), "{stderr}");
}

#[test]
fn cwd_non_hex_revision_is_refused_before_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("alethefy");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join(".project.json"), r#"{"id":"alethefy"}"#).unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&sibling);
    let log = tmp.path().join("should-not-exist.log");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&log, "done", "healthy"),
    );
    write_provider_config(
        &sibling,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let out = run(
        &db,
        &["publish", "--provider", "jenkins", "--revision", "short"],
        Some(&sibling),
    );
    assert_ne!(out.status.code(), Some(0));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("publish-invalid"), "{stderr}");
    assert!(
        !log.exists(),
        "provider must not be invoked on bad revision"
    );
}

#[test]
fn cwd_disabled_provider_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("alethefy");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join(".project.json"), r#"{"id":"alethefy"}"#).unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&sibling);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&scripts.join("log"), "done", "healthy"),
    );
    write_provider_config(
        &sibling,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: false\n",
            provider.display()
        ),
    );
    let out = run(&db, &["publish", "--provider", "jenkins"], Some(&sibling));
    assert_ne!(out.status.code(), Some(0));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("publish-invalid") || stderr.contains("disabled"),
        "{stderr}"
    );
}

#[test]
fn cwd_publish_persists_and_visible_via_deploy_status() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("alethefy");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join(".project.json"), r#"{"id":"alethefy"}"#).unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&sibling);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let log = scripts.join("recording.log");
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&log, "done", "healthy"),
    );
    write_provider_config(
        &sibling,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let out = run(
        &db,
        &["publish", "--provider", "jenkins", "--format", "json"],
        Some(&sibling),
    );
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let status = run_json(
        &db,
        &[
            "deploy",
            "status",
            "--project",
            "alethefy",
            "--format",
            "json",
        ],
        None,
    );
    assert_eq!(status["project"], "alethefy");
    let entries = status["entries"].as_array().unwrap();
    assert!(!entries.is_empty(), "deploy status must have entries");
}

#[test]
fn project_json_takes_precedence_over_forge_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let sibling = tmp.path().join("mixed");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join(".project.json"), r#"{"id":"from-json"}"#).unwrap();
    fs::write(
        sibling.join("forge.yaml"),
        "schema: 1\nproject:\n  id: from-yaml\n",
    )
    .unwrap();
    fs::write(sibling.join("README.md"), "hi\n").unwrap();
    init_git_repo(&sibling);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider = write_script(
        &scripts,
        "prov.sh",
        &recording_provider(&scripts.join("log"), "done", "healthy"),
    );
    write_provider_config(
        &sibling,
        &format!(
            "providers:\n  - id: jenkins\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    );
    let json = run_json(
        &db,
        &["publish", "--provider", "jenkins", "--dry-run"],
        Some(&sibling),
    );
    assert_eq!(json["project_id"], "from-json");
}
