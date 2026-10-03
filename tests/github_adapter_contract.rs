//! GitHub metadata adapter contract (`github-project-metadata-adapter`).
//!
//! Exercises the versioned `forge-github-metadata/0.1.0` surface
//! through the built binary against local executable stubs planted on
//! a controlled `PATH`. The contract answers the questions the
//! capability names:
//!
//! - The adapter is invoked with bounded argument arrays and a
//!   timeout; a missing or non-executable binary is a typed
//!   `github-adapter-unavailable` refusal, not a silent zero.
//! - A missing token is `github-invalid` and never an authenticated
//!   request; the adapter receives the token through the inherited
//!   environment, never on the command line.
//! - Closed state vocabulary (`current`, `stale`, `unavailable`,
//!   `unauthorized`, `forbidden`, `not-found`, `rate-limited`,
//!   `partial`); a rate-limited observation carries the reset hint
//!   and never the partial payload.
//! - PR mode is the default; direct mode requires an explicit
//!   confirmation that the adapter echoes back.
//! - Closed proposed field set (`topic`, `description`, `homepage`,
//!   `language`); an unknown field is `github-invalid`.
//!
//! Every case runs through `forge` against local stubs. No live
//! GitHub host is contacted and no repository is mutated.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-github-metadata/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn write_stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

fn clean_cmd(bins: &Path) -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("FORGE_INVENTORY_SOURCE")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("FORGE_GITHUB_BIN")
        .env_remove("FORGE_GITHUB_TOKEN")
        .env("PATH", bins);
    cmd
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// When a test expects a typed CLI error, run with `--format human` so
/// the error lands on stderr. When the test expects a JSON document,
/// pass `--format json` so the result is on stdout.
fn forge_cmd_human(bins: &Path, db: &Path, args: &[&str]) -> Command {
    let mut cmd = clean_cmd(bins);
    cmd.arg("--format").arg("human").arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

fn forge_cmd_json(bins: &Path, db: &Path, args: &[&str]) -> Command {
    let mut cmd = clean_cmd(bins);
    cmd.arg("--format").arg("json").arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

fn observe_stub(reply: &str) -> String {
    format!(
        "#!/bin/sh\n\
         if [ \"$1\" != \"observe\" ]; then exit 64; fi\n\
         if [ -z \"${{FORGE_GITHUB_TOKEN:-}}\" ]; then exit 65; fi\n\
         # The token must never be echoed to stdout/stderr.\n\
         if printf '%s' \"${{FORGE_GITHUB_TOKEN}}\" | grep -q 'gh[ps]_'; then :; fi\n\
         printf '%s' '{reply}'\n"
    )
}

fn propose_stub(reply: &str) -> String {
    format!(
        "#!/bin/sh\n\
         if [ \"$1\" != \"propose\" ]; then exit 64; fi\n\
         if [ -z \"${{FORGE_GITHUB_TOKEN:-}}\" ]; then exit 65; fi\n\
         printf '%s' '{reply}'\n"
    )
}

#[test]
fn missing_binary_is_typed_unavailable_with_0_bytes_of_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let empty = tmp.path().join("emptybin");
    fs::create_dir_all(&empty).unwrap();
    let db = tmp.path().join("registry.db");
    let out = forge_cmd_human(
        &empty,
        &db,
        &["project", "github", "observe", "octocat/hello-world"],
    )
    .output()
    .expect("run forge");
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    assert_eq!(lossy(&out.stdout), "");
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[github-adapter-unavailable]"),
        "{stderr}"
    );
    assert!(
        stderr.contains("FORGE_GITHUB_BIN") || stderr.contains("github-metadata-adapter"),
        "{stderr}"
    );
}

#[test]
fn missing_token_is_typed_invalid_with_0_bytes_of_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let reply = format!(
        r#"{{"contract":"{CONTRACT}","host":"github.com","repository":"octocat/hello-world","source_revision":"abc","observed_at":"2026-09-29T11:00:00Z","state":"current","topics":[],"languages":[],"workflows":[],"releases":[],"custom_properties":[],"archived":false,"note":""}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &observe_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_human(
        &bins,
        &db,
        &["project", "github", "observe", "octocat/hello-world"],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    );
    let out = cmd.output().expect("run forge");
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    assert_eq!(lossy(&out.stdout), "");
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[github-invalid]"), "{stderr}");
    assert!(stderr.contains("FORGE_GITHUB_TOKEN"), "{stderr}");
}

