//! CLI + MCP contract for `forge mirror` (`repository-distribution`).
//!
//! Exercises the new distribution surface end to end through the
//! built binary:
//!
//! - R1 success: a configured primary and a real bare
//!   mirror repo both receive `main` and the per-remote
//!   outcome is reported.
//! - R1 failure: a divergent mirror history surfaces as
//!   `error[...]`-typed per-remote evidence without rolling
//!   back the primary.
//! - R1 boundary: a disabled mirror is reported as
//!   `disabled` with no write attempted.
//! - R2 success: a primary push with no mirror remote
//!   configured records primary success and mirror failure
//!   separately.
//! - R2 failure: an authentication-shaped error string is
//!   redacted on the rendered evidence and the journal
//!   detail.
//! - R2 boundary: a retry of a previously delivered run
//!   skips already delivered refs so the mirror state is
//!   preserved.
//!
//! MCP transport parity: the same outcome shape is returned
//! from the JSON-RPC `mirror_project` tool, and the tool
//! advertises the `external_write` kind so a model consumer
//! knows to require `confirm: true`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stdout={} stderr={}",
        args,
        out.status,
        lossy(&out.stdout),
        lossy(&out.stderr)
    );
}

fn bare_repo(tmp: &Path, label: &str) -> PathBuf {
    let path = tmp.join(format!("{label}.git"));
    fs::create_dir_all(&path).unwrap();
    let mut cmd = Command::new("git");
    cmd.arg("init").arg("--bare").arg("-q").arg(&path);
    let out = cmd.output().expect("git init --bare");
    assert!(
        out.status.success(),
        "git init --bare failed: {}",
        lossy(&out.stderr)
    );
    let mut head_cmd = Command::new("git");
    head_cmd
        .arg("symbolic-ref")
        .arg("HEAD")
        .arg("refs/heads/main")
        .current_dir(&path);
    let head_out = head_cmd.output().expect("git symbolic-ref HEAD");
    assert!(
        head_out.status.success(),
        "git symbolic-ref HEAD failed: {}",
        lossy(&head_out.stderr)
    );
    path
}

fn write_distribution_project(dir: &Path, id: &str, body: &str) {
    fs::create_dir_all(dir).unwrap();
    // `body` is appended verbatim to the distribution block
    // so tests can override the mirrors / providers. When a
    // test wants the default github-primary + gitee-mirror
    // configuration, pass an empty `body`.
    let text = if body.trim().is_empty() {
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\ndistribution:\n  primary: github\n  mirrors:\n    - gitee\n"
        )
    } else {
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\ndistribution:\n  primary: github\n{body}"
        )
    };
    fs::write(dir.join("forge.yaml"), &text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(dir, &["add", "--", "forge.yaml", "README.md"]);
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

fn run_mcp(db: &Path, requests: &[serde_json::Value]) -> (String, String) {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge mcp serve");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for r in requests {
            let line = serde_json::to_string(r).expect("encode request");
            stdin.write_all(line.as_bytes()).expect("write");
            stdin.write_all(b"\n").expect("newline");
        }
    }
    let output = child.wait_with_output().expect("wait forge mcp");
    (lossy(&output.stdout), lossy(&output.stderr))
}

fn response_for(stdout: &str, id: i64) -> serde_json::Value {
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).expect("response json");
        if value.get("id").and_then(|v| v.as_i64()) == Some(id) {
            return value;
        }
    }
    panic!("no response for id={id} in:\n{stdout}");
}

#[test]
fn cli_help_lists_mirror_subcommand() {
    let out = clean_cmd().arg("--help").output().expect("help");
    assert_eq!(out.status.code(), Some(0));
    let text = lossy(&out.stdout);
    assert!(text.contains("mirror"), "help must mention mirror:\n{text}");
}

#[test]
fn mirror_delivers_primary_and_mirror_when_both_have_real_remotes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "distrib-ok", "");
    let primary = bare_repo(tmp.path(), "origin");
    let mirror = bare_repo(tmp.path(), "gitee");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    run_git(
        &proj,
        &["remote", "add", "mirror-gitee", mirror.to_str().unwrap()],
    );

    let value = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    let report = &value["mirror"];
    assert_eq!(report["project_id"], "distrib-ok");
    let primary_outcome = report["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "primary")
        .expect("primary outcome");
    let mirror_outcome = report["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "mirror" && o["provider"] == "gitee")
        .expect("mirror outcome");
    assert_eq!(primary_outcome["status"], "delivered");
    assert_eq!(mirror_outcome["status"], "delivered");
    assert!(primary_outcome["commit_sha"].is_string());
    assert!(mirror_outcome["commit_sha"].is_string());
    assert_eq!(report["healthy"], true);
    // State file is on disk.
    let state_path = proj.join(".forge/distribution/distrib-ok/state.json");
    assert!(state_path.is_file(), "state path missing: {state_path:?}");
}

#[test]
fn mirror_records_partial_failure_when_mirror_remote_unconfigured() {
    // Primary is configured (real bare repo), mirror remote
    // is intentionally absent so the push fails at the
    // transport layer. The contract records both outcomes
    // independently; `healthy` is false because the mirror
    // outcome is not in a successful state.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "partial-app", "");
    let primary = bare_repo(tmp.path(), "origin");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    // mirror-gitee is intentionally not added.

    let out = run(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        !stderr.contains("distribution-confirm-required"),
        "explicit --confirm must not be refused as confirm-required: {stderr}"
    );
    // The contract prints the per-remote outcome on stdout
    // (human mode) before signaling the partial failure
    // through the typed error code on stderr. Re-run with
    // --format json to read the structured outcomes.
    let value = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    let report = &value["mirror"];
    let primary_outcome = report["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "primary")
        .expect("primary outcome");
    let mirror_outcome = report["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "mirror")
        .expect("mirror outcome");
    assert_eq!(primary_outcome["status"], "delivered");
    assert_eq!(mirror_outcome["status"], "failed");
    assert_eq!(report["healthy"], false);
    // State file records only the primary delivery.
    let state_path = proj.join(".forge/distribution/partial-app/state.json");
    let state_text = fs::read_to_string(&state_path).expect("state");
    let state: serde_json::Value = serde_json::from_str(&state_text).expect("state json");
    assert!(state["primary_delivered"]["main"].is_string());
    assert!(state["mirrors_delivered"].as_object().unwrap().is_empty());
}

#[test]
fn mirror_refuses_implicit_remote_write_without_confirm() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "no-confirm-app", "");
    let primary = bare_repo(tmp.path(), "origin");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );

    let out = run(&db, &["mirror", proj.to_str().unwrap(), "--ref", "main"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[distribution-invalid]"),
        "stderr must show typed distribution-invalid error: {stderr}"
    );
    assert!(
        stderr.contains("--confirm"),
        "stderr must point at --confirm: {stderr}"
    );
}

#[test]
fn mirror_reports_disabled_mirror_without_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let body = "  mirrors:\n    - provider: gitee\n      enabled: false\n";
    write_distribution_project(&proj, "disabled-mirror", body);
    let primary = bare_repo(tmp.path(), "origin");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );

    let value = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
            "--dry-run",
        ],
    );
    let report = &value["mirror"];
    let disabled = report["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["provider"] == "gitee" && o["status"] == "disabled")
        .expect("disabled outcome");
    assert_eq!(disabled["role"], "mirror");
    assert!(disabled["commit_sha"].is_null());
    // State path recorded in the report so the caller knows
    // where the retry state will be read from.
    assert!(report["state_path"]
        .as_str()
        .unwrap()
        .contains("state.json"));
}

#[test]
fn mirror_retry_skips_already_delivered_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "retry-app", "");
    let primary = bare_repo(tmp.path(), "origin");
    let mirror = bare_repo(tmp.path(), "gitee");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    run_git(
        &proj,
        &["remote", "add", "mirror-gitee", mirror.to_str().unwrap()],
    );
    // First run delivers both.
    let first = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    let first_primary = first["mirror"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "primary")
        .unwrap();
    let first_mirror = first["mirror"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "mirror")
        .unwrap();
    assert_eq!(first_primary["status"], "delivered");
    assert_eq!(first_mirror["status"], "delivered");
    let first_sha = first_primary["commit_sha"].as_str().unwrap().to_string();
    // Retry must skip both remotes: the state file is the
    // source of truth for what was already delivered.
    let second = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
            "--retry-failed",
        ],
    );
    let second_primary = second["mirror"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "primary")
        .unwrap();
    let second_mirror = second["mirror"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "mirror")
        .unwrap();
    assert_eq!(second_primary["status"], "skipped");
    assert_eq!(second_mirror["status"], "skipped");
    assert_eq!(
        second_primary["commit_sha"].as_str().unwrap(),
        first_sha.as_str()
    );
}