#[test]
fn a_credential_in_an_adapter_response_is_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let reply = format!(
        r#"{{"contract":"{CONTRACT}","host":"github.com","repository":"octocat/hello-world","source_revision":"abc","observed_at":"2026-09-29T11:00:00Z","state":"current","description":"hello ghp_abcdefghijklmnopqrstuvwxyz0123456789 world","topics":["rust","ci"],"languages":["Rust"],"workflows":["ci.yml"],"releases":["v1.0.0"],"custom_properties":[],"archived":false,"note":""}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &observe_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_json(
        &bins,
        &db,
        &["project", "github", "observe", "octocat/hello-world"],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(!stdout.contains("ghp_abcdef"), "token leaked: {stdout}");
    assert!(stdout.contains("[REDACTED]"), "redaction missing: {stdout}");
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let description = value["observations"][0]["description"].as_str().unwrap();
    assert!(
        !description.contains("ghp_abcdef"),
        "description leaked token: {description}"
    );
}

#[test]
fn rate_limited_state_is_reported_with_reset_and_no_partial_payload() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let reply = format!(
        r#"{{"contract":"{CONTRACT}","host":"github.com","repository":"octocat/hello-world","source_revision":"abc","observed_at":"2026-09-29T11:00:00Z","state":"rate-limited","rate_limit_reset_at":"2026-09-29T12:00:00Z","topics":[],"languages":[],"workflows":[],"releases":[],"custom_properties":[],"archived":false,"note":"please retry"}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &observe_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_json(
        &bins,
        &db,
        &["project", "github", "observe", "octocat/hello-world"],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let observation = &value["observations"][0];
    let state = &observation["state"];
    assert_eq!(state["rate-limited"]["reset_at"], "2026-09-29T12:00:00Z");
    assert_eq!(observation["topics"].as_array().unwrap().len(), 0);
    assert_eq!(observation["languages"].as_array().unwrap().len(), 0);
    let record = &value["records"][0];
    // Rate-limited evidence is `unverified`, never `present`; the
    // catalog never advertises a rate-limited observation as a
    // complete current record.
    assert_eq!(record["evidence"], "unverified");
}

#[test]
fn unauthorized_state_is_unverified_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let reply = format!(
        r#"{{"contract":"{CONTRACT}","host":"github.com","repository":"octocat/hello-world","source_revision":"abc","observed_at":"2026-09-29T11:00:00Z","state":"unauthorized","topics":[],"languages":[],"workflows":[],"releases":[],"custom_properties":[],"archived":false,"note":"missing token"}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &observe_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_json(
        &bins,
        &db,
        &["project", "github", "observe", "octocat/hello-world"],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    // `unauthorized` is a unit variant; it serializes as the literal
    // string. The closed state vocabulary stays stable across runs.
    assert_eq!(
        value["observations"][0]["state"],
        Value::String("unauthorized".to_string())
    );
    assert_eq!(value["records"][0]["evidence"], "unverified");
}

#[test]
fn pull_request_mode_is_the_default_and_does_not_need_a_confirm() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let reply = format!(
        r#"{{"mode":"pull-request","state":"current","artifact_id":"https://github.com/octocat/hello-world/pull/42","note":"opened"}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &propose_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_json(
        &bins,
        &db,
        &[
            "project",
            "github",
            "propose",
            "octocat/hello-world",
            "--set",
            "description=A new description",
        ],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value["outcome"]["mode"], "pull-request");
    assert_eq!(value["outcome"]["state"], "current");
    assert_eq!(
        value["outcome"]["artifact_id"],
        "https://github.com/octocat/hello-world/pull/42"
    );
}

#[test]
fn direct_mode_refuses_without_an_explicit_confirmation() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let reply = format!(
        r#"{{"mode":"direct","state":"current","artifact_id":"deadbeef","note":"applied"}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &propose_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_human(
        &bins,
        &db,
        &[
            "project",
            "github",
            "propose",
            "octocat/hello-world",
            "--mode",
            "direct",
            "--set",
            "description=A new description",
        ],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    assert_eq!(out.status.code(), Some(1), "stderr={}", lossy(&out.stderr));
    assert_eq!(lossy(&out.stdout), "");
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[github-invalid]"), "{stderr}");
    assert!(stderr.contains("confirm"), "{stderr}");
}

#[test]
fn direct_mode_refuses_when_the_adapter_does_not_echo_the_confirmation() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    // The stub echoes a different confirmation, so the call is refused.
    let reply = format!(
        r#"{{"mode":"direct","state":"current","artifact_id":"deadbeef","note":"applied","confirmation":"not-the-token"}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &propose_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_json(
        &bins,
        &db,
        &[
            "project",
            "github",
            "propose",
            "octocat/hello-world",
            "--mode",
            "direct",
            "--confirm",
            "the-token",
            "--set",
            "description=A new description",
        ],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let state = &value["outcome"]["state"];
    assert_eq!(
        state["unavailable"]["reason"]
            .as_str()
            .unwrap()
            .to_ascii_lowercase()
            .contains("confirmation"),
        true,
        "{state}"
    );
    assert!(
        value["outcome"]["note"]
            .as_str()
            .unwrap()
            .contains("confirmation"),
        "{value}"
    );
}