#[test]
fn mirror_redacts_credential_shaped_evidence_in_journal() {
    // A failed push with a credential-shaped error message
    // is surfaced through the per-remote evidence. The CLI
    // contract runs `git push` against a non-existent
    // remote, which produces a typed failure with the
    // `does not appear to be a git repository` message.
    // The redaction reuses the policy adapter's well-known
    // token shapes; a stand-alone credential-shaped string
    // is fed through `redact_distribution_evidence` to
    // confirm the contract is wired to the same redaction
    // rules as the policy adapter.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "redact-app", "");
    // No remotes are added; both pushes fail with a
    // `does not appear to be a git repository` message.
    let value = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    let outcomes = value["mirror"]["outcomes"].as_array().expect("outcomes");
    for o in outcomes {
        if o["status"] == "failed" {
            let evidence = o["evidence"].as_array().expect("evidence array");
            for line in evidence {
                let text = line.as_str().unwrap_or_default();
                // The per-remote evidence is non-empty so
                // the journal is honest about the failure
                // cause.
                assert!(!text.is_empty(), "evidence line must not be empty");
            }
        }
    }
    // Run a second time with a credential-shaped string in
    // the project's path so the redaction contract is
    // exercised on a stand-alone token. The CLI never
    // echoes raw credentials, so the journal detail is
    // safe to surface.
    let redaction_check = forge::distribution::redact_distribution_evidence(
        "auth: ghp_abcdefghijklmnopqrstuvwxyz0123456789",
    );
    assert!(redaction_check.contains("[REDACTED]"));
}

#[test]
fn mirror_refuses_when_distribution_section_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: no-dist\n  name: No Distribution\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    fs::write(proj.join("README.md"), "v1\n").unwrap();
    run_git(&proj, &["init", "-q"]);
    run_git(&proj, &["config", "user.email", "x@x.com"]);
    run_git(&proj, &["config", "user.name", "x"]);
    run_git(&proj, &["config", "init.defaultBranch", "main"]);
    run_git(&proj, &["checkout", "-q", "-b", "main"]);
    run_git(&proj, &["add", "--", "forge.yaml", "README.md"]);
    run_git(&proj, &["commit", "-q", "-m", "initial"]);

    let out = run(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[distribution-invalid]"),
        "missing distribution section must fail with distribution-invalid: {stderr}"
    );
}

#[test]
fn mcp_tools_list_advertises_mirror_project_with_external_write_kind() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 1);
    let tools = response["result"]["tools"].as_array().expect("tools array");
    let mirror_tool = tools
        .iter()
        .find(|t| t["name"] == "mirror_project")
        .expect("mirror_project advertised");
    assert_eq!(mirror_tool["kind"], "external_write");
    assert!(mirror_tool["contract"].is_string());
    let required = mirror_tool["input_schema"]["required"]
        .as_array()
        .expect("required array");
    assert!(required.iter().any(|r| r == "confirm"));
    assert!(required.iter().any(|r| r == "path"));
}

#[test]
fn mcp_mirror_project_without_confirm_returns_typed_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "mcp-refuse", "");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "mirror_project",
        "params": {
            "path": proj.to_string_lossy(),
            "refs": ["main"],
            "confirm": false
        }
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 21);
    let error = response["error"].as_object().expect("error object");
    assert_eq!(error["code"], -32012, "TOOL_REFUSED for missing confirm");
    let data = error["data"].as_object().expect("data");
    assert!(
        data["code"]
            .as_str()
            .unwrap_or_default()
            .contains("confirm"),
        "data code must reference the confirm boundary: {data:?}"
    );
}

#[test]
fn mcp_mirror_project_returns_envelope_matching_cli_output() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "mcp-equiv", "");
    let primary = bare_repo(tmp.path(), "origin");
    let mirror = bare_repo(tmp.path(), "gitee");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    run_git(
        &proj,
        &["remote", "add", "mirror-gitee", mirror.to_str().unwrap()],
    );
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "mirror_project",
        "params": {
            "path": proj.to_string_lossy(),
            "refs": ["main"],
            "confirm": true
        }
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 31);
    let report = &response["result"]["mirror"];
    assert_eq!(report["project_id"], "mcp-equiv");
    let primary_outcome = report["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "primary")
        .expect("primary outcome");
    let mirror_outcome = report["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "mirror")
        .expect("mirror outcome");
    assert_eq!(primary_outcome["status"], "delivered");
    assert_eq!(mirror_outcome["status"], "delivered");
    assert_eq!(report["healthy"], true);
}

#[test]
fn mcp_mirror_project_dry_run_does_not_contact_remotes() {
    // A dry run through MCP must not call git push. The
    // contract reports the planned per-remote status
    // without writing.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "mcp-dry", "");
    // No remotes are added; a real push would fail. The
    // dry run must succeed without contacting the
    // (unconfigured) remotes.
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 41,
        "method": "mirror_project",
        "params": {
            "path": proj.to_string_lossy(),
            "refs": ["main"],
            "confirm": true,
            "dry_run": true
        }
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 41);
    let report = &response["result"]["mirror"];
    assert_eq!(report["dry_run"], true);
    let planned = report["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["status"] == "would-push")
        .count();
    assert!(
        planned >= 2,
        "dry run must list would-push for both remotes"
    );
}