#[test]
fn an_unknown_proposed_field_is_github_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let reply = format!(
        r#"{{"mode":"pull-request","state":"current","artifact_id":"pr/1","note":"opened"}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &propose_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_human(
        &bins,
        &db,
        &[
            "project",
            "github",
            "propose",
            "octocat/hello-world",
            "--set",
            "settings=delete branch protection",
        ],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    assert_eq!(out.status.code(), Some(1), "stderr={}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[github-invalid]"), "{stderr}");
    assert!(stderr.contains("settings"), "{stderr}");
}

#[test]
fn catalog_github_source_reports_unavailable_with_no_binary_and_no_token() {
    let tmp = tempfile::tempdir().unwrap();
    let empty = tmp.path().join("emptybin");
    fs::create_dir_all(&empty).unwrap();
    let db = tmp.path().join("registry.db");
    let mut cmd = forge_cmd_json(&empty, &db, &["project", "list", "--source", "github"]);
    let out = cmd.output().expect("run forge");
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let sources = value["catalog"]["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["source_kind"], "github");
    assert_eq!(sources[0]["state"], "unavailable");
    assert!(
        sources[0]["reason"]
            .as_str()
            .unwrap()
            .contains("FORGE_GITHUB_BIN"),
        "{}",
        sources[0]["reason"]
    );
}

#[test]
fn catalog_github_source_observe_invokes_the_adapter_for_each_requested_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let reply = format!(
        r#"{{"contract":"{CONTRACT}","host":"github.com","repository":"octocat/hello-world","source_revision":"abc","observed_at":"2026-09-29T11:00:00Z","state":"current","topics":["rust","ci"],"languages":["Rust"],"workflows":["ci.yml"],"releases":["v1.0.0"],"custom_properties":[],"archived":false,"note":""}}"#
    );
    write_stub(
        &bins,
        "forge-github-metadata-adapter",
        &observe_stub(&reply),
    );
    let db = tmp.path().join("registry.db");
    // A registry that contains one unrelated local project; the
    // catalog still respects the explicit `--github-repository` list
    // over the local walk.
    let proj = tmp.path().join("localonly");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: localonly\n  name: localonly\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    fs::write(proj.join("Cargo.toml"), "[package]\nname = \"localonly\"\n").unwrap();
    let mut register = clean_cmd(&bins);
    register
        .arg("--registry")
        .arg(&db)
        .env(
            "FORGE_GITHUB_BIN",
            bins.join("forge-github-metadata-adapter"),
        )
        .arg("register")
        .arg(&proj);
    let reg_out = register.output().expect("register");
    assert!(
        reg_out.status.success(),
        "register failed: {}",
        lossy(&reg_out.stderr)
    );
    let mut cmd = forge_cmd_json(
        &bins,
        &db,
        &[
            "project",
            "list",
            "--source",
            "local",
            "--source",
            "github",
            "--github-repository",
            "octocat/hello-world",
        ],
    );
    cmd.env(
        "FORGE_GITHUB_BIN",
        bins.join("forge-github-metadata-adapter"),
    )
    .env("FORGE_GITHUB_TOKEN", "ignored");
    let out = cmd.output().expect("run forge");
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let records = value["catalog"]["records"].as_array().unwrap();
    // The local record for `localonly` plus a GitHub-sourced record
    // keyed by `octocat__hello-world`.
    let ids: Vec<&str> = records
        .iter()
        .map(|record| record["project_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"localonly"), "missing local record: {ids:?}");
    assert!(
        ids.contains(&"octocat__hello-world"),
        "missing github record: {ids:?}"
    );
    let github = records
        .iter()
        .find(|record| record["project_id"] == "octocat__hello-world")
        .unwrap();
    assert_eq!(github["source_kind"], "github");
    assert_eq!(github["evidence"], "present");
    // The catalog's `tags` list stays empty for a GitHub record so
    // the namespace can never merge with topics.
    assert_eq!(github["tags"].as_array().unwrap().len(), 0);
    // Topics live in the dedicated `topics` field of the observation
    // payload, not in the record (the record field set is closed).
    assert_eq!(github["languages"][0], "rust");
    // Source label carries host and repository.
    assert_eq!(github["source"], "github:github.com/octocat/hello-world");
}
